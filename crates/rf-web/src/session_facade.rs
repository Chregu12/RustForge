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
//! backing store entry for that id. Outside any request scope (unit tests, CLI)
//! the facade falls back to a single process-local session — that fallback is
//! never used while serving concurrent HTTP requests, since each of those runs
//! inside its own [`session_scope`].
//!
//! ## Flash: true one-request lifetime
//!
//! A value written with [`SessionFacade::flash`] is readable on the **next**
//! request for that same client (via `get`/`has`), then automatically cleared on
//! the one after — the classic Laravel new/old flash aging, performed by
//! [`session_scope`] at the start of every request.
//!
//! # Examples
//!
//! ```rust
//! use rf_web::SessionFacade;
//! use serde_json::json;
//!
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
use std::collections::{HashMap, HashSet};
use std::sync::{Mutex, RwLock};

/// Cookie name carrying the per-client session id.
const SESSION_COOKIE: &str = "rf_session";

/// Data for a single client's session: the key/value map plus the flash bookkeeping
/// that gives flashed keys a one-request lifetime.
#[derive(Default)]
struct SessionData {
    /// Regular + flash values. Flash values live here too so `get`/`has` see them
    /// transparently for the one request they are alive.
    data: HashMap<String, Value>,
    /// Keys flashed during the CURRENT request (become `old` on the next request).
    flash_new: HashSet<String>,
    /// Keys flashed during the PREVIOUS request (removed on the next aging).
    flash_old: HashSet<String>,
}

impl SessionData {
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
static SESSIONS: Lazy<RwLock<HashMap<String, SessionData>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

/// Fallback session used by code paths that are NOT inside a request scope (unit
/// tests, CLI, startup). Never used while serving concurrent HTTP requests — each
/// of those runs inside its own [`session_scope`] task-local.
static FALLBACK: Lazy<Mutex<SessionData>> = Lazy::new(|| Mutex::new(SessionData::default()));

tokio::task_local! {
    /// The current request's session id, established by [`session_scope`].
    static CURRENT_SESSION_ID: String;
}

/// Run `f` against the active session: the per-request one keyed by the task-local
/// session id if a scope is established, otherwise the process-local fallback.
fn with_session<R>(f: impl FnOnce(&mut SessionData) -> R) -> R {
    let mut f = Some(f);
    let attempted = CURRENT_SESSION_ID.try_with(|sid| {
        let mut store = SESSIONS.write().unwrap();
        let entry = store.entry(sid.clone()).or_default();
        (f.take().unwrap())(entry)
    });
    match attempted {
        Ok(r) => r,
        Err(_) => (f.take().unwrap())(&mut FALLBACK.lock().unwrap()),
    }
}

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

/// Per-request middleware that establishes the current client's session scope,
/// mirroring `rf_request::capture_request` / `rf_auth::auth_scope`.
///
/// On each request it:
/// 1. reads the `rf_session` cookie, or mints a fresh session id if absent;
/// 2. ages that session's flash data (one-request lifetime);
/// 3. runs the handler inside a task-local scope so the [`SessionFacade`] reads
///    and writes ONLY this client's session; and
/// 4. sets/refreshes the `rf_session` cookie on the response.
///
/// ```ignore
/// use axum::{Router, routing::get, middleware};
/// use rf_web::session_scope;
/// let app = Router::new().route("/", get(handler))
///     .layer(middleware::from_fn(session_scope));
/// ```
pub async fn session_scope(req: Request, next: Next) -> Response {
    let session_id = cookie_session_id(&req).unwrap_or_else(generate_session_id);

    // Age flash at the start of the request so values flashed on the previous
    // request are readable now, then gone next time.
    {
        let mut store = SESSIONS.write().unwrap();
        store.entry(session_id.clone()).or_default().age_flash();
    }

    let mut response = CURRENT_SESSION_ID
        .scope(session_id.clone(), next.run(req))
        .await;

    let cookie = format!(
        "{}={}; Path=/; HttpOnly; SameSite=Lax",
        SESSION_COOKIE, session_id
    );
    if let Ok(value) = cookie.parse() {
        response.headers_mut().append(header::SET_COOKIE, value);
    }

    response
}

/// The SessionFacade providing a static API scoped to the current request's session.
///
/// Wire [`session_scope`] into your router so each client gets its own isolated
/// session; without it the facade operates on a single process-local fallback
/// (fine for tests/CLI, not for concurrent HTTP serving).
///
/// # Examples
///
/// ```rust
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Run a closure inside a fresh, uniquely-keyed session scope so tests do not
    /// clobber each other or the shared fallback.
    fn in_session<R>(id: &str, f: impl FnOnce() -> R) -> R {
        CURRENT_SESSION_ID.sync_scope(id.to_string(), f)
    }

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
}
