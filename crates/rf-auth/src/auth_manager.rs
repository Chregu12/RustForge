//! Request-scoped authentication manager.
//!
//! The authenticated user is held in a **per-request** task-local scope, not a
//! single process-global instance, so concurrent requests can never see each
//! other's login state. The public API is unchanged: `GLOBAL_AUTH.read()/.write()`
//! and `AuthManager` still work, but `AuthManager` is now a thin proxy over the
//! active scope. Establish a scope per request with [`with_auth_scope`] (or the
//! [`crate::middleware`] auth-scope middleware).

use crate::password::PasswordHasher;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::future::Future;
use std::sync::{Arc, Mutex, RwLock};

/// Resolves login credentials to a stored user record for [`AuthManager::attempt`].
///
/// Implement this on your app's user store (database, in-memory, etc.). Given the
/// submitted `credentials` (e.g. `{"email": ..., "password": ...}`), return the
/// matching user record **including its hashed password** so that `attempt` can
/// verify it, or `None` if no user matches the identifier. `attempt` never trusts
/// the submitted password directly — it always verifies it against the hash in the
/// record returned here.
pub trait UserProvider: Send + Sync {
    /// Look up the stored user record by its login identifier (not the password).
    fn retrieve_by_credentials(&self, credentials: &Value) -> Option<Value>;

    /// Look up a stored user record by its primary id. Used by the *verifying*
    /// [`AuthManager::login_using_id_verified`] path to confirm the id resolves to a
    /// real user before establishing the identity. The default implementation
    /// delegates to [`retrieve_by_credentials`](Self::retrieve_by_credentials) with
    /// `{"id": id}`; override it if your store keys users differently.
    fn retrieve_by_id(&self, id: u64) -> Option<Value> {
        self.retrieve_by_credentials(&serde_json::json!({ "id": id }))
    }

    /// Name of the field in the returned record holding the (hashed) password.
    fn password_field(&self) -> &str {
        "password"
    }
}

/// The mutable per-request authentication state.
#[derive(Default)]
struct AuthState {
    /// Currently authenticated user (as JSON).
    current_user: Option<Value>,
    /// Remember-me flag.
    via_remember: bool,
    /// Current guard name (`None` == the default "web").
    guard: Option<String>,
    /// Per-scope provider override; falls back to [`DEFAULT_PROVIDER`] when `None`.
    provider: Option<Arc<dyn UserProvider>>,
}

impl AuthState {
    fn guard_name(&self) -> String {
        self.guard.clone().unwrap_or_else(|| "web".to_string())
    }
}

tokio::task_local! {
    /// The per-request authentication state, established by [`with_auth_scope`].
    static AUTH_STATE: RefCell<AuthState>;
}

/// Process-global default [`UserProvider`], set once at startup via
/// `Auth::set_provider`. Per-request scopes with no override fall back to this.
static DEFAULT_PROVIDER: Lazy<RwLock<Option<Arc<dyn UserProvider>>>> =
    Lazy::new(|| RwLock::new(None));

/// Fallback state used by code paths that are NOT inside a per-request auth scope
/// (CLI, unit tests, startup). It is never used while serving concurrent HTTP
/// requests — each of those runs inside its own task-local [`AUTH_STATE`] scope.
static FALLBACK_STATE: Lazy<Mutex<AuthState>> = Lazy::new(|| Mutex::new(AuthState::default()));

/// Run `f` against the active auth state: the per-request task-local scope if one
/// is established, otherwise the process-global fallback.
fn with_state<R>(f: impl FnOnce(&mut AuthState) -> R) -> R {
    let mut f = Some(f);
    let attempted = AUTH_STATE.try_with(|cell| (f.take().unwrap())(&mut cell.borrow_mut()));
    match attempted {
        Ok(r) => r,
        Err(_) => (f.take().unwrap())(&mut FALLBACK_STATE.lock().unwrap()),
    }
}

/// True if the current task is running inside a per-request auth scope established
/// by [`with_auth_scope`] (or by the [`crate::middleware::auth_scope`] /
/// [`crate::middleware::require_auth`] middlewares).
///
/// This is used by [`crate::facade::Auth::user`] to distinguish "no scope at all
/// (programming error — missing middleware)" from "scope present but no user logged
/// in (legitimate optional-auth route)".
pub fn in_auth_scope() -> bool {
    AUTH_STATE.try_with(|_| ()).is_ok()
}

