//! Laravel-style View facade for RustForge
//!
//! This provides compatibility with the facade pattern.
//! For full functionality, use the `View` struct directly from this crate.

use once_cell::sync::Lazy;
use crate::engine::ViewEngine;
use std::sync::RwLock;
use serde_json::Value;
use std::collections::HashMap;

/// Global view engine instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_VIEW_ENGINE: Lazy<RwLock<Option<ViewEngine>>> = Lazy::new(|| {
    RwLock::new(None)
});

/// A simple view facade response
pub struct ViewFacadeResponse {
    pub name: String,
    pub data: HashMap<String, Value>,
}

impl ViewFacadeResponse {
    /// Render the view response
    pub fn render(&self) -> String {
        format!("View: {}", self.name)
    }

    /// Add data to the view response
    pub fn with_data(mut self, key: impl Into<String>, value: Value) -> Self {
        self.data.insert(key.into(), value);
        self
    }
}

/// Simple view facade for compatibility
///
/// # Note
///
/// For full view functionality with layouts and components,
/// use the `View` struct from this crate instead.
///
/// # Examples
///
/// ```rust
/// use rf_view::ViewFacade;
/// use std::collections::HashMap;
/// use serde_json::json;
///
/// let mut data = HashMap::new();
/// data.insert("title".to_string(), json!("Hello"));
/// let view = ViewFacade::make("welcome", data);
/// ```
pub struct ViewFacade;

impl ViewFacade {
    /// Make a view with data
    pub fn make(name: impl Into<String>, data: HashMap<String, Value>) -> ViewFacadeResponse {
        ViewFacadeResponse {
            name: name.into(),
            data,
        }
    }

    /// Check if a view exists
    pub fn exists(name: impl Into<String>) -> bool {
        // Simplified implementation - in real scenario would check file system
        !name.into().is_empty()
    }

    /// Render a view with data
    pub fn render(name: impl Into<String>, data: HashMap<String, Value>) -> String {
        let response = Self::make(name, data);
        response.render()
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
        let view = ViewFacade::make("test.view", data);
        assert_eq!(view.name, "test.view");
    }

    #[test]
    fn test_view_exists() {
        assert!(ViewFacade::exists("some.view"));
        assert!(!ViewFacade::exists(""));
    }

    #[test]
    fn test_view_render() {
        let mut data = HashMap::new();
        data.insert("title".to_string(), json!("Test"));
        let rendered = ViewFacade::render("test.view", data);
        assert_eq!(rendered, "View: test.view");
    }

    #[test]
    fn test_view_response_with_data() {
        let view = ViewFacade::make("test", HashMap::new())
            .with_data("key", json!("value"));
        assert!(view.data.contains_key("key"));
    }
}
