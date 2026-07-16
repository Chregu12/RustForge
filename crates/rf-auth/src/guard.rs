//! Authentication guard support

use crate::auth_manager::GLOBAL_AUTH;
use serde::Serialize;
use serde_json::Value;

/// Authentication guard for multi-guard authentication.
///
/// Guards allow you to use different authentication strategies
/// (e.g., "web" for sessions, "api" for tokens).
///
/// # Examples
///
/// ```ignore
/// use rf_auth::{Auth, auth_manager::with_auth_scope_sync};
///
/// with_auth_scope_sync(|| {
///     // Get a specific guard
///     let api_guard = Auth::guard("api");
///
///     // Check authentication on this guard
///     if api_guard.check() {
///         println!("Authenticated on API guard");
///     }
/// });
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
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.login(user)
    }

    /// Logout on this guard
    pub fn logout(&self) {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.logout();
    }

    /// Attempt login on this guard
    pub fn attempt(&self, credentials: Value) -> Result<bool, String> {
        let manager = GLOBAL_AUTH.write().unwrap();
        manager.attempt(credentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth_manager::with_auth_scope_sync;

    #[test]
    fn test_guard_creation() {
        let guard = Guard::new("api");
        assert_eq!(guard.name(), "api");
    }

    #[test]
    fn test_guard_check() {
        // Guard methods require a per-request auth scope — same as Auth facade.
        with_auth_scope_sync(|| {
            let guard = Guard::new("web");
            // Initially not authenticated
            assert!(!guard.check());
            assert!(guard.guest());
        });
    }
}
