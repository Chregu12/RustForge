//! OAuth Scope management

pub mod repository;

pub use repository::ScopeRepository;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// OAuth Scope definition
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Scope {
    /// Scope identifier (e.g., "read:posts")
    pub id: String,

    /// Human-readable description
    pub description: String,
}

impl Scope {
    /// Create a new scope
    pub fn new(id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            description: description.into(),
        }
    }
}

/// Scope checker for validating requested scopes
pub struct ScopeChecker {
    scopes: HashMap<String, Scope>,
}

impl ScopeChecker {
    /// Create a new scope checker
    pub fn new() -> Self {
        Self {
            scopes: HashMap::new(),
        }
    }

    /// Register a scope
    pub fn register(&mut self, scope: Scope) {
        self.scopes.insert(scope.id.clone(), scope);
    }

    /// Register multiple scopes
    pub fn register_many(&mut self, scopes: Vec<Scope>) {
        for scope in scopes {
            self.register(scope);
        }
    }

    /// Check if a scope exists
    pub fn exists(&self, scope_id: &str) -> bool {
        self.scopes.contains_key(scope_id)
    }

    /// Get a scope by ID
    pub fn get(&self, scope_id: &str) -> Option<&Scope> {
        self.scopes.get(scope_id)
    }

    /// Validate requested scopes
    pub fn validate(&self, requested: &[String]) -> Result<(), Vec<String>> {
        let invalid: Vec<String> = requested
            .iter()
            .filter(|s| !self.exists(s))
            .cloned()
            .collect();

        if invalid.is_empty() {
            Ok(())
        } else {
            Err(invalid)
        }
    }

    /// Get all scopes
    pub fn all(&self) -> Vec<&Scope> {
        self.scopes.values().collect()
    }

    /// Get scope count
    pub fn count(&self) -> usize {
        self.scopes.len()
    }
}

impl Default for ScopeChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_creation() {
        let scope = Scope::new("read:posts", "Read posts");
        assert_eq!(scope.id, "read:posts");
        assert_eq!(scope.description, "Read posts");
    }

    #[test]
    fn test_scope_checker() {
        let mut checker = ScopeChecker::new();
        checker.register(Scope::new("read:posts", "Read posts"));
        checker.register(Scope::new("write:posts", "Write posts"));

        assert!(checker.exists("read:posts"));
        assert!(checker.exists("write:posts"));
        assert!(!checker.exists("delete:posts"));

        assert_eq!(checker.count(), 2);
    }

    #[test]
    fn test_scope_validation() {
        let mut checker = ScopeChecker::new();
        checker.register(Scope::new("read:posts", "Read posts"));
        checker.register(Scope::new("write:posts", "Write posts"));

        // Valid scopes
        let valid = vec!["read:posts".to_string(), "write:posts".to_string()];
        assert!(checker.validate(&valid).is_ok());

        // Invalid scopes
        let invalid = vec!["read:posts".to_string(), "delete:posts".to_string()];
        let result = checker.validate(&invalid);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["delete:posts"]);
    }

    #[test]
    fn test_register_many() {
        let mut checker = ScopeChecker::new();
        let scopes = vec![
            Scope::new("read:posts", "Read posts"),
            Scope::new("write:posts", "Write posts"),
            Scope::new("delete:posts", "Delete posts"),
        ];

        checker.register_many(scopes);
        assert_eq!(checker.count(), 3);
    }
}
