//! # rf-view-facade
//!
//! Laravel-style View facade for RustForge

use once_cell::sync::Lazy;
use rf_view::ViewEngine;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde_json::Value;
use std::collections::HashMap;

pub static GLOBAL_VIEW_ENGINE: Lazy<Arc<RwLock<Option<ViewEngine>>>> = Lazy::new(|| {
    Arc::new(RwLock::new(None))
});

pub struct View;

impl View {
    pub fn make(name: impl Into<String>, data: HashMap<String, Value>) -> ViewResponse {
        ViewResponse {
            name: name.into(),
            data,
        }
    }

    pub async fn exists(name: impl Into<String>) -> bool {
        // Simplified implementation - in real scenario would check file system
        !name.into().is_empty()
    }

    pub async fn render(name: impl Into<String>, data: HashMap<String, Value>) -> String {
        let response = Self::make(name, data);
        response.render()
    }
}

pub struct ViewResponse {
    name: String,
    data: HashMap<String, Value>,
}

impl ViewResponse {
    pub fn render(&self) -> String {
        format!("View: {}", self.name)
    }

    pub fn with_data(mut self, key: impl Into<String>, value: Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_view_make() {
        let mut data = HashMap::new();
        data.insert("title".to_string(), json!("Test"));
        let view = View::make("test.view", data);
        assert_eq!(view.name, "test.view");
    }

    #[tokio::test]
    async fn test_view_exists() {
        assert!(View::exists("some.view").await);
        assert!(!View::exists("").await);
    }

    #[tokio::test]
    async fn test_view_render() {
        let mut data = HashMap::new();
        data.insert("title".to_string(), json!("Test"));
        let rendered = View::render("test.view", data).await;
        assert_eq!(rendered, "View: test.view");
    }

    #[test]
    fn test_view_response_with_data() {
        let view = View::make("test", HashMap::new())
            .with_data("key", json!("value"));
        assert!(view.data.contains_key("key"));
    }
}
