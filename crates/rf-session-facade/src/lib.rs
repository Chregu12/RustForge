//! # rf-session-facade
//!
//! Laravel-style Session facade for RustForge

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global session storage
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_SESSION: Lazy<RwLock<HashMap<String, Value>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

pub struct Session;

impl Session {
    pub fn get(key: &str) -> Option<Value> {
        let session = GLOBAL_SESSION.read().unwrap();
        session.get(key).cloned()
    }

    pub fn put(key: impl Into<String>, value: Value) {
        let mut session = GLOBAL_SESSION.write().unwrap();
        session.insert(key.into(), value);
    }

    pub fn has(key: &str) -> bool {
        let session = GLOBAL_SESSION.read().unwrap();
        session.contains_key(key)
    }

    pub fn forget(key: &str) {
        let mut session = GLOBAL_SESSION.write().unwrap();
        session.remove(key);
    }

    /// Alias for [`forget`] — naming-consistency convenience.
    ///
    /// [`forget`]: Session::forget
    pub fn delete(key: &str) {
        Self::forget(key)
    }

    pub fn flush() {
        let mut session = GLOBAL_SESSION.write().unwrap();
        session.clear();
    }

    pub fn flash(key: impl Into<String>, value: Value) {
        // Simplified flash implementation
        Self::put(key, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_put_and_get() {
        Session::put("test_key", json!("test_value"));
        let value = Session::get("test_key");
        assert_eq!(value, Some(json!("test_value")));
    }

    #[test]
    fn test_session_has() {
        Session::put("exists", json!(true));
        assert!(Session::has("exists"));
        assert!(!Session::has("not_exists"));
    }

    #[test]
    fn test_session_forget() {
        Session::put("to_forget", json!("value"));
        Session::forget("to_forget");
        assert!(!Session::has("to_forget"));
    }

    #[test]
    fn test_session_flush() {
        Session::put("key1", json!("value1"));
        Session::put("key2", json!("value2"));
        Session::flush();
        assert!(!Session::has("key1"));
        assert!(!Session::has("key2"));
    }

    #[test]
    fn test_session_flash() {
        Session::flash("flash_key", json!("flash_value"));
        assert!(Session::has("flash_key"));
    }
}