/// Run an async request handler inside a fresh per-request auth scope, so its
/// login state cannot leak into other concurrent requests. This is what the
/// auth-scope middleware wraps each request in.
pub async fn with_auth_scope<F, R>(fut: F) -> R
where
    F: Future<Output = R>,
{
    AUTH_STATE.scope(RefCell::new(AuthState::default()), fut).await
}

/// Synchronous variant of [`with_auth_scope`] for non-async contexts (tests, CLI).
pub fn with_auth_scope_sync<R>(f: impl FnOnce() -> R) -> R {
    AUTH_STATE.sync_scope(RefCell::new(AuthState::default()), f)
}

/// Authentication manager — a thin proxy over the active [`AuthState`] scope.
///
/// It carries no state itself; all reads/writes go to the per-request task-local
/// scope (or the process-global fallback outside a scope).
#[derive(Debug, Clone, Copy, Default)]
pub struct AuthManager;

impl AuthManager {
    /// Create a manager proxy. (State lives in the active scope, not in the proxy.)
    pub fn new() -> Self {
        AuthManager
    }

    /// Register the [`UserProvider`] used by [`attempt`](Self::attempt). Inside a
    /// request/test scope this sets the scope-local provider; at app startup (no
    /// scope) it sets the process-global default that every request inherits.
    pub fn set_provider(&self, provider: Arc<dyn UserProvider>) {
        if in_auth_scope() {
            with_state(|s| s.provider = Some(provider));
        } else {
            *DEFAULT_PROVIDER.write().unwrap() = Some(provider);
        }
    }

    /// The provider that applies to the current scope (override, else the default).
    fn active_provider(&self) -> Option<Arc<dyn UserProvider>> {
        with_state(|s| s.provider.clone())
            .or_else(|| DEFAULT_PROVIDER.read().unwrap().clone())
    }

    /// Check if a user is authenticated in the current scope.
    pub fn check(&self) -> bool {
        with_state(|s| s.current_user.is_some())
    }

    /// Check if the current scope has no authenticated user.
    pub fn guest(&self) -> bool {
        with_state(|s| s.current_user.is_none())
    }

