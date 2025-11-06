//! OAuth2 Scope Management
//!
//! Defines and validates OAuth2 scopes

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OAuth2 Scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    /// Scope identifier (e.g., "users:read")
    pub id: String,

    /// Human-readable description
    pub description: String,

    /// Whether this scope is dangerous/privileged
    pub is_dangerous: bool,
}

impl Scope {
    pub fn new(id: String, description: String, is_dangerous: bool) -> Self {
        Self {
            id,
            description,
            is_dangerous,
        }
    }
}

/// Scope Manager
pub struct ScopeManager {
    scopes: HashMap<String, Scope>,
}

impl ScopeManager {
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
        }
    }

    /// Create with default scopes
    pub fn with_defaults() -> Self {
        let mut manager = Self::new();

        // Default scopes matching Laravel Passport style
        manager.register(Scope::new(
            "*".to_string(),
            "Full access to all resources".to_string(),
            true,
        ));

        manager.register(Scope::new(
            "users:read".to_string(),
            "Read user information".to_string(),
            false,
        ));

        manager.register(Scope::new(
            "users:write".to_string(),
            "Update user information".to_string(),
            false,
        ));

        manager.register(Scope::new(
            "users:delete".to_string(),
            "Delete user accounts".to_string(),
            true,
        ));

        manager.register(Scope::new(
            "api:read".to_string(),
            "Read API resources".to_string(),
            false,
        ));

        manager.register(Scope::new(
            "api:write".to_string(),
            "Create and update API resources".to_string(),
            false,
        ));

        manager.register(Scope::new(
            "admin".to_string(),
            "Administrative access".to_string(),
            true,
        ));

        manager
    }

    /// Register a new scope
    pub fn register(&mut self, scope: Scope) {
        self.scopes.insert(scope.id.clone(), scope);
    }

    /// Get scope by ID
    pub fn get(&self, id: &str) -> Option<&Scope> {
        self.scopes.get(id)
    }

    /// Check if scope exists
    pub fn exists(&self, id: &str) -> bool {
        self.scopes.contains_key(id)
    }

    /// Validate requested scopes
    pub fn validate(&self, requested_scopes: &[String]) -> Result<Vec<String>, String> {
        let mut validated = Vec::new();

        for scope_id in requested_scopes {
            // Wildcard scope grants all scopes
            if scope_id == "*" {
                if self.exists("*") {
                    return Ok(vec!["*".to_string()]);
                } else {
                    return Err("Wildcard scope not allowed".to_string());
                }
            }

            if !self.exists(scope_id) {
                return Err(format!("Invalid scope: {}", scope_id));
            }

            validated.push(scope_id.clone());
        }

        Ok(validated)
    }

    /// Check if granted scopes satisfy required scopes
    pub fn satisfies(&self, granted: &[String], required: &[String]) -> bool {
        // Wildcard grants everything
        if granted.contains(&"*".to_string()) {
            return true;
        }

        // Check each required scope is granted
        for req in required {
            if !granted.contains(req) {
                return false;
            }
        }

        true
    }

    /// Get all scopes
    pub fn all(&self) -> Vec<&Scope> {
        self.scopes.values().collect()
    }

    /// Get all dangerous scopes
    pub fn dangerous(&self) -> Vec<&Scope> {
        self.scopes
            .values()
            .filter(|s| s.is_dangerous)
            .collect()
    }

    /// Filter scopes by pattern
    pub fn filter(&self, pattern: &str) -> Vec<&Scope> {
        self.scopes
            .values()
            .filter(|s| s.id.starts_with(pattern))
            .collect()
    }
}

impl Default for ScopeManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_manager_defaults() {
        let manager = ScopeManager::with_defaults();

        assert!(manager.exists("*"));
        assert!(manager.exists("users:read"));
        assert!(manager.exists("users:write"));
        assert!(manager.exists("admin"));
    }

    #[test]
    fn test_validate_scopes() {
        let manager = ScopeManager::with_defaults();

        let valid = vec!["users:read".to_string(), "users:write".to_string()];
        assert!(manager.validate(&valid).is_ok());

        let invalid = vec!["users:read".to_string(), "invalid:scope".to_string()];
        assert!(manager.validate(&invalid).is_err());
    }

    #[test]
    fn test_wildcard_scope() {
        let manager = ScopeManager::with_defaults();

        let wildcard = vec!["*".to_string()];
        let validated = manager.validate(&wildcard).unwrap();

        assert_eq!(validated, vec!["*".to_string()]);
    }

    #[test]
    fn test_satisfies() {
        let manager = ScopeManager::with_defaults();

        // Wildcard satisfies everything
        assert!(manager.satisfies(&["*".to_string()], &["users:read".to_string()]));

        // Exact match
        assert!(manager.satisfies(&["users:read".to_string()], &["users:read".to_string()]));

        // Missing scope
        assert!(!manager.satisfies(&["users:read".to_string()], &["users:write".to_string()]));

        // Subset
        assert!(manager.satisfies(
            &["users:read".to_string(), "users:write".to_string()],
            &["users:read".to_string()]
        ));
    }

    #[test]
    fn test_dangerous_scopes() {
        let manager = ScopeManager::with_defaults();

        let dangerous = manager.dangerous();
        let dangerous_ids: Vec<&str> = dangerous.iter().map(|s| s.id.as_str()).collect();

        assert!(dangerous_ids.contains(&"*"));
        assert!(dangerous_ids.contains(&"users:delete"));
        assert!(dangerous_ids.contains(&"admin"));
    }

    #[test]
    fn test_filter_scopes() {
        let manager = ScopeManager::with_defaults();

        let user_scopes = manager.filter("users:");
        assert_eq!(user_scopes.len(), 3); // users:read, users:write, users:delete

        let api_scopes = manager.filter("api:");
        assert_eq!(api_scopes.len(), 2); // api:read, api:write
    }

    #[test]
    fn test_custom_scope() {
        let mut manager = ScopeManager::new();

        manager.register(Scope::new(
            "custom:action".to_string(),
            "Custom action".to_string(),
            false,
        ));

        assert!(manager.exists("custom:action"));
        assert_eq!(manager.get("custom:action").unwrap().description, "Custom action");
    }
}
