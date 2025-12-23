//! Laravel-style Config facade for RustForge

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::RwLock;

/// Global configuration storage
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_CONFIG: Lazy<RwLock<HashMap<String, String>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

pub struct Config;

impl Config {
    pub fn get(key: &str) -> Option<String> {
        let config = GLOBAL_CONFIG.read().unwrap();
        config.get(key).cloned()
    }

    pub fn get_or(key: &str, default: impl Into<String>) -> String {
        Self::get(key).unwrap_or_else(|| default.into())
    }

    pub fn set(key: impl Into<String>, value: impl Into<String>) {
        let mut config = GLOBAL_CONFIG.write().unwrap();
        config.insert(key.into(), value.into());
    }

    pub fn has(key: &str) -> bool {
        let config = GLOBAL_CONFIG.read().unwrap();
        config.contains_key(key)
    }

    pub fn all() -> HashMap<String, String> {
        let config = GLOBAL_CONFIG.read().unwrap();
        config.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_set_and_get() {
        Config::set("test_key", "test_value");
        let value = Config::get("test_key");
        assert_eq!(value, Some("test_value".to_string()));
    }

    #[test]
    fn test_config_get_or() {
        Config::set("existing", "value");
        assert_eq!(Config::get_or("existing", "default"), "value");
        assert_eq!(Config::get_or("non_existing", "default"), "default");
    }

    #[test]
    fn test_config_has() {
        Config::set("exists", "yes");
        assert!(Config::has("exists"));
        assert!(!Config::has("not_exists"));
    }

    #[test]
    fn test_config_all() {
        Config::set("key1", "value1");
        Config::set("key2", "value2");
        let all = Config::all();
        assert!(all.contains_key("key1"));
        assert!(all.contains_key("key2"));
    }
}