    /// Get the currently authenticated user.
    pub fn user<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        with_state(|s| {
            s.current_user
                .as_ref()
                .and_then(|user| serde_json::from_value(user.clone()).ok())
        })
    }

    /// Get the ID of the currently authenticated user.
    pub fn id(&self) -> Option<u64> {
        with_state(|s| {
            s.current_user
                .as_ref()
                .and_then(|user| user.get("id").and_then(|id| id.as_u64()))
        })
    }

    /// Login a user into the current scope.
    pub fn login<T: Serialize>(&self, user: T) -> Result<(), String> {
        let user_json =
            serde_json::to_value(user).map_err(|e| format!("Failed to serialize user: {}", e))?;
        with_state(|s| {
            s.current_user = Some(user_json);
            s.via_remember = false;
        });
        Ok(())
    }

    /// Login a user with a remember-me flag.
    pub fn login_with_remember<T: Serialize>(
        &self,
        user: T,
        remember: bool,
    ) -> Result<(), String> {
        self.login(user)?;
        with_state(|s| s.via_remember = remember);
        Ok(())
    }

    /// **Verifying** login-by-id: only establishes the identity if `id` resolves to
    /// a real user via the active [`UserProvider`].
    ///
    /// Unlike the trusting facade `Auth::login_using_id` (which fabricates
    /// `{"id": id}` for any integer), this looks the user up through
    /// [`UserProvider::retrieve_by_id`] first. Returns `Ok(true)` and logs the user
    /// in (password field stripped) when the id matches a stored user; `Ok(false)`
    /// when no provider is configured or no user has that id — a phantom id is never
    /// authorized.
    pub fn login_using_id_verified(&self, id: u64, remember: bool) -> Result<bool, String> {
        let provider = match self.active_provider() {
            Some(p) => p,
            None => return Ok(false),
        };

        let record = match provider.retrieve_by_id(id) {
            Some(r) => r,
            None => return Ok(false),
        };

        // Never keep the password hash in the session state.
        let mut user = record;
        if let Some(obj) = user.as_object_mut() {
            obj.remove(provider.password_field());
        }
        with_state(|s| {
            s.current_user = Some(user);
            s.via_remember = remember;
        });
        Ok(true)
    }

    /// Logout the current scope's user.
    pub fn logout(&self) {
        with_state(|s| {
            s.current_user = None;
            s.via_remember = false;
        });
    }

    /// Attempt to authenticate with the given credentials.
    ///
    /// Looks the user up through the active [`UserProvider`] and verifies the
    /// submitted password against the stored hash (bcrypt/argon2 auto-detected). On
    /// success the user is logged into the current scope (password field stripped)
    /// and `true` is returned; on any mismatch `false`.
    ///
    /// **Fail-closed**: if no provider is configured, the user is not found, or the
    /// record has no usable password hash, authentication is denied. It never
    /// authenticates unverified input.
    pub fn attempt(&self, credentials: Value) -> Result<bool, String> {
        let provider = match self.active_provider() {
            Some(p) => p,
            None => return Ok(false),
        };

        let password = match credentials.get("password").and_then(Value::as_str) {
            Some(p) => p.to_string(),
            None => return Ok(false),
        };

        let record = match provider.retrieve_by_credentials(&credentials) {
            Some(r) => r,
            None => return Ok(false),
        };

        let hash = match record.get(provider.password_field()).and_then(Value::as_str) {
            Some(h) => h.to_string(),
            None => return Ok(false),
        };

        let verified = PasswordHasher::default()
            .verify(&password, &hash)
            .map_err(|e| format!("Password verification failed: {}", e))?;

        if !verified {
            return Ok(false);
        }

        // Log the user in, but never keep the password hash in the session state.
        let mut user = record;
        if let Some(obj) = user.as_object_mut() {
            obj.remove(provider.password_field());
        }
        with_state(|s| {
            s.current_user = Some(user);
            s.via_remember = false;
        });
        Ok(true)
    }

    /// Whether the current user was authenticated via remember-me.
    pub fn via_remember(&self) -> bool {
        with_state(|s| s.via_remember)
    }

    /// The current guard name.
    pub fn guard_name(&self) -> String {
        with_state(|s| s.guard_name())
    }

    /// Set the current guard.
    pub fn set_guard(&self, guard: String) {
        with_state(|s| s.guard = Some(guard));
    }

    /// Check if the current user has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        with_state(|s| {
            if let Some(user) = &s.current_user {
                if let Some(roles) = user.get("roles").and_then(|r| r.as_array()) {
                    return roles.iter().any(|r| r.as_str() == Some(role));
                }
            }
            false
        })
    }

    /// Check if the current user has any of the given roles.
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if the current user has all of the given roles.
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

/// Accessor that preserves the historical `GLOBAL_AUTH.read()/.write().unwrap()`
/// call sites, now backed by the per-request scope instead of one shared instance.
pub struct GlobalAuth;

/// Global entry point for the auth manager proxy. `read()`/`write()` both hand out
/// an [`AuthManager`] proxy over the active scope (the names are kept only for
/// source compatibility; there is no lock to contend on).
pub static GLOBAL_AUTH: GlobalAuth = GlobalAuth;

impl GlobalAuth {
    /// Get a manager proxy (compatibility shim; never fails).
    pub fn read(&self) -> Result<AuthManager, std::convert::Infallible> {
        Ok(AuthManager)
    }

