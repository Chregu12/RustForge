//! Token abilities and permission checking

use serde::{Deserialize, Serialize};

/// Represents a token ability/scope
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Ability(pub String);

impl Ability {
    /// Create a new ability
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    /// Wildcard ability (all permissions)
    pub fn wildcard() -> Self {
        Self("*".to_string())
    }

    /// Check if ability matches a pattern
    pub fn matches(&self, pattern: &str) -> bool {
        if self.0 == "*" {
            return true;
        }
        self.0 == pattern
    }
}

/// Helper trait for checking abilities
pub trait AbilityChecker {
    fn can(&self, ability: &str) -> bool;
    fn can_any(&self, abilities: &[&str]) -> bool;
    fn can_all(&self, abilities: &[&str]) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ability() {
        let ability = Ability::new("read:posts");
        assert!(ability.matches("read:posts"));
        assert!(!ability.matches("write:posts"));

        let wildcard = Ability::wildcard();
        assert!(wildcard.matches("anything"));
    }
}
