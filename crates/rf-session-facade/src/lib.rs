//! # rf-session-facade
//!
//! Laravel-style Session facade for RustForge

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub static GLOBAL_SESSION: Lazy<Arc<RwLock<HashMap<String, Value>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

pub struct Session;

impl Session {
    pub async fn get(key: &str) -> Option<Value> {
        let session = GLOBAL_SESSION.read().await;
        session.get(key).cloned()
    }

    pub async fn put(key: impl Into<String>, value: Value) {
        let mut session = GLOBAL_SESSION.write().await;
        session.insert(key.into(), value);
    }

    pub async fn has(key: &str) -> bool {
        let session = GLOBAL_SESSION.read().await;
        session.contains_key(key)
    }

    pub async fn forget(key: &str) {
        let mut session = GLOBAL_SESSION.write().await;
        session.remove(key);
    }

    pub async fn flush() {
        let mut session = GLOBAL_SESSION.write().await;
        session.clear();
    }

    pub async fn flash(key: impl Into<String>, value: Value) {
        // Simplified flash implementation
        Self::put(key, value).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_session_put_and_get() {
        Session::put("test_key", json!("test_value")).await;
        let value = Session::get("test_key").await;
        assert_eq!(value, Some(json!("test_value")));
    }

    #[tokio::test]
    async fn test_session_has() {
        Session::put("exists", json!(true)).await;
        assert!(Session::has("exists").await);
        assert!(!Session::has("not_exists").await);
    }

    #[tokio::test]
    async fn test_session_forget() {
        Session::put("to_forget", json!("value")).await;
        Session::forget("to_forget").await;
        assert!(!Session::has("to_forget").await);
    }

    #[tokio::test]
    async fn test_session_flush() {
        Session::put("key1", json!("value1")).await;
        Session::put("key2", json!("value2")).await;
        Session::flush().await;
        assert!(!Session::has("key1").await);
        assert!(!Session::has("key2").await);
    }

    #[tokio::test]
    async fn test_session_flash() {
        Session::flash("flash_key", json!("flash_value")).await;
        assert!(Session::has("flash_key").await);
    }
}
