//! Global Passport manager for facade pattern

use once_cell::sync::Lazy;
use crate::{PassportConfig, Scope};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Global Passport manager instance
pub static GLOBAL_PASSPORT: Lazy<Arc<RwLock<PassportManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(PassportManager::new()))
});

/// Passport manager that holds the global OAuth2 configuration and state
#[derive(Debug)]
pub struct PassportManager {
    /// Database connection (optional, set via configure)
    db: Option<Arc<DatabaseConnection>>,
    /// Passport configuration
    config: PassportConfig,
    /// Registered scopes
    scopes: HashMap<String, Scope>,
    /// Default scopes
    default_scopes: Vec<String>,
    /// Current access token (set during request)
    current_token_id: Option<String>,
    /// Current user ID
    current_user_id: Option<i64>,
}

impl PassportManager {
    /// Create a new Passport manager
    pub fn new() -> Self {
        Self {
            db: None,
            config: PassportConfig::default(),
            scopes: HashMap::new(),
            default_scopes: Vec::new(),
            current_token_id: None,
            current_user_id: None,
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

    /// Get the configuration
    pub fn config(&self) -> &PassportConfig {
        &self.config
    }

    /// Get mutable configuration
    pub fn config_mut(&mut self) -> &mut PassportConfig {
        &mut self.config
    }

    /// Register a scope
    pub fn register_scope(&mut self, scope: Scope) {
        self.scopes.insert(scope.id.clone(), scope);
    }

    /// Register multiple scopes
    pub fn register_scopes(&mut self, scopes: Vec<Scope>) {
        for scope in scopes {
            self.register_scope(scope);
        }
    }

    /// Check if a scope exists
    pub fn has_scope(&self, scope_id: &str) -> bool {
        self.scopes.contains_key(scope_id)
    }

    /// Get a scope by ID
    pub fn get_scope(&self, scope_id: &str) -> Option<&Scope> {
        self.scopes.get(scope_id)
    }

    /// Get all scopes
    pub fn all_scopes(&self) -> Vec<&Scope> {
        self.scopes.values().collect()
    }

    /// Set default scopes
    pub fn set_default_scopes(&mut self, scopes: Vec<String>) {
        self.default_scopes = scopes;
    }

    /// Get default scopes
    pub fn default_scopes(&self) -> &[String] {
        &self.default_scopes
    }

    /// Set the current access token (called by middleware)
    pub fn set_current_token(&mut self, token_id: String, user_id: i64) {
        self.current_token_id = Some(token_id);
        self.current_user_id = Some(user_id);
    }

    /// Get the current token ID
    pub fn current_token_id(&self) -> Option<&str> {
        self.current_token_id.as_deref()
    }

    /// Get the current user ID
    pub fn current_user_id(&self) -> Option<i64> {
        self.current_user_id
    }

    /// Clear the current authentication context
    pub fn clear_context(&mut self) {
        self.current_token_id = None;
        self.current_user_id = None;
    }

    /// Check if a user is authenticated via Passport
    pub fn check(&self) -> bool {
        self.current_token_id.is_some()
    }
}

impl Default for PassportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_new() {
        let manager = PassportManager::new();
        assert!(!manager.check());
        assert_eq!(manager.current_user_id(), None);
    }

    #[test]
    fn test_manager_scopes() {
        let mut manager = PassportManager::new();
        let scope = Scope::new("read:posts", "Read posts");

        manager.register_scope(scope.clone());
        assert!(manager.has_scope("read:posts"));
        assert!(!manager.has_scope("write:posts"));
        assert_eq!(manager.all_scopes().len(), 1);
    }

    #[test]
    fn test_manager_default_scopes() {
        let mut manager = PassportManager::new();

        manager.set_default_scopes(vec!["read:posts".to_string()]);
        assert_eq!(manager.default_scopes(), &["read:posts"]);
    }

    #[test]
    fn test_manager_clear_context() {
        let mut manager = PassportManager::new();
        manager.clear_context();
        assert!(!manager.check());
    }
}