    /// Get a manager proxy (compatibility shim; never fails).
    pub fn write(&self) -> Result<AuthManager, std::convert::Infallible> {
        Ok(AuthManager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test provider backing a single user with a real bcrypt-hashed password.
    struct TestProvider {
        email: String,
        password_hash: String,
    }

    impl UserProvider for TestProvider {
        fn retrieve_by_credentials(&self, credentials: &Value) -> Option<Value> {
            let email = credentials.get("email").and_then(Value::as_str)?;
            if email == self.email {
                Some(serde_json::json!({
                    "id": 1,
                    "email": self.email,
                    "name": "Real User",
                    "password": self.password_hash,
                }))
            } else {
                None
            }
        }
    }

    fn test_provider() -> Arc<dyn UserProvider> {
        let hash = PasswordHasher::bcrypt(4).unwrap().hash("secret").unwrap();
        Arc::new(TestProvider {
            email: "test@example.com".to_string(),
            password_hash: hash,
        })
    }

    #[test]
    fn test_attempt_fails_closed_without_provider() {
        with_auth_scope_sync(|| {
            let m = AuthManager;
            let credentials =
                serde_json::json!({"email": "test@example.com", "password": "secret"});
            assert!(!m.attempt(credentials).unwrap());
            assert!(!m.check());
        });
    }

    #[test]
    fn test_attempt_succeeds_with_correct_password() {
        with_auth_scope_sync(|| {
            let m = AuthManager;
            m.set_provider(test_provider()); // scope-local (we're in a scope)
            let credentials =
                serde_json::json!({"email": "test@example.com", "password": "secret"});
            assert!(m.attempt(credentials).unwrap());
            assert!(m.check());
            // Password hash must never be exposed in the session state.
            assert!(m.user::<Value>().unwrap().get("password").is_none());
        });
    }

    #[test]
    fn test_attempt_rejects_wrong_password_and_unknown_user() {
        with_auth_scope_sync(|| {
            let m = AuthManager;
            m.set_provider(test_provider());
            assert!(!m
                .attempt(serde_json::json!({"email": "test@example.com", "password": "nope"}))
                .unwrap());
            assert!(!m
                .attempt(serde_json::json!({"email": "ghost@example.com", "password": "secret"}))
                .unwrap());
            assert!(!m.check());
        });
    }

    /// Provider that only knows a single user with id 1 (id-keyed lookup).
    struct IdProvider;
    impl UserProvider for IdProvider {
        fn retrieve_by_credentials(&self, _credentials: &Value) -> Option<Value> {
            None
        }
        fn retrieve_by_id(&self, id: u64) -> Option<Value> {
            if id == 1 {
                Some(serde_json::json!({
                    "id": 1,
                    "name": "Real User",
                    "password": "irrelevant-hash",
                }))
            } else {
                None
            }
        }
    }

    #[test]
    fn test_login_using_id_verified_rejects_phantom_and_accepts_real() {
        // A5 regression: the verifying path must reject an id for a non-existent
        // user while accepting a real one — no phantom authorization.
        with_auth_scope_sync(|| {
            let m = AuthManager;
            m.set_provider(Arc::new(IdProvider));

            // Phantom id -> rejected, no session established.
            assert!(!m.login_using_id_verified(999, false).unwrap());
            assert!(!m.check());

            // Real id -> accepted, logged in, password stripped.
            assert!(m.login_using_id_verified(1, true).unwrap());
            assert!(m.check());
            assert_eq!(m.id(), Some(1));
            assert!(m.via_remember());
            assert!(m.user::<Value>().unwrap().get("password").is_none());
        });
    }

    #[test]
    fn test_login_using_id_verified_fails_closed_without_provider() {
        with_auth_scope_sync(|| {
            let m = AuthManager;
            // No provider configured -> even a plausible id is not authorized.
            assert!(!m.login_using_id_verified(1, false).unwrap());
            assert!(!m.check());
        });
    }

    #[test]
    fn test_guard_name_default_and_set() {
        with_auth_scope_sync(|| {
            let m = AuthManager;
            assert_eq!(m.guard_name(), "web");
            m.set_guard("api".to_string());
            assert_eq!(m.guard_name(), "api");
        });
    }

    #[tokio::test]
    async fn test_login_state_is_isolated_between_concurrent_scopes() {
        // Two concurrent request scopes must not see each other's user.
        let a = with_auth_scope(async {
            let m = AuthManager;
            m.login(serde_json::json!({"id": 1, "name": "Alice"})).unwrap();
            // yield so the other task interleaves while we're "logged in"
            tokio::task::yield_now().await;
            (m.check(), m.id())
        });
        let b = with_auth_scope(async {
            let m = AuthManager;
            tokio::task::yield_now().await;
            // b never logged in -> must be a guest despite a's concurrent login
            (m.check(), m.id())
        });
        let ((a_check, a_id), (b_check, b_id)) = tokio::join!(a, b);
        assert!(a_check && a_id == Some(1), "scope A keeps its own user");
        assert!(!b_check && b_id.is_none(), "scope B is NOT authenticated by A's login");
    }
}
