//! Authentication guard support

use crate::manager::GLOBAL_AUTH;
use serde::Serialize;
use serde_json::Value;

/// Authentication guard for multi-guard authentication.
///
/// Guards allow you to use different authentication strategies
/// (e.g., "web" for sessions, "api" for tokens).
///
/// # Examples
///
/// ```rust
/// use rf_auth_facade::Auth;
///
/// // Get a specific guard
/// let api_guard = Auth::guard("api");
///
/// // Check authentication on this guard
/// if api_guard.check() {
///     println!("Authenticated on API guard");
/// }
/// ```
pub struct Guard {
    name: String,
}

impl Guard {
    /// Create a new guard
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
        }
    }

    /// Get the guard name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Check if authenticated on this guard
    pub fn check(&self) -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.check()
    }

    /// Check if guest on this guard
    pub fn guest(&self) -> bool {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.guest()
    }

    /// Get the user for this guard
    pub fn user<T: for<'de> serde::Deserialize<'de>>(&self) -> Option<T> {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.user()
    }

    /// Get the user ID for this guard
    pub fn id(&self) -> Option<u64> {
        let manager = GLOBAL_AUTH.read().unwrap();
        manager.id()
    }

    /// Login a user on this guard
    pub fn login<T: Serialize>(&self, user: T) -> Result<(), String> {
        let mut manager = GLOBAL_AUTH.write().unwrap();
        manager.login(user)
    }

    /// Logout on this guard
    pub fn logout(&self) {
        let mut manager = GLOBAL_AUTH.write().unwrap();
        manager.logout();
    }

    /// Attempt login on this guard
    pub fn attempt(&self, credentials: Value) -> Result<bool, String> {
        let mut manager = GLOBAL_AUTH.write().unwrap();
        manager.attempt(credentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_creation() {
        let guard = Guard::new("api");
        assert_eq!(guard.name(), "api");
    }

    #[test]
    fn test_guard_check() {
        // Serialize against other tests that mutate the global auth state.
        let _guard = crate::manager::AUTH_TEST_GUARD
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // Reset global auth state (may be dirty from other tests)
        GLOBAL_AUTH.write().unwrap().logout();

        let guard = Guard::new("web");
        // Initially not authenticated
        assert!(!guard.check());
        assert!(guard.guest());
    }
}
