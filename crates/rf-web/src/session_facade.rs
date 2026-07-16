//! Laravel-style Session facade for RustForge.
//!
//! The facade exposes a static, no-`.await` API (`Session::get/put/has/forget/
//! flush/flash`) but is scoped to the **current request's session**, keyed by a
//! session id carried in a cookie. Each visitor gets their OWN server-side session
//! map, so a value or flash set by one client can never leak to another.
//!
//! Isolation works exactly like the per-request auth scope
//! (`rf_auth::with_auth_scope`) and request context (`rf_request::capture_request`):
//! the [`session_scope`] middleware mints/loads the caller's session id, sets a
//! task-local for the duration of the request, and the facade reads/writes the
//! backing store entry for that id.
//!
//! **The [`session_scope`] middleware MUST be wired into your router.** Calling
//! any `SessionFacade` method without an active scope panics with a clear
//! diagnostic message. There is no silent process-global fallback: in a concurrent
//! web server, a single shared session across all callers is a security bug
//! (session data from one client would bleed into every other client).
//!
//! ## Session lifetime and GC
//!
//! Sessions have an **idle lifetime**: any session that has not been touched for
//! longer than the configured lifetime is treated as absent and eligible for
//! removal. The default is **24 hours** of idle time; override it at runtime with
//! the `SESSION_LIFETIME_SECS` environment variable.
//!
//! Eviction happens in two ways:
//!
//! * **Opportunistic** — on every `with_session` access the entry for the current
//!   request's session id is checked; if expired it is evicted right there before
//!   a fresh entry is created.
//! * **Background sweep** — [`session_scope`] lazily spawns a tokio task (once per
//!   process, only when a tokio runtime is present) that calls [`sweep_sessions`]
//!   every 15 minutes, evicting all expired entries in one pass.  When no runtime
//!   is present (CLI, unit tests) the sweep does not run; opportunistic eviction
//!   still protects correctness.
//!
//! ## Flash: true one-request lifetime
//!
//! A value written with [`SessionFacade::flash`] is readable on the **next**
//! request for that same client (via `get`/`has`), then automatically cleared on
//! the one after — the classic Laravel new/old flash aging, performed by
//! [`session_scope`] at the start of every request.
//!
//! ## Session regeneration (fixation defense)
//!
//! Call [`SessionFacade::regenerate`] after a successful login.  It mints a new
//! session id and migrates the current session data to it, invalidating the old
//! id.  This prevents session-fixation attacks: an attacker who planted their own
//! id before the victim's login can no longer use that id once regeneration has
//! happened.
//!
//! [`session_scope`] also guards the other direction: if the client sends a session
//! id that is not present in the in-memory store (i.e. an unknown, forged, or
//! **expired** id), a fresh id is generated rather than echoing the attacker-supplied
//! value.
//!
//! # Examples
//!
//! ```rust,ignore
//! // SessionFacade methods MUST be called inside a session_scope — the snippet
//! // below shows the API shape; see the integration tests for a runnable example
//! // that wires the middleware.
//! use rf_web::SessionFacade;
//! use serde_json::json;
//!
//! // (inside a handler covered by session_scope middleware)
//! SessionFacade::put("user_id", json!(123));
//! if let Some(user_id) = SessionFacade::get("user_id") {
//!     println!("User ID: {}", user_id);
//! }
//! ```

use axum::{
    extract::Request,
    http::header,
    middleware::Next,
    response::Response,
};
use once_cell::sync::Lazy;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Once, RwLock,
};
use std::time::{Duration, Instant};

/// Cookie name carrying the per-client session id.
const SESSION_COOKIE: &str = "rf_session";

/// Default session idle lifetime: 24 hours.
const DEFAULT_SESSION_LIFETIME_SECS: u64 = 86_400;

/// How often the background sweeper wakes up to evict expired sessions.
const GC_INTERVAL: Duration = Duration::from_secs(15 * 60); // 15 minutes

/// Overridable session lifetime in nanoseconds.  `0` means "use the env var /
/// hard-coded default".  Tests override this via [`set_test_session_lifetime`]
/// so they do not need to sleep for 24 hours.
static SESSION_LIFETIME_NANOS: AtomicU64 = AtomicU64::new(0);

