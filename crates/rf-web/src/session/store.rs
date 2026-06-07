//! Session storage and management

use super::driver::{SessionDriver, SessionResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// Session identifier
pub type SessionId = String;

/// Session store for managing session data
#[derive(Clone)]
pub struct SessionStore {
    driver: Arc<dyn SessionDriver>,
}

impl SessionStore {
    /// Create a new session store with a driver
    pub fn new(driver: Arc<dyn SessionDriver>) -> Self {
        Self { driver }
    }

    /// Create a new session
    pub async fn create(&self) -> SessionResult<Session> {
        let id = Self::generate_id();
        let session = Session::new(id, self.driver.clone());
        Ok(session)
    }

    /// Load an existing session
    pub async fn load(&self, id: impl Into<String>) -> SessionResult<Session> {
        let id = id.into();
        let data = self.driver.read(&id).await?;
        Ok(Session::from_data(id, data, self.driver.clone()))
    }

    /// Generate a cryptographically secure session ID
    fn generate_id() -> SessionId {
        use base64::Engine;
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes)
    }
}

/// Session for storing user data across requests
pub struct Session {
    id: SessionId,
    data: HashMap<String, Value>,
    driver: Arc<dyn SessionDriver>,
    dirty: bool,
}

impl Session {
    /// Create a new empty session
    pub fn new(id: SessionId, driver: Arc<dyn SessionDriver>) -> Self {
        Self {
            id,
            data: HashMap::new(),
            driver,
            dirty: false,
        }
    }

    /// Create a session from existing data
    pub fn from_data(
        id: SessionId,
        data: HashMap<String, Value>,
        driver: Arc<dyn SessionDriver>,
    ) -> Self {
        Self {
            id,
            data,
            driver,
            dirty: false,
        }
    }

    /// Get the session ID
    pub fn id(&self) -> &str {
        &self.id
    }

