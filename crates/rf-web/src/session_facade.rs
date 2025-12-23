//! Laravel-style Session facade for RustForge
//!
//! This provides a simple global session store for convenience.
//! For request-scoped sessions, use the `Session` from the session module.

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global session storage
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_SESSION: Lazy<RwLock<HashMap<String, Value>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

/// The SessionFacade providing a static API for global session storage.
///
/// # Note
///
/// This is a simple global key-value store. For proper request-scoped
/// sessions with drivers, use `Session` from the session module.
///
/// # Examples
///
/// ```rust
/// use rf_web::SessionFacade;
/// use serde_json::json;
///
/// SessionFacade::put("user_id", json!(123));
/// if let Some(user_id) = SessionFacade::get("user_id") {
///     println!("User ID: {}", user_id);
/// }
/// ```
pub struct SessionFacade;

impl SessionFacade {
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
        SessionFacade::put("test_key", json!("test_value"));
        let value = SessionFacade::get("test_key");
        assert_eq!(value, Some(json!("test_value")));
    }

    #[test]
    fn test_session_has() {
        SessionFacade::put("exists", json!(true));
        assert!(SessionFacade::has("exists"));
    }

    #[test]
    fn test_session_forget() {
        SessionFacade::put("to_forget", json!("value"));
        SessionFacade::forget("to_forget");
        assert!(!SessionFacade::has("to_forget"));
    }

    #[test]
    fn test_session_flush() {
        SessionFacade::put("key1", json!("value1"));
        SessionFacade::put("key2", json!("value2"));
        SessionFacade::flush();
        assert!(!SessionFacade::has("key1"));
        assert!(!SessionFacade::has("key2"));
    }

    #[test]
    fn test_session_flash() {
        SessionFacade::flash("flash_key", json!("flash_value"));
        assert!(SessionFacade::has("flash_key"));
    }
}