/// Return the currently configured session idle lifetime.
///
/// Resolution order:
/// 1. In-process override (tests only, via [`set_test_session_lifetime`]).
/// 2. `SESSION_LIFETIME_SECS` environment variable.
/// 3. Hard-coded 24-hour default.
fn session_lifetime() -> Duration {
    let nanos = SESSION_LIFETIME_NANOS.load(Ordering::Relaxed);
    if nanos > 0 {
        return Duration::from_nanos(nanos);
    }
    let secs = std::env::var("SESSION_LIFETIME_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_SESSION_LIFETIME_SECS);
    Duration::from_secs(secs)
}

/// Data for a single client's session: the key/value map plus the flash bookkeeping
/// that gives flashed keys a one-request lifetime, and an idle-expiry timestamp.
struct SessionData {
    /// Regular + flash values. Flash values live here too so `get`/`has` see them
    /// transparently for the one request they are alive.
    data: HashMap<String, Value>,
    /// Keys flashed during the CURRENT request (become `old` on the next request).
    flash_new: HashSet<String>,
    /// Keys flashed during the PREVIOUS request (removed on the next aging).
    flash_old: HashSet<String>,
    /// Monotonic timestamp refreshed on every [`with_session`] access.  Used to
    /// implement idle-expiry: a session is expired when
    /// `last_activity.elapsed() > session_lifetime()`.
    last_activity: Instant,
}

impl Default for SessionData {
    fn default() -> Self {
        Self {
            data: HashMap::new(),
            flash_new: HashSet::new(),
            flash_old: HashSet::new(),
            last_activity: Instant::now(),
        }
    }
}

impl SessionData {
    /// Returns `true` when this session has been idle longer than the configured
    /// lifetime and should be treated as absent.
    fn is_expired(&self) -> bool {
        self.last_activity.elapsed() > session_lifetime()
    }

    /// Advance the flash lifecycle by one request (called at request start):
    /// drop values that were flashed two requests ago, and promote this-request
    /// flashes to `old` so they survive exactly one further request.
    fn age_flash(&mut self) {
        let expiring = std::mem::take(&mut self.flash_old);
        for key in &expiring {
            // Only remove if it is still a flash value and was not overwritten by a
            // durable `put` under the same key in the meantime.
            if !self.flash_new.contains(key) {
                self.data.remove(key);
            }
        }
        self.flash_old = std::mem::take(&mut self.flash_new);
    }
}

/// Process-wide backing store: one [`SessionData`] per session id. Sessions living
/// server-side keyed by id is correct and intentional; the isolation comes from
/// each request only ever touching its OWN id (via the task-local below).
///
/// Entries are bounded by `session_lifetime()` and periodically evicted by the
/// background sweeper started in [`ensure_session_gc_started`].
static SESSIONS: Lazy<RwLock<HashMap<String, SessionData>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

tokio::task_local! {
    /// The current request's session id, wrapped in a `RefCell` so that
    /// [`SessionFacade::regenerate`] can swap in a new id during the same request.
    static CURRENT_SESSION_ID: RefCell<String>;
}

// ---------------------------------------------------------------------------
// Background GC sweeper
// ---------------------------------------------------------------------------

/// Evict all sessions whose idle time exceeds the configured lifetime.
///
/// Called by the background tokio task every [`GC_INTERVAL`].  Also callable
/// directly from tests (no tokio runtime required) to trigger a sweep synchronously.
pub fn sweep_sessions() {
    let lifetime = session_lifetime();
    let mut store = SESSIONS.write().unwrap();
    store.retain(|_, v| !v.is_expired() || v.last_activity.elapsed() <= lifetime);
}

/// Ensure the background GC task is spawned exactly once per process.
///
/// Uses `std::sync::Once` so repeated calls (one per HTTP request) are
/// extremely cheap (a single atomic load after the first call).  If called
/// outside a tokio runtime (CLI, unit tests) the `try_current()` check fails
/// and no task is spawned — correctness is preserved by opportunistic
/// eviction inside `with_session`.
fn ensure_session_gc_started() {
    static GC_INIT: Once = Once::new();
    GC_INIT.call_once(|| {
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async {
                loop {
                    tokio::time::sleep(GC_INTERVAL).await;
                    sweep_sessions();
                }
            });
        }
    });
}

// ---------------------------------------------------------------------------
// Core access helper
// ---------------------------------------------------------------------------

