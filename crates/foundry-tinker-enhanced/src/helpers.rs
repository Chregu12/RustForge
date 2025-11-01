//! Built-in helper functions for Tinker REPL

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::HashMap;
use std::env;

/// Built-in helper functions for Tinker
pub struct TinkerHelpers {
    cache: HashMap<String, Value>,
    config: HashMap<String, Value>,
}

impl TinkerHelpers {
    /// Create new helpers instance
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            config: HashMap::new(),
        }
    }

    /// Get current timestamp
    pub fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }

    /// Get environment variable with optional default
    pub fn env(&self, key: &str, default: Option<&str>) -> Result<String> {
        env::var(key)
            .or_else(|_| default.map(String::from).ok_or_else(|| anyhow!("Variable not found")))
    }

    /// Get configuration value with optional default
    pub fn config(&self, key: &str, default: Option<Value>) -> Value {
        self.config
            .get(key)
            .cloned()
            .or(default)
            .unwrap_or(Value::Null)
    }

    /// Set configuration value
    pub fn set_config(&mut self, key: String, value: Value) {
        self.config.insert(key, value);
    }

    /// Get value from cache
    pub fn cache_get(&self, key: &str) -> Option<&Value> {
        self.cache.get(key)
    }

    /// Put value in cache
    pub fn cache_put(&mut self, key: String, value: Value) {
        self.cache.insert(key, value);
    }

    /// Dump and die - pretty print value
    pub fn dd(&self, value: &Value) -> String {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| format!("{:?}", value))
    }

    /// Get all available helper names
    pub fn list_helpers() -> Vec<(&'static str, &'static str)> {
        vec![
            ("now()", "Get current timestamp"),
            ("env(key, default?)", "Get environment variable"),
            ("config(key, default?)", "Get configuration value"),
            ("cache_get(key)", "Get value from cache"),
            ("cache_put(key, value)", "Store value in cache"),
            ("db_query(sql)", "Execute raw SQL query"),
            ("dd(value)", "Dump and die - pretty print value"),
        ]
    }

    /// Format helpers list for display
    pub fn format_helpers() -> String {
        let mut output = String::from("\n Available Helpers:\n\n");
        for (name, description) in Self::list_helpers() {
            output.push_str(&format!("  {:30} - {}\n", name, description));
        }
        output
    }
}

impl Default for TinkerHelpers {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_now() {
        let helpers = TinkerHelpers::new();
        let now = helpers.now();
        assert!(now <= Utc::now());
    }

    #[test]
    fn test_env() {
        let helpers = TinkerHelpers::new();
        env::set_var("TEST_VAR", "test_value");
        let result = helpers.env("TEST_VAR", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "test_value");
    }

    #[test]
    fn test_env_with_default() {
        let helpers = TinkerHelpers::new();
        let result = helpers.env("NONEXISTENT_VAR", Some("default"));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "default");
    }

    #[test]
    fn test_config() {
        let mut helpers = TinkerHelpers::new();
        helpers.set_config("app.name".to_string(), Value::String("Foundry".to_string()));

        let result = helpers.config("app.name", None);
        assert_eq!(result, Value::String("Foundry".to_string()));
    }

    #[test]
    fn test_cache_operations() {
        let mut helpers = TinkerHelpers::new();
        helpers.cache_put("key1".to_string(), Value::Number(42.into()));

        let result = helpers.cache_get("key1");
        assert!(result.is_some());
        assert_eq!(*result.unwrap(), Value::Number(42.into()));
    }

    #[test]
    fn test_dd() {
        let helpers = TinkerHelpers::new();
        let value = serde_json::json!({"name": "test", "count": 42});
        let output = helpers.dd(&value);
        assert!(output.contains("name"));
        assert!(output.contains("test"));
    }

    #[test]
    fn test_list_helpers() {
        let helpers = TinkerHelpers::list_helpers();
        assert!(!helpers.is_empty());
        assert!(helpers.iter().any(|(name, _)| name.contains("now")));
    }

    #[test]
    fn test_format_helpers() {
        let output = TinkerHelpers::format_helpers();
        assert!(output.contains("now()"));
        assert!(output.contains("env("));
        assert!(output.contains("cache_"));
    }
}
