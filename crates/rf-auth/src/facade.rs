//! Auth facade providing Laravel-style static authentication API
//!
//! All methods are simple to use - no `.await` needed!
//! I/O operations (like database queries) are handled internally.

use crate::auth_manager::{in_auth_scope, GLOBAL_AUTH};
use crate::guard::Guard;
use serde::Serialize;
use serde_json::Value;

/// The Auth facade providing a static-like API for authentication.
///
/// Simple, Laravel-style API - no `.await` needed anywhere!
///
/// # Examples
///
/// ```rust
/// use rf_auth::Auth;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct User {
///     id: u64,
///     email: String,
///     name: String,
/// }
///
/// fn example() -> Result<(), String> {
///     // Login
///     let user = User { id: 1, email: "user@example.com".into(), name: "John".into() };
///     Auth::login(user)?;
///
///     // Check authentication
///     if Auth::check() {
///         println!("User is authenticated");
///     }
///
///     // Get current user
///     if let Some(user) = Auth::user::<User>() {
///         println!("Welcome, {}", user.name);
///     }
///
///     // Attempt login with credentials
///     let credentials = serde_json::json!({
///         "email": "user@example.com",
///         "password": "secret"
///     });
///     if Auth::attempt(credentials)? {
///         println!("Login successful!");
///     }
///
///     // Logout
///     Auth::logout();
///     Ok(())
/// }
/// ```
pub struct Auth;

impl Auth {
    /// Check if a user is authenticated
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if Auth::check() {
    ///     println!("User is authenticated");
    /// }
    /// ```
    pub fn check() -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.check()
    }

