//! Auth facade providing Laravel-style static authentication API
//!
//! All methods are simple to use - no `.await` needed!
//! I/O operations (like database queries) are handled internally.

use crate::guard::Guard;
use crate::auth_manager::GLOBAL_AUTH;
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

    /// Get the currently authenticated user
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
    /// if let Some(user) = Auth::user::<User>() {
    ///     println!("User email: {}", user.email);
    /// }
    /// ```
    pub fn user<T: for<'de> serde::Deserialize<'de>>() -> Option<T> {
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
        let mut manager = GLOBAL_AUTH.write().unwrap();
        manager.login(user)
    }

    /// Login a user with remember me
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

        let mut manager = GLOBAL_AUTH.write().unwrap();
        manager.login_with_remember(user, remember)
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
        let mut manager = GLOBAL_AUTH.write().unwrap();
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
        let mut manager = GLOBAL_AUTH.write().unwrap();
        manager.attempt(credentials)
    }

    /// Register the [`UserProvider`](crate::UserProvider) that [`attempt`](Self::attempt)
    /// uses to look up and verify credentials. Until one is set, `attempt` fails
    /// closed and authenticates no one.
    pub fn set_provider(provider: std::sync::Arc<dyn crate::UserProvider>) {
        let mut manager = GLOBAL_AUTH.write().unwrap();
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
        let mut manager = GLOBAL_AUTH.write().unwrap();
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

    #[test]
    fn test_auth_guard_creation() {
        let guard = Auth::guard("api");
        assert_eq!(guard.name(), "api");
    }

    #[test]
    fn test_auth_static_methods_exist() {
        // Just verify methods compile and are callable
        let _ = Auth::check();
        let _ = Auth::guest();
        let _ = Auth::id();
    }

    #[test]
    fn test_auth_login_logout() {
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
        struct TestUser {
            id: u64,
            email: String,
        }

        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
        };

        // Login
        Auth::login(user.clone()).unwrap();
        assert!(Auth::check());
        assert!(!Auth::guest());
        assert_eq!(Auth::id(), Some(1));

        // Get user
        let retrieved: Option<TestUser> = Auth::user();
        assert_eq!(retrieved, Some(user));

        // Logout
        Auth::logout();
        assert!(!Auth::check());
        assert!(Auth::guest());
    }
}
