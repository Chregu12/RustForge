//! Laravel-style Config facade for RustForge
//!
//! Provides both a type-safe strongly-typed API and a convenient string-based
//! API for configuration access.  All lock operations use `expect` with
//! descriptive messages instead of bare `unwrap()`.

use once_cell::sync::Lazy;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::types::AppConfig;

/// Global configuration storage using JSON values for type-safe retrieval.
///
/// Prefer using [`Config::typed`] for strongly-typed access when an
/// [`AppConfig`] has been loaded via [`Config::init`].
pub static GLOBAL_CONFIG: Lazy<RwLock<HashMap<String, Value>>> = Lazy::new(|| {
    RwLock::new(HashMap::new())
});

/// Typed application configuration (loaded once at startup).
static TYPED_CONFIG: Lazy<RwLock<Option<AppConfig>>> = Lazy::new(|| RwLock::new(None));

pub struct Config;

impl Config {
    // ------------------------------------------------------------------
    // Typed configuration API
    // ------------------------------------------------------------------

    /// Initialise the typed configuration from an [`AppConfig`].
    ///
    /// This should be called once at application startup.  After calling
    /// `init`, all key/value pairs are also available through the
    /// string-based API (via [`Config::get`]).
    pub fn init(app_config: AppConfig) {
        // Store the typed config
        {
            let mut typed = TYPED_CONFIG
                .write()
                .expect("TYPED_CONFIG lock poisoned");
            *typed = Some(app_config.clone());
        }

        // Mirror into the flat key/value store for backwards-compat
        Config::set("server.host", &app_config.server.host);
        Config::set_value("server.port", Value::Number(app_config.server.port.into()));
        Config::set_value(
            "server.workers",
            serde_json::to_value(app_config.server.workers).unwrap_or(Value::Null),
        );
        Config::set_value(
            "server.timeout",
            serde_json::to_value(app_config.server.timeout).unwrap_or(Value::Null),
        );
        Config::set("database.url", &app_config.database.url);
        Config::set_value(
            "database.max_connections",
            Value::Number(app_config.database.max_connections.into()),
        );
        Config::set("auth.jwt_secret", &app_config.auth.jwt_secret);
        Config::set_value(
            "auth.token_expiry_hours",
            serde_json::to_value(app_config.auth.token_expiry_hours).unwrap_or(Value::Null),
        );
    }

    /// Get the typed application configuration (requires prior [`Config::init`]).
    pub fn typed() -> Option<AppConfig> {
        let typed = TYPED_CONFIG
            .read()
            .expect("TYPED_CONFIG lock poisoned");
        typed.clone()
    }

    // ------------------------------------------------------------------
    // String-based convenience API (backed by serde_json::Value)
    // ------------------------------------------------------------------

    /// Get a string configuration value.
    pub fn get(key: &str) -> Option<String> {
        let config = GLOBAL_CONFIG
            .read()
            .expect("GLOBAL_CONFIG lock poisoned");
        config.get(key).and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            other => Some(other.to_string()),
        })
    }

    /// Get a string value with a default fallback.
    pub fn get_or(key: &str, default: impl Into<String>) -> String {
        Self::get(key).unwrap_or_else(|| default.into())
    }

    /// Get a typed value by deserializing the stored JSON value.
    pub fn get_value<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
        let config = GLOBAL_CONFIG
            .read()
            .expect("GLOBAL_CONFIG lock poisoned");
        config
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a typed value with a default fallback.
    pub fn get_value_or<T: serde::de::DeserializeOwned>(key: &str, default: T) -> T {
        Self::get_value(key).unwrap_or(default)
    }

    /// Set a string configuration value.
    pub fn set(key: impl Into<String>, value: impl Into<String>) {
        let mut config = GLOBAL_CONFIG
            .write()
            .expect("GLOBAL_CONFIG lock poisoned");
        config.insert(key.into(), Value::String(value.into()));
    }

    /// Set a JSON value directly.
    pub fn set_value(key: impl Into<String>, value: Value) {
        let mut config = GLOBAL_CONFIG
            .write()
            .expect("GLOBAL_CONFIG lock poisoned");
        config.insert(key.into(), value);
    }

    /// Check if a key exists.
    pub fn has(key: &str) -> bool {
        let config = GLOBAL_CONFIG
            .read()
            .expect("GLOBAL_CONFIG lock poisoned");
        config.contains_key(key)
    }

    /// Get all configuration as a flat map.
    pub fn all() -> HashMap<String, Value> {
        let config = GLOBAL_CONFIG
            .read()
            .expect("GLOBAL_CONFIG lock poisoned");
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
        assert_eq!(Config::get_or("non_existing_xyz", "default"), "default");
    }

    #[test]
    fn test_config_has() {
        Config::set("exists_check", "yes");
        assert!(Config::has("exists_check"));
        assert!(!Config::has("not_exists_check"));
    }

    #[test]
    fn test_config_all() {
        Config::set("all_key1", "value1");
        Config::set("all_key2", "value2");
        let all = Config::all();
        assert!(all.contains_key("all_key1"));
        assert!(all.contains_key("all_key2"));
    }

    #[test]
    fn test_typed_value_get() {
        Config::set_value("typed_port", Value::Number(8080.into()));
        let port: Option<u16> = Config::get_value("typed_port");
        assert_eq!(port, Some(8080));
    }

    #[test]
    fn test_typed_value_or() {
        let workers: usize = Config::get_value_or("missing_workers", 4);
        assert_eq!(workers, 4);
    }

    #[test]
    fn test_init_typed_config() {
        use crate::types::{AppConfig, AuthConfig, DatabaseConfig, ServerConfig};

        let app_config = AppConfig {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 9000,
                workers: 8,
                timeout: 60,
            },
            database: DatabaseConfig {
                url: "postgres://localhost/test".to_string(),
                max_connections: 20,
                ..Default::default()
            },
            auth: AuthConfig::default(),
        };

        Config::init(app_config.clone());

        let typed = Config::typed().unwrap();
        assert_eq!(typed.server.port, 9000);
        assert_eq!(typed.server.workers, 8);

        // Also available through flat API
        let port: Option<u16> = Config::get_value("server.port");
        assert_eq!(port, Some(9000));
    }
}
