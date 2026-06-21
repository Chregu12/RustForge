//! OAuth2 Scope Management
//!
//! Advanced scope checking and validation

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// OAuth2 Scope
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Scope(pub String);

impl Scope {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Check if this scope matches a pattern
    pub fn matches(&self, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Support wildcard patterns like "posts:*"
        if pattern.ends_with("*") {
            let prefix = pattern.trim_end_matches('*');
            return self.0.starts_with(prefix);
        }

        self.0 == pattern
    }

    /// Parse scopes from space-separated string (RFC 6749)
    pub fn parse_scopes(scope_string: &str) -> Vec<Scope> {
        scope_string
            .split_whitespace()
            .map(Scope::new)
            .collect()
    }

    /// Join scopes into space-separated string
    pub fn join_scopes(scopes: &[Scope]) -> String {
        scopes
            .iter()
            .map(|s| s.0.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Scope checker for validating token scopes
pub trait ScopeChecker {
    /// Check if has specific scope
    fn has_scope(&self, scope: &str) -> bool;

    /// Check if has any of the scopes
    fn has_any_scope(&self, scopes: &[&str]) -> bool {
        scopes.iter().any(|&s| self.has_scope(s))
    }

    /// Check if has all scopes
    fn has_all_scopes(&self, scopes: &[&str]) -> bool {
        scopes.iter().all(|&s| self.has_scope(s))
    }
}

/// Scope set with validation
#[derive(Debug, Clone)]
pub struct ScopeSet {
    scopes: HashSet<Scope>,
}

impl ScopeSet {
    pub fn new() -> Self {
        Self {
            scopes: HashSet::new(),
        }
    }

    pub fn from_vec(scopes: Vec<String>) -> Self {
        Self {
            scopes: scopes.into_iter().map(Scope::new).collect(),
        }
    }

    pub fn from_string(scope_string: &str) -> Self {
        Self {
            scopes: Scope::parse_scopes(scope_string).into_iter().collect(),
        }
    }

    pub fn add(&mut self, scope: Scope) {
        self.scopes.insert(scope);
    }

    pub fn contains(&self, scope: &Scope) -> bool {
        self.scopes.contains(scope)
    }

    pub fn contains_pattern(&self, pattern: &str) -> bool {
        self.scopes.iter().any(|s| s.matches(pattern))
    }

    pub fn is_subset_of(&self, other: &ScopeSet) -> bool {
        self.scopes.iter().all(|s| other.contains(s))
    }

    // Intentional inherent `to_string` for the public scope-set API; keep as-is.
    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let scopes: Vec<_> = self.scopes.iter().cloned().collect();
        Scope::join_scopes(&scopes)
    }

    pub fn to_vec(&self) -> Vec<String> {
        self.scopes.iter().map(|s| s.0.clone()).collect()
    }
}

impl Default for ScopeSet {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeChecker for ScopeSet {
    fn has_scope(&self, scope: &str) -> bool {
        self.contains_pattern(scope)
    }
}

/// Scope validator for checking requested vs allowed scopes
pub struct ScopeValidator {
    allowed_scopes: ScopeSet,
}

impl ScopeValidator {
    pub fn new(allowed_scopes: Vec<String>) -> Self {
        Self {
            allowed_scopes: ScopeSet::from_vec(allowed_scopes),
        }
    }

    /// Validate that requested scopes are allowed
    pub fn validate(&self, requested: &ScopeSet) -> Result<(), ScopeError> {
        if !requested.is_subset_of(&self.allowed_scopes) {
            return Err(ScopeError::InvalidScope);
        }
        Ok(())
    }

    /// Filter requested scopes to only those that are allowed
    pub fn filter(&self, requested: &ScopeSet) -> ScopeSet {
        let mut filtered = ScopeSet::new();
        for scope in &requested.scopes {
            if self.allowed_scopes.contains(scope) {
                filtered.add(scope.clone());
            }
        }
        filtered
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ScopeError {
    #[error("Invalid scope requested")]
    InvalidScope,

    #[error("Insufficient scopes")]
    InsufficientScopes,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scope_parsing() {
        let scopes = Scope::parse_scopes("read write admin");
        assert_eq!(scopes.len(), 3);
        assert_eq!(scopes[0].0, "read");
        assert_eq!(scopes[1].0, "write");
        assert_eq!(scopes[2].0, "admin");
    }

    #[test]
    fn test_scope_joining() {
        let scopes = vec![Scope::new("read"), Scope::new("write"), Scope::new("admin")];
        let joined = Scope::join_scopes(&scopes);
        assert_eq!(joined, "read write admin");
    }

    #[test]
    fn test_scope_matches() {
        let scope = Scope::new("posts:read");

        assert!(scope.matches("posts:read"));
        assert!(!scope.matches("posts:write"));
        assert!(scope.matches("posts:*"));
        assert!(scope.matches("*"));
    }

    #[test]
    fn test_scope_set() {
        let mut set = ScopeSet::new();
        set.add(Scope::new("read"));
        set.add(Scope::new("write"));

        assert!(set.has_scope("read"));
        assert!(set.has_scope("write"));
        assert!(!set.has_scope("admin"));
    }

    #[test]
    fn test_scope_set_from_string() {
        let set = ScopeSet::from_string("read write admin");

        assert!(set.has_scope("read"));
        assert!(set.has_scope("write"));
        assert!(set.has_scope("admin"));
    }

    #[test]
    fn test_scope_validation() {
        let validator = ScopeValidator::new(vec![
            "read".to_string(),
            "write".to_string(),
            "admin".to_string(),
        ]);

        let valid_scopes = ScopeSet::from_string("read write");
        assert!(validator.validate(&valid_scopes).is_ok());

        let invalid_scopes = ScopeSet::from_string("read write super_admin");
        assert!(validator.validate(&invalid_scopes).is_err());
    }

    #[test]
    fn test_scope_filtering() {
        let validator = ScopeValidator::new(vec!["read".to_string(), "write".to_string()]);

        let requested = ScopeSet::from_string("read write admin");
        let filtered = validator.filter(&requested);

        assert!(filtered.has_scope("read"));
        assert!(filtered.has_scope("write"));
        assert!(!filtered.has_scope("admin"));
    }

    #[test]
    fn test_scope_subset() {
        let set1 = ScopeSet::from_string("read write");
        let set2 = ScopeSet::from_string("read write admin");

        assert!(set1.is_subset_of(&set2));
        assert!(!set2.is_subset_of(&set1));
    }
}
