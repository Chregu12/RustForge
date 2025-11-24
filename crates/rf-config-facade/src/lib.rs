//! # rf-config-facade
//!
//! Laravel-style Config facade for RustForge

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub static GLOBAL_CONFIG: Lazy<Arc<RwLock<HashMap<String, String>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(HashMap::new()))
});

pub struct Config;

impl Config {
    pub async fn get(key: &str) -> Option<String> {
        let config = GLOBAL_CONFIG.read().await;
        config.get(key).cloned()
    }

    pub async fn get_or(key: &str, default: impl Into<String>) -> String {
        Self::get(key).await.unwrap_or_else(|| default.into())
    }

    pub async fn set(key: impl Into<String>, value: impl Into<String>) {
        let mut config = GLOBAL_CONFIG.write().await;
        config.insert(key.into(), value.into());
    }

    pub async fn has(key: &str) -> bool {
        let config = GLOBAL_CONFIG.read().await;
        config.contains_key(key)
    }

    pub async fn all() -> HashMap<String, String> {
        let config = GLOBAL_CONFIG.read().await;
        config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_config_set_and_get() {
        Config::set("test_key", "test_value").await;
        let value = Config::get("test_key").await;
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_config_get_or() {
        Config::set("existing", "value").await;
        assert_eq!(Config::get_or("existing", "default").await, "value");
        assert_eq!(Config::get_or("non_existing", "default").await, "default");
    }

    #[tokio::test]
    async fn test_config_has() {
        Config::set("exists", "yes").await;
        assert!(Config::has("exists").await);
        assert!(!Config::has("not_exists").await);
    }

    #[tokio::test]
    async fn test_config_all() {
        Config::set("key1", "value1").await;
        Config::set("key2", "value2").await;
        let all = Config::all().await;
        assert!(all.contains_key("key1"));
        assert!(all.contains_key("key2"));
    }
}