    /// Check if the current user is a guest (not authenticated)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if Auth::guest() {
    ///     println!("User is not authenticated");
    /// }
    /// ```
    pub fn guest() -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.guest()
    }

    /// Get the currently authenticated user.
    ///
    /// Returns `None` when the scope is set but no user is logged in — the
    /// **legitimate** guest case (e.g. an optional-auth route).
    ///
    /// # Panics
    ///
    /// Panics with a diagnostic message when called **outside a
    /// `with_auth_scope`** (i.e. without the `auth_scope`, `require_auth`, or
    /// `require_auth_with` middleware in the router).  A silent `None` here
    /// would hide a missing-middleware bug as if the user were simply a guest,
    /// so a panic is the correct fail-fast signal in both debug and release.
    ///
    /// ```text
    /// thread 'main' panicked at 'Auth::user() called outside a with_auth_scope …'
    /// ```
    ///
    /// To avoid the panic in tests, wrap your test body with
    /// [`with_auth_scope_sync`](crate::with_auth_scope_sync):
    ///
    /// ```ignore
    /// use rf_auth::{Auth, with_auth_scope_sync};
    ///
    /// with_auth_scope_sync(|| {
    ///     assert!(Auth::user::<serde_json::Value>().is_none()); // guest — legit
    /// });
    /// ```
    ///
    /// # Examples
    ///
    /// ```ignore
    /// use rf_auth::Auth;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    ///     email: String,
    /// }
    ///
    /// // Inside a handler behind `require_auth` / `auth_scope` middleware:
    /// if let Some(user) = Auth::user::<User>() {
    ///     println!("User email: {}", user.email);
    /// }
    /// ```
    pub fn user<T: for<'de> serde::Deserialize<'de>>() -> Option<T> {
        if !in_auth_scope() {
            panic!(
                "Auth::user() called outside a with_auth_scope — \
                 add the auth_scope middleware (or require_auth / require_auth_with) \
                 to your router, or wrap tests with `with_auth_scope_sync`"
            );
        }
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.user()
    }

    /// Get the ID of the currently authenticated user
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if let Some(id) = Auth::id() {
    ///     println!("User ID: {}", id);
    /// }
    /// ```
    pub fn id() -> Option<u64> {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.id()
    }

    /// Login a user
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    ///     email: String,
    /// }
    ///
    /// let user = User {
    ///     id: 1,
    ///     email: "user@example.com".to_string(),
    /// };
    ///
    /// Auth::login(user).unwrap();
    /// assert!(Auth::check());
    /// ```
    pub fn login<T: Serialize>(user: T) -> Result<(), String> {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.login(user)
    }

    /// Login by id, **trusting the caller**.
    ///
    /// This fabricates an identity `{"id": id}` and logs it in *without* checking
    /// that the id belongs to a real user. Use it only when the id has already been
    /// validated (e.g. freshly decoded from a signed token you issued). For a bearer
    /// / user-supplied id, prefer [`login_using_id_verified`](Self::login_using_id_verified),
    /// which rejects phantom ids.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// Auth::login_using_id(1, true).unwrap();
    /// assert!(Auth::check());
    /// ```
    pub fn login_using_id(id: u64, remember: bool) -> Result<(), String> {
        let user = serde_json::json!({
            "id": id,
        });

        let manager = GLOBAL_AUTH.write().unwrap();
        manager.login_with_remember(user, remember)
    }

    /// **Verifying** login by id: only logs in if `id` resolves to a real user via
    /// the configured [`UserProvider`](crate::UserProvider).
    ///
    /// Unlike [`login_using_id`](Self::login_using_id), this does not trust the
    /// caller: it looks the user up through
    /// [`UserProvider::retrieve_by_id`](crate::UserProvider::retrieve_by_id) and
    /// returns `Ok(true)` only when a stored user has that id (logging them in with
    /// the password field stripped). It returns `Ok(false)` when no provider is
    /// configured or no such user exists — so a bearer/id for a non-existent user is
    /// rejected instead of authorizing a phantom user.
    ///
    /// Register the provider once at startup with
    /// [`set_provider`](Self::set_provider).
    pub fn login_using_id_verified(id: u64, remember: bool) -> Result<bool, String> {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.login_using_id_verified(id, remember)
    }

    /// Logout the currently authenticated user
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// Auth::logout();
    /// assert!(Auth::guest());
    /// ```
    pub fn logout() {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.logout();
    }

    /// Attempt to authenticate a user with credentials
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    /// use serde_json::json;
    ///
    /// let credentials = json!({
    ///     "email": "user@example.com",
    ///     "password": "secret"
    /// });
    ///
    /// if Auth::attempt(credentials).unwrap() {
    ///     println!("Login successful!");
    /// }
    /// ```
    pub fn attempt(credentials: Value) -> Result<bool, String> {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.attempt(credentials)
    }

    /// Register the [`UserProvider`](crate::UserProvider) that [`attempt`](Self::attempt)
    /// uses to look up and verify credentials. Until one is set, `attempt` fails
    /// closed and authenticates no one.
    pub fn set_provider(provider: std::sync::Arc<dyn crate::UserProvider>) {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.set_provider(provider);
    }

    /// Check if the user was authenticated via remember me
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if Auth::via_remember() {
    ///     println!("Authenticated via remember me");
    /// }
    /// ```
    pub fn via_remember() -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.via_remember()
    }

    /// Get a guard instance
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// let api_guard = Auth::guard("api");
    /// if api_guard.check() {
    ///     println!("Authenticated on API guard");
    /// }
    /// ```
    pub fn guard(name: &str) -> Guard {
        Guard::new(name)
    }

    /// Check if user has a specific role
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if Auth::has_role("admin") {
    ///     println!("User is an admin");
    /// }
    /// ```
    pub fn has_role(role: &str) -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.has_role(role)
    }

    /// Check if user has any of the given roles
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if Auth::has_any_role(&["admin", "moderator"]) {
    ///     println!("User has elevated privileges");
    /// }
    /// ```
    pub fn has_any_role(roles: &[&str]) -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.has_any_role(roles)
    }

    /// Check if user has all of the given roles
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_auth::Auth;
    ///
    /// if Auth::has_all_roles(&["user", "verified"]) {
    ///     println!("User is verified");
    /// }
    /// ```
    pub fn has_all_roles(roles: &[&str]) -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.has_all_roles(roles)
    }

    /// Set the default guard
    pub fn set_default_guard(guard: String) {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.set_guard(guard);
    }

    /// Get the name of the default guard
    pub fn get_default_guard() -> String {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.guard_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_manager::with_auth_scope_sync;

    #[test]
    fn test_auth_guard_creation() {
        let guard = Auth::guard("api");
        assert_eq!(guard.name(), "api");
    }

    #[test]
    fn test_auth_static_methods_exist() {
        // Just verify non-scope-gated methods compile and are callable.
        // Auth::check / guest / id do NOT require a scope (they fall back to the
        // process-global state for non-HTTP contexts). Auth::user() does, so it is
        // tested separately below.
        let _ = Auth::check();
        let _ = Auth::guest();
        let _ = Auth::id();
    }

    /// Login → check → get user → logout cycle — must be inside a scope so that
    /// `Auth::user()` (which is scope-gated) works correctly.
    #[test]
    fn test_auth_login_logout() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct TestUser {
            id: u64,
            email: String,
        }

        with_auth_scope_sync(|| {
            let user = TestUser {
                id: 1,
                email: "test@example.com".to_string(),
            };

            // Login
            Auth::login(user.clone()).unwrap();
            assert!(Auth::check());
            assert!(!Auth::guest());
            assert_eq!(Auth::id(), Some(1));

            // Get user — inside a scope, absent user is legit None; here user IS set.
            let retrieved: Option<TestUser> = Auth::user();
            assert_eq!(retrieved, Some(user));

            // Logout
            Auth::logout();
            assert!(!Auth::check());
            assert!(Auth::guest());
        });
    }

    // ── Fail-fast tests for Auth::user() ─────────────────────────────────────

    /// `Auth::user()` called with NO auth scope must panic, not silently return `None`.
    /// A missing-middleware bug must never be masked as "just a guest".
    #[test]
    #[should_panic(expected = "Auth::user() called outside a with_auth_scope")]
    fn test_auth_user_panics_outside_scope() {
        let _: Option<serde_json::Value> = Auth::user();
    }

    /// Inside a scope with no user logged in → `None` is the **legitimate** guest
    /// result (optional-auth route), NOT a panic.
    #[test]
    fn test_auth_user_returns_none_inside_scope_when_no_user_logged_in() {
        with_auth_scope_sync(|| {
            let user: Option<serde_json::Value> = Auth::user();
            assert!(
                user.is_none(),
                "scope present but no user logged in → None is legit (no panic)"
            );
        });
    }

    /// Inside a scope with a user logged in → returns the user (not None, not a panic).
    #[test]
    fn test_auth_user_returns_user_inside_scope_when_logged_in() {
        with_auth_scope_sync(|| {
            Auth::login(serde_json::json!({"id": 42, "email": "alice@example.com"})).unwrap();
            let user: Option<serde_json::Value> = Auth::user();
            assert!(user.is_some());
            assert_eq!(
                user.unwrap().get("id").and_then(|v| v.as_u64()),
                Some(42)
            );
        });
    }
}
