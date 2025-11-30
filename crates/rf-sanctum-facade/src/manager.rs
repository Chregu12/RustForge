//! Global Sanctum manager for facade pattern

use chrono::{DateTime, Utc};
use once_cell::sync::Lazy;
use rf_sanctum::{PersonalAccessToken, TokenStats};
use sea_orm::DatabaseConnection;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Global Sanctum manager instance
pub static GLOBAL_SANCTUM: Lazy<Arc<RwLock<SanctumManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(SanctumManager::new()))
});

/// Sanctum manager that holds the current authentication state
#[derive(Debug)]
pub struct SanctumManager {
    /// Database connection (optional, set via configure)
    db: Option<Arc<DatabaseConnection>>,
    /// Current access token (set during request)
    current_token: Option<PersonalAccessToken>,
    /// Current user ID
    current_user_id: Option<i64>,
    /// Current tokenable type
    current_tokenable_type: Option<String>,
}

impl SanctumManager {
    /// Create a new Sanctum manager
    pub fn new() -> Self {
        Self {
            db: None,
            current_token: None,
            current_user_id: None,
            current_tokenable_type: None,
        }
    }

    /// Set the database connection
    pub fn set_database(&mut self, db: Arc<DatabaseConnection>) {
        self.db = Some(db);
    }

    /// Get the database connection
    pub fn database(&self) -> Option<Arc<DatabaseConnection>> {
        self.db.clone()
    }

    /// Set the current access token (called by middleware)
    pub fn set_current_token(&mut self, token: PersonalAccessToken, user_id: i64, tokenable_type: String) {
        self.current_token = Some(token);
        self.current_user_id = Some(user_id);
        self.current_tokenable_type = Some(tokenable_type);
    }

    /// Get the current access token
    pub fn current_token(&self) -> Option<&PersonalAccessToken> {
        self.current_token.as_ref()
    }

    /// Get the current user ID
    pub fn current_user_id(&self) -> Option<i64> {
        self.current_user_id
    }

    /// Get the current tokenable type
    pub fn current_tokenable_type(&self) -> Option<&str> {
        self.current_tokenable_type.as_deref()
    }

    /// Clear the current authentication context
    pub fn clear_context(&mut self) {
        self.current_token = None;
        self.current_user_id = None;
        self.current_tokenable_type = None;
    }

    /// Check if a user is authenticated via Sanctum
    pub fn check(&self) -> bool {
        self.current_token.is_some()
    }

    /// Check if the current token has a specific ability
    pub fn token_can(&self, ability: &str) -> bool {
        if let Some(token) = &self.current_token {
            token.can(ability)
        } else {
            false
        }
    }

    /// Check if the current token has any of the abilities
    pub fn token_can_any(&self, abilities: &[&str]) -> bool {
        if let Some(token) = &self.current_token {
            token.can_any(abilities)
        } else {
            false
        }
    }

    /// Check if the current token has all abilities
    pub fn token_can_all(&self, abilities: &[&str]) -> bool {
        if let Some(token) = &self.current_token {
            token.can_all(abilities)
        } else {
            false
        }
    }
}

impl Default for SanctumManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_new() {
        let manager = SanctumManager::new();
        assert!(!manager.check());
        assert_eq!(manager.current_user_id(), None);
    }

    #[test]
    fn test_manager_check() {
        let manager = SanctumManager::new();
        assert!(!manager.check());
    }

    #[test]
    fn test_manager_clear_context() {
        let mut manager = SanctumManager::new();
        manager.clear_context();
        assert!(!manager.check());
    }
}
