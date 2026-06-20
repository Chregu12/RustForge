//! Global authentication manager

use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::RwLock;

/// Global authentication manager instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_AUTH: Lazy<RwLock<AuthManager>> = Lazy::new(|| {
    RwLock::new(AuthManager::new())
});

/// Serializes every test (across modules) that mutates the process-global
/// [`GLOBAL_AUTH`]. Without this, e.g. `guard::test_guard_check` resets the
/// global state with `logout()` while `facade::test_auth_login_logout` is
/// mid-assertion, intermittently failing `Auth::check()`. Lock this guard at
/// the start of any test that logs in/out through the global manager.
/// `into_inner` ignores poisoning so one failing test does not cascade.
#[cfg(test)]
pub(crate) static AUTH_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Authentication manager that holds the current authentication state
#[derive(Debug)]
pub struct AuthManager {
    /// Currently authenticated user (as JSON)
    current_user: Option<Value>,
    /// Remember me state
    via_remember: bool,
    /// Current guard name
    guard: String,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new() -> Self {
        Self {
            current_user: None,
            via_remember: false,
            guard: "web".to_string(),
        }
    }

    /// Check if a user is authenticated
    pub fn check(&self) -> bool {
        self.current_user.is_some()
    }

    /// Check if a user is not authenticated
    pub fn guest(&self) -> bool {
        self.current_user.is_none()
    }

    /// Get the currently authenticated user
    pub fn user<T: for<'de> Deserialize<'de>>(&self) -> Option<T> {
        self.current_user.as_ref().and_then(|user| {
            serde_json::from_value(user.clone()).ok()
        })
    }

    /// Get the ID of the currently authenticated user
    pub fn id(&self) -> Option<u64> {
        self.current_user.as_ref().and_then(|user| {
            user.get("id")
                .and_then(|id| id.as_u64())
        })
    }

    /// Login a user
    pub fn login<T: Serialize>(&mut self, user: T) -> Result<(), String> {
        let user_json = serde_json::to_value(user)
            .map_err(|e| format!("Failed to serialize user: {}", e))?;

        self.current_user = Some(user_json);
        self.via_remember = false;
        Ok(())
    }

    /// Login a user with remember me
    pub fn login_with_remember<T: Serialize>(&mut self, user: T, remember: bool) -> Result<(), String> {
        self.login(user)?;
        self.via_remember = remember;
        Ok(())
    }

    /// Logout the current user
    pub fn logout(&mut self) {
        self.current_user = None;
        self.via_remember = false;
    }

    /// Attempt login with credentials
    pub fn attempt(&mut self, credentials: Value) -> Result<bool, String> {
        // This is a simplified version
        // In a real implementation, this would:
        // 1. Query the database for the user
        // 2. Verify the password
        // 3. Login if successful

        // For now, we just check if credentials have email and password
        if credentials.get("email").is_some() && credentials.get("password").is_some() {
            // Mock user login
            let mock_user = serde_json::json!({
                "id": 1,
                "email": credentials["email"],
                "name": "Mock User"
            });
            self.current_user = Some(mock_user);
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Check if the user was authenticated via remember me
    pub fn via_remember(&self) -> bool {
        self.via_remember
    }

    /// Get the current guard name
    pub fn guard_name(&self) -> &str {
        &self.guard
    }

    /// Set the current guard
    pub fn set_guard(&mut self, guard: String) {
        self.guard = guard;
    }

    /// Check if user has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        if let Some(user) = &self.current_user {
            if let Some(roles) = user.get("roles").and_then(|r| r.as_array()) {
                return roles.iter().any(|r| r.as_str() == Some(role));
            }
        }
        false
    }

    /// Check if user has any of the given roles
    pub fn has_any_role(&self, roles: &[&str]) -> bool {
        roles.iter().any(|role| self.has_role(role))
    }

    /// Check if user has all of the given roles
    pub fn has_all_roles(&self, roles: &[&str]) -> bool {
        roles.iter().all(|role| self.has_role(role))
    }
}

impl Default for AuthManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestUser {
        id: u64,
        email: String,
        name: String,
    }

    #[test]
    fn test_auth_manager_new() {
        let manager = AuthManager::new();
        assert!(!manager.check());
        assert!(manager.guest());
    }

    #[test]
    fn test_auth_manager_login() {
        let mut manager = AuthManager::new();
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login(user.clone()).unwrap();
        assert!(manager.check());
        assert!(!manager.guest());

        let current: Option<TestUser> = manager.user();
        assert_eq!(current, Some(user));
    }

    #[test]
    fn test_auth_manager_logout() {
        let mut manager = AuthManager::new();
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login(user).unwrap();
        assert!(manager.check());

        manager.logout();
        assert!(!manager.check());
        assert!(manager.guest());
    }

    #[test]
    fn test_auth_manager_id() {
        let mut manager = AuthManager::new();
        assert_eq!(manager.id(), None);

        let user = TestUser {
            id: 42,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login(user).unwrap();
        assert_eq!(manager.id(), Some(42));
    }

    #[test]
    fn test_auth_manager_via_remember() {
        let mut manager = AuthManager::new();
        let user = TestUser {
            id: 1,
            email: "test@example.com".to_string(),
            name: "Test User".to_string(),
        };

        manager.login_with_remember(user, true).unwrap();
        assert!(manager.via_remember());
    }

    #[test]
    fn test_auth_manager_attempt() {
        let mut manager = AuthManager::new();

        let credentials = serde_json::json!({
            "email": "test@example.com",
            "password": "secret"
        });

        let result = manager.attempt(credentials).unwrap();
        assert!(result);
        assert!(manager.check());
    }

    #[test]
    fn test_auth_manager_guard() {
        let mut manager = AuthManager::new();
        assert_eq!(manager.guard_name(), "web");

        manager.set_guard("api".to_string());
        assert_eq!(manager.guard_name(), "api");
    }
}
