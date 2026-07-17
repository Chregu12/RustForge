//! Props management for Inertia.js
//!
//! Handles regular props, shared props, and lazy-loaded props.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Props container for Inertia responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Props {
    #[serde(flatten)]
    data: HashMap<String, Value>,
}

impl Props {
    /// Create a new empty Props container
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Add a prop
    pub fn with<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key.into(), json_value);
        }
        self
    }

    /// Get a prop value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Get all props
    pub fn all(&self) -> &HashMap<String, Value> {
        &self.data
    }

    /// Merge with another Props
    pub fn merge(mut self, other: Props) -> Self {
        self.data.extend(other.data);
        self
    }

    /// Check if a prop exists
    pub fn has(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Remove a prop
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }
}

impl Default for Props {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashMap<String, Value>> for Props {
    fn from(data: HashMap<String, Value>) -> Self {
        Self { data }
    }
}

/// Shared props that are included in all Inertia responses
#[derive(Clone)]
pub struct SharedProps {
    props: Arc<RwLock<HashMap<String, Value>>>,
}

impl SharedProps {
    /// Create a new SharedProps container
    pub fn new() -> Self {
        Self {
            props: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a shared prop
    pub async fn add<T: Serialize>(&self, key: impl Into<String>, value: T) {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.props.write().await.insert(key.into(), json_value);
        }
    }

    /// Get all shared props as a Props instance
    pub async fn all(&self) -> Props {
        let data = self.props.read().await.clone();
        Props { data }
    }

    /// Remove a shared prop
    pub async fn remove(&self, key: &str) -> Option<Value> {
        self.props.write().await.remove(key)
    }

    /// Clear all shared props
    pub async fn clear(&self) {
        self.props.write().await.clear();
    }
}

impl Default for SharedProps {
    fn default() -> Self {
        Self::new()
    }
}

/// Lazy prop - evaluated only when requested
pub struct LazyProp {
    key: String,
    evaluator: Box<dyn Fn() -> Value + Send + Sync>,
}

impl LazyProp {
    /// Create a new lazy prop
    pub fn new<F>(key: impl Into<String>, evaluator: F) -> Self
    where
        F: Fn() -> Value + Send + Sync + 'static,
    {
        Self {
            key: key.into(),
            evaluator: Box::new(evaluator),
        }
    }

    /// Get the key
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Evaluate the lazy prop
    pub fn evaluate(&self) -> Value {
        (self.evaluator)()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_props_creation() {
        let props = Props::new()
            .with("name", "John")
            .with("age", 30)
            .with("active", true);

        assert_eq!(props.get("name"), Some(&json!("John")));
        assert_eq!(props.get("age"), Some(&json!(30)));
        assert_eq!(props.get("active"), Some(&json!(true)));
    }

    #[test]
    fn test_props_merge() {
        let props1 = Props::new().with("name", "John");
        let props2 = Props::new().with("age", 30);

        let merged = props1.merge(props2);

        assert!(merged.has("name"));
        assert!(merged.has("age"));
    }

    #[tokio::test]
    async fn test_shared_props() {
        let shared = SharedProps::new();
        shared.add("app_name", "RustForge").await;
        shared.add("version", "1.0.0").await;

        let all = shared.all().await;
        assert!(all.has("app_name"));
        assert!(all.has("version"));
    }

    #[test]
    fn test_lazy_prop() {
        let lazy = LazyProp::new("expensive_data", || {
            // Simulate expensive computation
            json!({"result": 42})
        });

        assert_eq!(lazy.key(), "expensive_data");
        let value = lazy.evaluate();
        assert_eq!(value, json!({"result": 42}));
    }
}
