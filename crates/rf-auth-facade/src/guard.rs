//! Authentication guard support

use crate::manager::GLOBAL_AUTH;
use serde::Serialize;
use serde_json::Value;

/// Authentication guard
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
    pub async fn check(&self) -> bool {
        let manager = GLOBAL_AUTH.read().await;
        manager.check()
    }

    /// Get the user for this guard
    pub async fn user<T: for<'de> serde::Deserialize<'de>>(&self) -> Option<T> {
        let manager = GLOBAL_AUTH.read().await;
        manager.user()
    }

    /// Login a user on this guard
    pub async fn login<T: Serialize>(&self, user: T) -> Result<(), String> {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.login(user)
    }

    /// Logout on this guard
    pub async fn logout(&self) {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.logout();
    }

    /// Attempt login on this guard
    pub async fn attempt(&self, credentials: Value) -> Result<bool, String> {
        let mut manager = GLOBAL_AUTH.write().await;
        manager.attempt(credentials)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_guard_creation() {
        let guard = Guard::new("api");
        assert_eq!(guard.name(), "api");
    }

    #[tokio::test]
    async fn test_guard_check() {
        let guard = Guard::new("web");
        assert!(!guard.check().await);
    }
}