/// Run `f` against the current request's per-client session.
///
/// On every call:
/// * If the session exists but has expired (idle > lifetime), it is evicted
///   **opportunistically** right here and a fresh entry is created — the caller
///   sees an empty session, not stale data.
/// * `last_activity` is refreshed to now (idle-expiry semantics).
///
/// # Panics
///
/// Panics if called outside a [`session_scope`] task-local scope.  This is an
/// explicit fail-fast: in a concurrent web server a process-global session shared
/// by all callers would be a security bug (data from one client bleeds into every
/// other client).  Wire [`session_scope`] as a router layer to silence this panic.
fn with_session<R>(f: impl FnOnce(&mut SessionData) -> R) -> R {
    CURRENT_SESSION_ID.try_with(|cell| {
        let sid = cell.borrow().clone();
        let mut store = SESSIONS.write().unwrap();

        // Opportunistic expiry: evict this entry now if it has idled out.
        // The or_default() below then creates a fresh empty entry.
        if store.get(&sid).map(|d| d.is_expired()).unwrap_or(false) {
            store.remove(&sid);
        }

        let entry = store.entry(sid).or_default();
        // Refresh last-activity timestamp on every access (idle-expiry semantics).
        entry.last_activity = Instant::now();
        f(entry)
    })
    .unwrap_or_else(|_| {
        panic!(
            "SessionFacade used without the session_scope middleware — \
             add session_scope() to your router. \
             A process-global shared session does not exist: it would silently \
             bleed one client's data into every other concurrent client."
        )
    })
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Generate a cryptographically secure session id (256 bits, URL-safe base64).
fn generate_session_id() -> String {
    use base64::Engine;
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Extract the `rf_session` id from the request `Cookie` header, if present.
fn cookie_session_id(req: &Request) -> Option<String> {
    req.headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookies| {
            cookies.split(';').find_map(|c| {
                let mut parts = c.trim().splitn(2, '=');
                let name = parts.next()?.trim();
                let value = parts.next()?.trim();
                (name == SESSION_COOKIE).then(|| value.to_string())
            })
        })
}

/// Returns `true` when called from within a task that is running inside a
/// [`session_scope`] — i.e., from a real HTTP request handler.  Returns `false`
/// in unit tests, CLI code, or any context where no scope has been established.
///
/// Downstream crates (e.g. `rf-views`) use this to decide whether to delegate
/// flash reads/writes to the per-client [`SessionFacade`] or to a local
/// fallback, without hard-coding knowledge of the task-local internals.
pub fn in_session_scope() -> bool {
    CURRENT_SESSION_ID.try_with(|_| ()).is_ok()
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

/// Per-request middleware that establishes the current client's session scope.
///
/// On each request it:
/// 1. reads the `rf_session` cookie, or mints a fresh session id if absent, if
///    the supplied id is not present in the session store, or if the session has
///    **expired** (session-fixation defense + idle-expiry enforcement);
/// 2. ages that session's flash data (one-request lifetime);
/// 3. runs the handler inside a task-local scope so the [`SessionFacade`] reads
///    and writes ONLY this client's session; and
/// 4. sets/refreshes the `rf_session` cookie on the response, using the **final**
///    session id (which may have changed if the handler called
///    [`SessionFacade::regenerate`]).
///
/// Also lazily starts the background GC sweeper (once per process) that evicts
/// expired sessions every 15 minutes.
///
/// ```ignore
/// use axum::{Router, routing::get, middleware};
/// use rf_web::session_scope;
/// let app = Router::new().route("/", get(handler))
///     .layer(middleware::from_fn(session_scope));
/// ```
pub async fn session_scope(req: Request, next: Next) -> Response {
    // Lazily start the GC sweeper on the first real request (needs a tokio runtime).
    ensure_session_gc_started();

    let candidate_id = cookie_session_id(&req);

    // Session fixation + expiry defense: only reuse a supplied session id when it
    // actually exists in the store AND has not expired.  An attacker can plant an
    // id before the victim visits; if we echo that id back as authenticated, they
    // know the victim's session.  An expired session is similarly rejected.
    let session_id = match candidate_id {
        Some(id) => {
            let valid = SESSIONS
                .read()
                .unwrap()
                .get(&id)
                .map(|d| !d.is_expired())
                .unwrap_or(false);
            if valid { id } else { generate_session_id() }
        }
        None => generate_session_id(),
    };

    // Age flash at the start of the request so values flashed on the previous
    // request are readable now, then gone next time.
    {
        let mut store = SESSIONS.write().unwrap();
        store.entry(session_id.clone()).or_default().age_flash();
    }

    // Run the handler inside a task-local scope.  The RefCell lets the handler
    // call SessionFacade::regenerate() to swap the session id mid-request; we
    // capture the final id (which may differ from session_id) before the scope
    // ends so we can set the correct cookie.
    let (final_id, mut response) = CURRENT_SESSION_ID
        .scope(
            RefCell::new(session_id),
            async move {
                let response = next.run(req).await;
                // Still inside the scope — read back the (possibly regenerated) id.
                let final_id = CURRENT_SESSION_ID.with(|cell| cell.borrow().clone());
                (final_id, response)
            },
        )
        .await;

    // Add the Secure flag when:
    //   • APP_ENV=production (fail-safe: production traffic should always be HTTPS), or
    //   • SESSION_SECURE=true is explicitly set.
    // In development and test environments the flag is omitted so localhost HTTPS
    // is not required.
    let secure_attr = {
        let env = std::env::var("APP_ENV").unwrap_or_default();
        let forced = std::env::var("SESSION_SECURE").as_deref() == Ok("true");
        if env == "production" || forced { "; Secure" } else { "" }
    };
    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax{}",
        SESSION_COOKIE, final_id, secure_attr
    );
    if let Ok(value) = cookie.parse() {
        response.headers_mut().append(header::SET_COOKIE, value);
    }

    response
}

// ---------------------------------------------------------------------------
// Facade
// ---------------------------------------------------------------------------

/// The SessionFacade providing a static API scoped to the current request's session.
///
/// Wire [`session_scope`] into your router so each client gets its own isolated
/// session.  **Calling any method without an active [`session_scope`] is a
/// programming error and panics immediately** — there is no silent process-global
/// fallback.  In a concurrent web server, a shared process-global session would
/// silently bleed one client's data into every other concurrent client, which is
/// a security bug.
///
/// Sessions expire after a configurable idle lifetime (default 24 hours).  An
/// expired session is treated as absent: `get` returns `None` and the next write
/// creates a fresh entry.
///
/// # DX-layer convenience — task-local requirement
///
/// `SessionFacade` is part of RustForge's **optional** Laravel-style DX layer. Every
/// method reads/writes the per-client session identified by the task-local set by the
/// [`session_scope`] middleware.
///
/// **`session_scope` is required.** Any call to `get`, `put`, `has`, `forget`,
/// `flush`, or `flash` outside a [`session_scope`]-wrapped handler panics with a
/// diagnostic message telling you exactly what to fix.
///
/// To check whether a scope is active before calling the facade (e.g. in shared
/// helper code that may run both inside and outside a request), use
/// [`in_session_scope()`].
///
/// # Examples
///
/// ```rust,ignore
/// // These calls must be inside a handler covered by session_scope middleware.
/// use rf_web::SessionFacade;
/// use serde_json::json;
///
/// SessionFacade::put("user_id", json!(123));
/// if let Some(user_id) = SessionFacade::get("user_id") {
///     println!("User ID: {}", user_id);
/// }
/// ```
pub struct SessionFacade;

impl SessionFacade {
    /// Get a value from the current client's session (includes a flash value that
    /// is still alive for this request).
    pub fn get(key: &str) -> Option<Value> {
        with_session(|s| s.data.get(key).cloned())
    }

    /// Put a durable value into the current client's session.
    pub fn put(key: impl Into<String>, value: Value) {
        let key = key.into();
        with_session(|s| {
            // A durable put cancels any pending flash expiry for this key.
            s.flash_new.remove(&key);
            s.flash_old.remove(&key);
            s.data.insert(key, value);
        });
    }

    /// True if the current client's session has a value (durable or live flash).
    pub fn has(key: &str) -> bool {
        with_session(|s| s.data.contains_key(key))
    }

    /// Remove a value from the current client's session.
    pub fn forget(key: &str) {
        with_session(|s| {
            s.data.remove(key);
            s.flash_new.remove(key);
            s.flash_old.remove(key);
        });
    }

    /// Clear the current client's session entirely.
    pub fn flush() {
        with_session(|s| {
            s.data.clear();
            s.flash_new.clear();
            s.flash_old.clear();
        });
    }

    /// Flash a value for exactly one request: it is readable via [`get`](Self::get)
    /// / [`has`](Self::has) on this client's NEXT request, then automatically
    /// cleared on the one after (aged out by [`session_scope`]).
    pub fn flash(key: impl Into<String>, value: Value) {
        let key = key.into();
        with_session(|s| {
            s.data.insert(key.clone(), value);
            s.flash_new.insert(key);
        });
    }

    /// Regenerate the session id, migrating all session data to a fresh id.
    ///
    /// Call this immediately after a successful login to defend against session
    /// fixation: any id the attacker may have planted (e.g. via a shared link or
    /// XSS) becomes invalid the moment the user authenticates, because the
    /// authenticated session now lives under a brand-new id that the attacker does
    /// not know.
    ///
    /// - All current session data (durable values and flash bookkeeping) is
    ///   preserved under the new id.
    /// - The old id is removed from the store so subsequent requests bearing it
    ///   are treated as unknown and get a fresh session (no fixation, no replay).
    /// - [`session_scope`] automatically picks up the new id for the `Set-Cookie`
    ///   header at the end of the request.
    /// - Outside a session scope (CLI / unit tests) this is a no-op.
    pub fn regenerate() {
        let _ = CURRENT_SESSION_ID.try_with(|cell| {
            let old_id = cell.borrow().clone();
            let new_id = generate_session_id();

            // Migrate data from old to new id in the global store.
            {
                let mut store = SESSIONS.write().unwrap();
                if let Some(data) = store.remove(&old_id) {
                    store.insert(new_id.clone(), data);
                } else {
                    store.entry(new_id.clone()).or_default();
                }
            }

            // Update the task-local so subsequent with_session() calls use the new
            // id, and so session_scope picks up the new id for Set-Cookie.
            *cell.borrow_mut() = new_id;
        });
    }

    /// Return the current session id, if running inside a [`session_scope`].
    ///
    /// Returns `None` outside a request scope (CLI, unit tests without an explicit
    /// scope).  Primarily useful for testing and debugging.
    pub fn id() -> Option<String> {
        CURRENT_SESSION_ID.try_with(|cell| cell.borrow().clone()).ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // -----------------------------------------------------------------------
    // Test helpers
    // -----------------------------------------------------------------------

    /// Run a closure inside a fresh, uniquely-keyed session scope so tests do not
    /// clobber each other (each test id is distinct to avoid cross-test interference).
    fn in_session<R>(id: &str, f: impl FnOnce() -> R) -> R {
        CURRENT_SESSION_ID.sync_scope(RefCell::new(id.to_string()), f)
    }

    /// Override the session idle lifetime for a test, returning the previous value.
    /// Callers must restore via [`restore_session_lifetime`] when done.
    fn set_test_session_lifetime(d: Duration) -> u64 {
        let nanos = d.as_nanos() as u64;
        SESSION_LIFETIME_NANOS.swap(nanos, Ordering::SeqCst)
    }

    /// Restore the session lifetime previously returned by [`set_test_session_lifetime`].
    fn restore_session_lifetime(prev: u64) {
        SESSION_LIFETIME_NANOS.store(prev, Ordering::SeqCst);
    }

    /// Artificially age a session entry so that it appears to have been idle for
    /// `age`.  Used in tests to simulate expiry without sleeping.
    ///
    /// Uses `Instant::checked_sub` — callers must pass an `age` that is shorter
    /// than the system uptime (any value ≤ a few hours is always safe).
    fn age_session_for_test(id: &str, age: Duration) {
        let mut store = SESSIONS.write().unwrap();
        if let Some(data) = store.get_mut(id) {
            if let Some(past) = Instant::now().checked_sub(age) {
                data.last_activity = past;
            }
        }
    }

    // -----------------------------------------------------------------------
    // Existing session-isolation and flash tests (unchanged semantics)
    // -----------------------------------------------------------------------

    #[test]
    fn test_session_put_and_get() {
        in_session("t_put_get", || {
            SessionFacade::put("test_key", json!("test_value"));
            assert_eq!(SessionFacade::get("test_key"), Some(json!("test_value")));
        });
    }

    #[test]
    fn test_session_has() {
        in_session("t_has", || {
            SessionFacade::put("exists", json!(true));
            assert!(SessionFacade::has("exists"));
        });
    }

    #[test]
    fn test_session_forget() {
        in_session("t_forget", || {
            SessionFacade::put("to_forget", json!("value"));
            SessionFacade::forget("to_forget");
            assert!(!SessionFacade::has("to_forget"));
        });
    }

    #[test]
    fn test_session_flush() {
        in_session("t_flush", || {
            SessionFacade::put("key1", json!("value1"));
            SessionFacade::put("key2", json!("value2"));
            SessionFacade::flush();
            assert!(!SessionFacade::has("key1"));
            assert!(!SessionFacade::has("key2"));
        });
    }

    #[test]
    fn test_session_flash() {
        in_session("t_flash", || {
            SessionFacade::flash("flash_key", json!("flash_value"));
            assert!(SessionFacade::has("flash_key"));
        });
    }

    #[test]
    fn test_sessions_are_isolated_per_id() {
        in_session("client_a", || SessionFacade::put("secret", json!("A")));
        // A different session id sees NOTHING from client A.
        in_session("client_b", || {
            assert!(!SessionFacade::has("secret"), "client B must not see A's data");
            SessionFacade::put("secret", json!("B"));
            assert_eq!(SessionFacade::get("secret"), Some(json!("B")));
        });
        // Client A still sees only its own value.
        in_session("client_a", || {
            assert_eq!(SessionFacade::get("secret"), Some(json!("A")));
        });
    }

    #[test]
    fn test_flash_lives_exactly_one_request() {
        // Request 1: flash a value.
        in_session("flash_life", || {
            SessionFacade::flash("msg", json!("hi"));
        });
        // Request 2: age_flash runs at scope entry via `age_flash`; simulate the
        // middleware aging then read — value must be present.
        {
            let mut store = SESSIONS.write().unwrap();
            store.entry("flash_life".to_string()).or_default().age_flash();
        }
        in_session("flash_life", || {
            assert_eq!(
                SessionFacade::get("msg"),
                Some(json!("hi")),
                "flash must survive to the next request"
            );
        });
        // Request 3: age again -> gone.
        {
            let mut store = SESSIONS.write().unwrap();
            store.entry("flash_life".to_string()).or_default().age_flash();
        }
        in_session("flash_life", || {
            assert!(
                !SessionFacade::has("msg"),
                "flash must be cleared after one request"
            );
        });
    }

    #[test]
    fn test_in_session_scope_detects_scope() {
        assert!(!in_session_scope(), "no scope outside in_session helper");
        in_session("scope_detect", || {
            assert!(in_session_scope(), "in_session_scope() must return true inside scope");
        });
    }

    // -----------------------------------------------------------------------
    // regenerate() tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_regenerate_changes_session_id() {
        in_session("regen_id", || {
            let old_id = SessionFacade::id().expect("must be in scope");
            SessionFacade::regenerate();
            let new_id = SessionFacade::id().expect("must be in scope after regenerate");
            assert_ne!(old_id, new_id, "regenerate must change the session id");
        });
    }

    #[test]
    fn test_regenerate_preserves_session_data() {
        in_session("regen_data", || {
            SessionFacade::put("role", json!("admin"));
            SessionFacade::put("user_id", json!(42));
            SessionFacade::regenerate();
            assert_eq!(
                SessionFacade::get("role"),
                Some(json!("admin")),
                "durable value must survive regeneration"
            );
            assert_eq!(
                SessionFacade::get("user_id"),
                Some(json!(42)),
                "durable value must survive regeneration"
            );
        });
    }

    #[test]
    fn test_regenerate_old_id_removed_from_store() {
        in_session("regen_old_gone", || {
            SessionFacade::put("secret", json!("data"));
            let old_id = SessionFacade::id().expect("must be in scope");

            SessionFacade::regenerate();
            let new_id = SessionFacade::id().expect("must be in scope");

            // Old id must be absent from the store (prevents replay / fixation).
            let store = SESSIONS.read().unwrap();
            assert!(
                !store.contains_key(&old_id),
                "old session id must be removed from the store after regeneration"
            );
            // New id must carry the data.
            assert!(
                store
                    .get(&new_id)
                    .map(|s| s.data.contains_key("secret"))
                    .unwrap_or(false),
                "new session id must hold the migrated data"
            );
        });
    }

    #[test]
    fn test_regenerate_outside_scope_is_noop() {
        // regenerate() uses try_with() directly (no with_session call), so it
        // remains a deliberate no-op outside scope — there is nothing to migrate.
        SessionFacade::regenerate();
    }

    // -----------------------------------------------------------------------
    // Fail-fast tests: prove the process-global fallback is GONE.
    //
    // Each of these deliberately calls a SessionFacade method WITHOUT establishing
    // a session_scope.  The correct behavior is an immediate panic with a clear
    // diagnostic.  A silent return — or, worse, sharing a global session — is the
    // security bug the external review flagged: in a concurrent web server, a
    // global session shared by all callers bleeds one client's data into every
    // other client.
    // -----------------------------------------------------------------------

    /// Calling `SessionFacade::get` without a scope must panic, not return None
    /// from a process-global shared session.
    #[test]
    #[should_panic(expected = "SessionFacade used without the session_scope middleware")]
    fn test_get_without_scope_panics() {
        // No in_session() wrapper — deliberately outside any scope.
        let _ = SessionFacade::get("any_key");
    }

    /// Calling `SessionFacade::put` without a scope must panic.
    #[test]
    #[should_panic(expected = "SessionFacade used without the session_scope middleware")]
    fn test_put_without_scope_panics() {
        SessionFacade::put("key", json!("value"));
    }

    /// Calling `SessionFacade::has` without a scope must panic.
    #[test]
    #[should_panic(expected = "SessionFacade used without the session_scope middleware")]
    fn test_has_without_scope_panics() {
        let _ = SessionFacade::has("key");
    }

    /// Calling `SessionFacade::forget` without a scope must panic.
    #[test]
    #[should_panic(expected = "SessionFacade used without the session_scope middleware")]
    fn test_forget_without_scope_panics() {
        SessionFacade::forget("key");
    }

    /// Calling `SessionFacade::flush` without a scope must panic.
    #[test]
    #[should_panic(expected = "SessionFacade used without the session_scope middleware")]
    fn test_flush_without_scope_panics() {
        SessionFacade::flush();
    }

    /// Calling `SessionFacade::flash` without a scope must panic.
    #[test]
    #[should_panic(expected = "SessionFacade used without the session_scope middleware")]
    fn test_flash_without_scope_panics() {
        SessionFacade::flash("key", json!("value"));
    }

    /// Prove that two simulated concurrent clients (distinct scope ids) are fully
    /// isolated: value written under id A is never visible under id B, and vice versa.
    /// This is the per-client isolation guarantee the fail-fast change must not break.
    #[test]
    fn test_concurrent_clients_are_isolated() {
        // Client A writes a secret.
        in_session("iso_client_a", || {
            SessionFacade::put("secret", json!("only_for_A"));
        });

        // Client B has no session at all — it must see nothing from A.
        in_session("iso_client_b", || {
            assert!(
                !SessionFacade::has("secret"),
                "client B must not see client A's session data"
            );
            SessionFacade::put("secret", json!("only_for_B"));
        });

        // After B writes its own value, A's session is still intact.
        in_session("iso_client_a", || {
            assert_eq!(
                SessionFacade::get("secret"),
                Some(json!("only_for_A")),
                "client A's data must not be overwritten by client B"
            );
        });

        // And B's session is also intact.
        in_session("iso_client_b", || {
            assert_eq!(
                SessionFacade::get("secret"),
                Some(json!("only_for_B")),
                "client B must see only its own data"
            );
        });
    }

    // -----------------------------------------------------------------------
    // TTL / GC tests (Cycle-17 Item 2)
    // -----------------------------------------------------------------------

    /// (a) A session whose last_activity is artificially aged past the configured
    /// lifetime is treated as absent (get returns None; a fresh empty session is
    /// silently created on access), proving opportunistic eviction in with_session.
    #[test]
    fn test_expired_session_treated_as_absent() {
        let id = "ttl_expired_session";

        // Set a short lifetime (100 ms) so we can age the session well past it.
        let prev = set_test_session_lifetime(Duration::from_millis(100));

        // Insert a session entry with a known value.
        in_session(id, || {
            SessionFacade::put("key", json!("should_disappear"));
        });
        assert!(
            SESSIONS.read().unwrap().contains_key(id),
            "session must be in the store after insert"
        );

        // Age the entry by 200 ms (well past the 100 ms lifetime).
        age_session_for_test(id, Duration::from_millis(200));

        // Now access the session: with_session must detect the expired entry,
        // evict it, and create a fresh one — so the old value is gone.
        in_session(id, || {
            assert!(
                !SessionFacade::has("key"),
                "expired session must be treated as absent — old value must not leak"
            );
        });

        restore_session_lifetime(prev);
    }

    /// (b) sweep_sessions() removes expired entries and shrinks the map.
    ///
    /// Insert N sessions; age M of them past the lifetime; call sweep_sessions();
    /// verify the map has shrunk by exactly M.
    #[test]
    fn test_gc_sweep_removes_expired_entries() {
        let prefix = "sweep_test_";

        // Use a 100 ms lifetime for this test.
        let prev = set_test_session_lifetime(Duration::from_millis(100));

        let total = 6usize;
        let to_expire = 4usize;

        // Insert `total` distinct sessions.
        for i in 0..total {
            let id = format!("{}{}", prefix, i);
            in_session(&id, || {
                SessionFacade::put("v", json!(i));
            });
        }

        // Record store size before eviction (may include other test sessions, so
        // count only the ones we just inserted).
        {
            let store = SESSIONS.read().unwrap();
            let our_count = (0..total).filter(|i| store.contains_key(&format!("{}{}", prefix, i))).count();
            assert_eq!(our_count, total, "all {total} sessions must be present before sweep");
        }

        // Age the first `to_expire` sessions past the lifetime.
        for i in 0..to_expire {
            let id = format!("{}{}", prefix, i);
            age_session_for_test(&id, Duration::from_millis(200));
        }

        // Run the sweeper directly (no tokio runtime required in this sync test).
        sweep_sessions();

        // The expired sessions must be gone; the live ones must remain.
        {
            let store = SESSIONS.read().unwrap();
            for i in 0..to_expire {
                let id = format!("{}{}", prefix, i);
                assert!(
                    !store.contains_key(&id),
                    "session {id} was expired and must have been removed by sweep"
                );
            }
            for i in to_expire..total {
                let id = format!("{}{}", prefix, i);
                assert!(
                    store.contains_key(&id),
                    "session {id} was NOT expired and must still be present after sweep"
                );
            }
        }

        restore_session_lifetime(prev);
    }

    /// (c) A live (non-expired) session is unaffected by sweep and remains fully
    /// accessible with all its data intact, and remains isolated from other clients.
    #[test]
    fn test_live_session_survives_sweep_and_stays_isolated() {
        let live_id   = "live_session_survives";
        let dead_id   = "dead_session_evicted";
        let other_id  = "other_live_session";

        let prev = set_test_session_lifetime(Duration::from_millis(100));

        // Insert three sessions.
        in_session(live_id, || {
            SessionFacade::put("role", json!("admin"));
            SessionFacade::put("user", json!(7));
        });
        in_session(dead_id, || {
            SessionFacade::put("secret", json!("gone"));
        });
        in_session(other_id, || {
            SessionFacade::put("data", json!("also_survives"));
        });

        // Age only the dead session.
        age_session_for_test(dead_id, Duration::from_millis(200));

        // Sweep: dead_id should be evicted, others remain.
        sweep_sessions();

        // The dead session is gone.
        assert!(
            !SESSIONS.read().unwrap().contains_key(dead_id),
            "expired session must be removed by sweep"
        );

        // The live session still has all its data.
        in_session(live_id, || {
            assert_eq!(
                SessionFacade::get("role"),
                Some(json!("admin")),
                "live session: 'role' must survive sweep"
            );
            assert_eq!(
                SessionFacade::get("user"),
                Some(json!(7)),
                "live session: 'user' must survive sweep"
            );
        });

        // The other live session is also intact and isolated from live_id.
        in_session(other_id, || {
            assert_eq!(
                SessionFacade::get("data"),
                Some(json!("also_survives")),
                "other live session must survive sweep"
            );
            assert!(
                !SessionFacade::has("role"),
                "other session must not see live_id's 'role'"
            );
        });

        restore_session_lifetime(prev);
    }
}