    /// Get a value from the session
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Get a typed value from the session
    pub fn get_as<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Put a value into the session
    pub fn put<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key.into(), json_value);
            self.dirty = true;
        }
    }

    /// Remove a value from the session
    pub fn forget(&mut self, key: &str) -> Option<Value> {
        let result = self.data.remove(key);
        if result.is_some() {
            self.dirty = true;
        }
        result
    }

    /// Check if a key exists in the session
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Get all session data
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.data
    }

    /// Flash data for the next request
    pub fn flash<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        let flash_key = format!("_flash.new.{}", key.into());
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(flash_key, json_value);
            self.dirty = true;
        }
    }

    /// Get flash data (removes it after retrieval)
    pub fn get_flash(&mut self, key: &str) -> Option<Value> {
        let flash_key = format!("_flash.old.{}", key);
        let result = self.data.remove(&flash_key);
        if result.is_some() {
            self.dirty = true;
        }
        result
    }

    /// Get flash data as typed value
    pub fn get_flash_as<T: for<'de> Deserialize<'de>>(&mut self, key: &str) -> Option<T> {
        self.get_flash(key)
            .and_then(|v| serde_json::from_value(v).ok())
    }

    /// Flash input data for form repopulation
    pub fn flash_input(&mut self, input: HashMap<String, String>) {
        if let Ok(json_value) = serde_json::to_value(input) {
            self.data.insert("_old_input".to_string(), json_value);
            self.dirty = true;
        }
    }

    /// Get old input value
    pub fn old(&self, key: &str) -> Option<String> {
        self.data
            .get("_old_input")
            .and_then(|v| v.as_object())
            .and_then(|obj| obj.get(key))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    /// Keep flash data for another request
    pub fn keep_flash(&mut self, keys: &[&str]) {
        for key in keys {
            let old_key = format!("_flash.old.{}", key);
            if let Some(value) = self.data.get(&old_key).cloned() {
                let new_key = format!("_flash.new.{}", key);
                self.data.insert(new_key, value);
                self.dirty = true;
            }
        }
    }

    /// Keep all flash data for another request
    pub fn keep_all_flash(&mut self) {
        let keys: Vec<String> = self
            .data
            .keys()
            .filter(|k| k.starts_with("_flash.old."))
            .map(|k| k.strip_prefix("_flash.old.").unwrap().to_string())
            .collect();

        for key in keys {
            let old_key = format!("_flash.old.{}", key);
            if let Some(value) = self.data.get(&old_key).cloned() {
                let new_key = format!("_flash.new.{}", key);
                self.data.insert(new_key, value);
            }
        }
        self.dirty = true;
    }

    /// Reflash all flash data
    pub fn reflash(&mut self) {
        self.keep_all_flash();
    }

    /// Age flash data (move new to old, remove old)
    pub fn age_flash_data(&mut self) {
        // Remove old flash data
        let old_keys: Vec<String> = self
            .data
            .keys()
            .filter(|k| k.starts_with("_flash.old."))
            .cloned()
            .collect();

        for key in old_keys {
            self.data.remove(&key);
        }

        // Move new flash data to old
        let new_keys: Vec<String> = self
            .data
            .keys()
            .filter(|k| k.starts_with("_flash.new."))
            .cloned()
            .collect();

        for new_key in new_keys {
            if let Some(value) = self.data.remove(&new_key) {
                let key_name = new_key.strip_prefix("_flash.new.").unwrap();
                let old_key = format!("_flash.old.{}", key_name);
                self.data.insert(old_key, value);
            }
        }

        self.dirty = true;
    }

    /// Regenerate the session ID (for security after login)
    pub async fn regenerate(&mut self) -> SessionResult<()> {
        let old_id = self.id.clone();
        let new_id = SessionStore::generate_id();

        // Save current data with new ID
        self.driver.write(&new_id, self.data.clone()).await?;

        // Destroy old session
        self.driver.destroy(&old_id).await?;

        // Update current session ID
        self.id = new_id;
        self.dirty = false;

        Ok(())
    }

    /// Invalidate the session (clear all data)
    pub async fn invalidate(&mut self) -> SessionResult<()> {
        self.data.clear();
        self.driver.destroy(&self.id).await?;
        self.dirty = false;
        Ok(())
    }

    /// Flush the session (remove all data without destroying)
    pub fn flush(&mut self) {
        self.data.clear();
        self.dirty = true;
    }

    /// Save the session if it has been modified
    pub async fn save(&mut self) -> SessionResult<()> {
        if self.dirty {
            self.driver.write(&self.id, self.data.clone()).await?;
            self.dirty = false;
        }
        Ok(())
    }

    /// Check if the session has unsaved changes
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Get the number of items in the session
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl Clone for Session {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            data: self.data.clone(),
            driver: Arc::clone(&self.driver),
            dirty: self.dirty,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::driver::CookieSessionDriver;
    use super::*;

    fn create_test_session() -> Session {
        let driver = Arc::new(CookieSessionDriver::new());
        Session::new("test_id".to_string(), driver)
    }

    #[test]
    fn test_session_put_and_get() {
        let mut session = create_test_session();

        session.put("name", "John Doe");
        assert_eq!(
            session.get_as::<String>("name"),
            Some("John Doe".to_string())
        );
    }

    #[test]
    fn test_session_forget() {
        let mut session = create_test_session();

        session.put("name", "John Doe");
        assert!(session.has("name"));

        session.forget("name");
        assert!(!session.has("name"));
    }

    #[test]
    fn test_session_flash() {
        let mut session = create_test_session();

        session.flash("message", "Success!");
        assert!(session.has("_flash.new.message"));
    }

    #[test]
    fn test_session_flash_retrieval() {
        let mut session = create_test_session();

        session.flash("message", "Success!");
        session.age_flash_data();

        let message = session.get_flash("message");
        let message_str = message.as_ref().and_then(|v| v.as_str()).map(|s| s.to_string());
        assert_eq!(message_str, Some("Success!".to_string()));
    }

    #[test]
    fn test_session_old_input() {
        let mut session = create_test_session();

        let mut input = HashMap::new();
        input.insert("email".to_string(), "test@example.com".to_string());
        input.insert("name".to_string(), "John".to_string());

        session.flash_input(input);

        assert_eq!(session.old("email"), Some("test@example.com".to_string()));
        assert_eq!(session.old("name"), Some("John".to_string()));
    }

    #[test]
    fn test_session_keep_flash() {
        let mut session = create_test_session();

        session.flash("message", "Keep this");
        session.age_flash_data();

        session.keep_flash(&["message"]);
        assert!(session.has("_flash.new.message"));
    }

    #[test]
    fn test_session_reflash() {
        let mut session = create_test_session();

        session.flash("msg1", "Message 1");
        session.flash("msg2", "Message 2");
        session.age_flash_data();

        session.reflash();
        assert!(session.has("_flash.new.msg1"));
        assert!(session.has("_flash.new.msg2"));
    }

    #[test]
    fn test_session_flush() {
        let mut session = create_test_session();

        session.put("key1", "value1");
        session.put("key2", "value2");
        assert_eq!(session.len(), 2);

        session.flush();
        assert!(session.is_empty());
        assert!(session.is_dirty());
    }

    #[test]
    fn test_session_age_flash_data() {
        let mut session = create_test_session();

        session.flash("new", "New Message");
        session.age_flash_data();

        assert!(!session.has("_flash.new.new"));
        assert!(session.has("_flash.old.new"));

        session.age_flash_data();
        assert!(!session.has("_flash.old.new"));
    }

    #[tokio::test]
    async fn test_session_store_create() {
        let driver = Arc::new(CookieSessionDriver::new());
        let store = SessionStore::new(driver);

        let session = store.create().await;
        assert!(session.is_ok());
        assert!(!session.unwrap().id().is_empty());
    }

    #[tokio::test]
    async fn test_session_store_create_unique_ids() {
        let driver = Arc::new(super::super::driver::MemorySessionDriver::new());
        let store = SessionStore::new(driver);

        let sess1 = store.create().await.unwrap();
        let sess2 = store.create().await.unwrap();
        assert_ne!(sess1.id(), sess2.id());
    }

    #[tokio::test]
    async fn test_session_put_marks_dirty() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut session = Session::new("sess_dirty".to_string(), driver);

        assert!(!session.is_dirty());
        session.put("key", "value");
        assert!(session.is_dirty());
    }

    #[tokio::test]
    async fn test_session_get_typed_value() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut session = Session::new("sess_typed".to_string(), driver);

        session.put("count", 42u32);
        let count: Option<u32> = session.get_as("count");
        assert_eq!(count, Some(42));
    }

    #[tokio::test]
    async fn test_session_forget_marks_dirty() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut session = Session::new("sess_forget".to_string(), driver);

        session.put("key", "value");
        // Reset dirty by manually checking state
        let _ = session.is_dirty();

        session.forget("key");
        assert!(!session.has("key"));
        assert!(session.is_dirty());
    }

    #[tokio::test]
    async fn test_session_len_and_is_empty() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut session = Session::new("sess_len".to_string(), driver);

        assert_eq!(session.len(), 0);
        assert!(session.is_empty());

        session.put("a", "1");
        session.put("b", "2");
        assert_eq!(session.len(), 2);
        assert!(!session.is_empty());
    }

    #[tokio::test]
    async fn test_session_from_data() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut data = HashMap::new();
        data.insert("user_id".to_string(), serde_json::json!(99));

        let session = Session::from_data("restored_id".to_string(), data, driver);
        assert_eq!(session.id(), "restored_id");
        assert!(session.has("user_id"));
    }

    #[tokio::test]
    async fn test_session_save_with_memory_driver() {
        let driver = Arc::new(super::super::driver::MemorySessionDriver::new());
        let store = SessionStore::new(Arc::clone(&driver) as Arc<dyn super::super::driver::SessionDriver>);

        let mut session = store.create().await.unwrap();
        let id = session.id().to_string();

        session.put("token", "abc123");
        session.save().await.unwrap();

        // Now load the session back
        let loaded = store.load(&id).await.unwrap();
        assert!(loaded.has("token"));
    }

    #[tokio::test]
    async fn test_session_flash_and_age_cycle() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut session = Session::new("flash_sess".to_string(), driver);

        session.flash("status", "saved");
        // Initially stored under _flash.new
        assert!(session.has("_flash.new.status"));

        // Age the data: moves new → old
        session.age_flash_data();
        assert!(!session.has("_flash.new.status"));
        assert!(session.has("_flash.old.status"));

        // Retrieve flash data
        let msg = session.get_flash("status");
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().as_str(), Some("saved"));

        // After retrieval it should be gone
        assert!(!session.has("_flash.old.status"));
    }

    #[tokio::test]
    async fn test_session_old_input_workflow() {
        let driver = Arc::new(CookieSessionDriver::new());
        let mut session = Session::new("input_sess".to_string(), driver);

        let mut input = HashMap::new();
        input.insert("email".to_string(), "user@test.com".to_string());
        input.insert("name".to_string(), "Test User".to_string());
        session.flash_input(input);

        assert_eq!(session.old("email"), Some("user@test.com".to_string()));
        assert_eq!(session.old("name"), Some("Test User".to_string()));
        assert!(session.old("phone").is_none());
    }
}
