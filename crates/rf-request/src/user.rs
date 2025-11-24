//! User types for authentication

use serde::{Deserialize, Serialize};

/// Represents an authenticated user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// User ID
    pub id: u64,
    /// User email
    pub email: String,
    /// User name (optional)
    pub name: Option<String>,
    /// Additional user attributes
    #[serde(flatten)]
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

impl User {
    /// Create a new user with ID and email
    pub fn new(id: u64, email: String) -> Self {
        Self {
            id,
            email,
            name: None,
            attributes: serde_json::Map::new(),
        }
    }

    /// Create a user with ID, email, and name
    pub fn with_name(id: u64, email: String, name: String) -> Self {
        Self {
            id,
            email,
            name: Some(name),
            attributes: serde_json::Map::new(),
        }
    }

    /// Set an attribute
    pub fn set_attribute(&mut self, key: String, value: serde_json::Value) {
        self.attributes.insert(key, value);
    }

    /// Get an attribute
    pub fn get_attribute(&self, key: &str) -> Option<&serde_json::Value> {
        self.attributes.get(key)
    }

    /// Check if user has a specific attribute
    pub fn has_attribute(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_user() {
        let user = User::new(1, "test@example.com".to_string());
        assert_eq!(user.id, 1);
        assert_eq!(user.email, "test@example.com");
        assert!(user.name.is_none());
    }

    #[test]
    fn test_user_with_name() {
        let user = User::with_name(1, "test@example.com".to_string(), "Test User".to_string());
        assert_eq!(user.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_user_attributes() {
        let mut user = User::new(1, "test@example.com".to_string());
        user.set_attribute("role".to_string(), serde_json::json!("admin"));

        assert!(user.has_attribute("role"));
        assert_eq!(
            user.get_attribute("role"),
            Some(&serde_json::json!("admin"))
        );
    }
}
