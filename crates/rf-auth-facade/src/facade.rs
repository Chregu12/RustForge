//! Auth facade providing Laravel-style static authentication API

use crate::guard::Guard;
use crate::manager::GLOBAL_AUTH;
use serde::Serialize;
use serde_json::Value;

/// The Auth facade providing a static-like API for authentication.
///
/// This is the main entry point for authentication in your application.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_auth_facade::Auth;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize)]
/// struct User {
///     id: u64,
///     email: String,
///     name: String,
/// }
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Login
/// let user = User {
///     id: 1,
///     email: "user@example.com".to_string(),
///     name: "John Doe".to_string(),
/// };
/// Auth::login(user).await?;
///
/// // Check authentication
/// if Auth::check().await {
///     println!("Authenticated");
/// }
///
/// // Get current user
/// if let Some(user) = Auth::user::<User>().await {
///     println!("User: {}", user.name);
/// }
///
/// // Logout
/// Auth::logout().await;
/// # Ok(())
/// # }
/// ```
pub struct Auth;

impl Auth {
    /// Check if a user is authenticated
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if Auth::check().await {
    ///     println!("User is authenticated");
    /// }
    /// # }
    /// ```
    pub async fn check() -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.check()
    }

    /// Check if the current user is a guest (not authenticated)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if Auth::guest().await {
    ///     println!("User is not authenticated");
    /// }
    /// # }
    /// ```
    pub async fn guest() -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.guest()
    }

    /// Get the currently authenticated user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    ///     email: String,
    /// }
    ///
    /// # async fn example() {
    /// if let Some(user) = Auth::user::<User>().await {
    ///     println!("User email: {}", user.email);
    /// }
    /// # }
    /// ```
    pub async fn user<T: for<'de> serde::Deserialize<'de>>() -> Option<T> {
        let manager = GLOBAL_AUTH.read().await;
        manager.user()
    }

    /// Get the ID of the currently authenticated user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if let Some(id) = Auth::id().await {
    ///     println!("User ID: {}", id);
    /// }
    /// # }
    /// ```
    pub async fn id() -> Option<u64> {
        let manager = GLOBAL_AUTH.read().await;
        manager.id()
    }

    /// Login a user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    ///     email: String,
    /// }
    ///
    /// # async fn example() -> Result<(), String> {
    /// let user = User {
    ///     id: 1,
    ///     email: "user@example.com".to_string(),
    /// };
    ///
    /// Auth::login(user).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn login<T: Serialize>(user: T) -> Result<(), String> {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.login(user)
    }

    /// Login a user with remember me
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    /// use serde::{Serialize, Deserialize};
    ///
    /// #[derive(Serialize, Deserialize)]
    /// struct User {
    ///     id: u64,
    ///     email: String,
    /// }
    ///
    /// # async fn example() -> Result<(), String> {
    /// let user = User {
    ///     id: 1,
    ///     email: "user@example.com".to_string(),
    /// };
    ///
    /// Auth::login_using_id(1, true).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn login_using_id(id: u64, remember: bool) -> Result<(), String> {
        let user = serde_json::json!({
            "id": id,
        });

        let mut manager = GLOBAL_AUTH.write().await;
        manager.login_with_remember(user, remember)
    }

    /// Logout the currently authenticated user
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// Auth::logout().await;
    /// # }
    /// ```
    pub async fn logout() {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.logout();
    }

    /// Attempt to authenticate a user with credentials
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), String> {
    /// let credentials = json!({
    ///     "email": "user@example.com",
    ///     "password": "secret"
    /// });
    ///
    /// if Auth::attempt(credentials).await? {
    ///     println!("Login successful!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn attempt(credentials: Value) -> Result<bool, String> {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.attempt(credentials)
    }

    /// Check if the user was authenticated via remember me
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if Auth::via_remember().await {
    ///     println!("Authenticated via remember me");
    /// }
    /// # }
    /// ```
    pub async fn via_remember() -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.via_remember()
    }

    /// Get a guard instance
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// let api_guard = Auth::guard("api").await;
    /// if api_guard.check().await {
    ///     println!("Authenticated on API guard");
    /// }
    /// # }
    /// ```
    pub async fn guard(name: &str) -> Guard {
        Guard::new(name)
    }

    /// Check if user has a specific role
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if Auth::has_role("admin").await {
    ///     println!("User is an admin");
    /// }
    /// # }
    /// ```
    pub async fn has_role(role: &str) -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.has_role(role)
    }

    /// Check if user has any of the given roles
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if Auth::has_any_role(&["admin", "moderator"]).await {
    ///     println!("User has elevated privileges");
    /// }
    /// # }
    /// ```
    pub async fn has_any_role(roles: &[&str]) -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.has_any_role(roles)
    }

    /// Check if user has all of the given roles
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_auth_facade::Auth;
    ///
    /// # async fn example() {
    /// if Auth::has_all_roles(&["user", "verified"]).await {
    ///     println!("User is verified");
    /// }
    /// # }
    /// ```
    pub async fn has_all_roles(roles: &[&str]) -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.has_all_roles(roles)
    }

    /// Set the default guard
    pub async fn set_default_guard(guard: String) {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.set_guard(guard);
    }

    /// Get the name of the default guard
    pub async fn get_default_guard() -> String {
        let manager = GLOBAL_AUTH.read().await;
        manager.guard_name().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests with global state are omitted due to parallel execution issues
    // In production, use a test framework with serial execution or isolated state

    #[tokio::test]
    async fn test_auth_guard_creation() {
        let guard = Auth::guard("api").await;
        assert_eq!(guard.name(), "api");
    }

    #[tokio::test]
    async fn test_auth_static_methods_exist() {
        // Just verify methods compile and are callable
        let _ = Auth::check().await;
        let _ = Auth::guest().await;
        let _ = Auth::id().await;
    }
}
