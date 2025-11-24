//! Session management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a user session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: String,
    /// Session data
    pub data: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session with the given ID
    pub fn new(id: String) -> Self {
        Self {
            id,
            data: HashMap::new(),
        }
    }

    /// Get a value from the session
    pub fn get<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a value in the session
    pub fn set<T: Serialize>(&mut self, key: String, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key, json_value);
        }
    }

    /// Check if a key exists in the session
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Remove a value from the session
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    /// Clear all session data
    pub fn clear(&mut self) {
        self.data.clear();
    }

    /// Flash a value (typically used for one-time messages)
    pub fn flash<T: Serialize>(&mut self, key: String, value: T) {
        self.set(format!("_flash_{}", key), value);
    }

    /// Get a flashed value and remove it
    pub fn get_flash<T: serde::de::DeserializeOwned>(&mut self, key: &str) -> Option<T> {
        let flash_key = format!("_flash_{}", key);
        let value = self.get(&flash_key);
        self.remove(&flash_key);
        value
    }

    /// Get all session data
    pub fn all(&self) -> &HashMap<String, serde_json::Value> {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_session() {
        let session = Session::new("session_123".to_string());
        assert_eq!(session.id, "session_123");
        assert!(session.data.is_empty());
    }

    #[test]
    fn test_session_set_get() {
        let mut session = Session::new("session_123".to_string());
        session.set("user_id".to_string(), 42u64);

        let user_id: u64 = session.get("user_id").unwrap();
        assert_eq!(user_id, 42);
    }

    #[test]
    fn test_session_has() {
        let mut session = Session::new("session_123".to_string());
        session.set("key".to_string(), "value");

        assert!(session.has("key"));
        assert!(!session.has("nonexistent"));
    }

    #[test]
    fn test_session_remove() {
        let mut session = Session::new("session_123".to_string());
        session.set("key".to_string(), "value");

        assert!(session.has("key"));
        session.remove("key");
        assert!(!session.has("key"));
    }

    #[test]
    fn test_session_flash() {
        let mut session = Session::new("session_123".to_string());
        session.flash("message".to_string(), "Hello, World!");

        let message: String = session.get_flash("message").unwrap();
        assert_eq!(message, "Hello, World!");

        // Flash should be removed after retrieval
        assert!(session.get_flash::<String>("message").is_none());
    }

    #[test]
    fn test_session_clear() {
        let mut session = Session::new("session_123".to_string());
        session.set("key1".to_string(), "value1");
        session.set("key2".to_string(), "value2");

        assert_eq!(session.data.len(), 2);
        session.clear();
        assert_eq!(session.data.len(), 0);
    }
}
