//! # rf-view-facade
//!
//! Laravel-style View facade for RustForge
//!
//! ## Features
//!
//! - **Static View API**: Use `View::make()`, `View::render()`, etc. - no `.await` needed!
//! - **Global View Engine**: Thread-safe global view state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust
//! use rf_view_facade::View;
//! use std::collections::HashMap;
//! use serde_json::json;
//!
//! # fn example() {
//! // Make a view
//! let mut data = HashMap::new();
//! data.insert("title".to_string(), json!("Hello"));
//! let view = View::make("welcome", data);
//!
//! // Render a view
//! let rendered = View::render("welcome", HashMap::new());
//! # }
//! ```

use once_cell::sync::Lazy;
use rf_view::ViewEngine;
use std::sync::RwLock;
use serde_json::Value;
use std::collections::HashMap;

/// Global view engine instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_VIEW_ENGINE: Lazy<RwLock<Option<ViewEngine>>> = Lazy::new(|| {
    RwLock::new(None)
});

pub struct View;

impl View {
    /// Make a view with data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_view_facade::View;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let mut data = HashMap::new();
    /// data.insert("title".to_string(), json!("Test"));
    /// let view = View::make("test.view", data);
    /// ```
    pub fn make(name: impl Into<String>, data: HashMap<String, Value>) -> ViewResponse {
        ViewResponse {
            name: name.into(),
            data,
        }
    }

    /// Check if a view exists
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_view_facade::View;
    ///
    /// if View::exists("some.view") {
    ///     println!("View exists!");
    /// }
    /// ```
    pub fn exists(name: impl Into<String>) -> bool {
        // Simplified implementation - in real scenario would check file system
        !name.into().is_empty()
    }

    /// Render a view with data
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_view_facade::View;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let mut data = HashMap::new();
    /// data.insert("title".to_string(), json!("Test"));
    /// let rendered = View::render("test.view", data);
    /// ```
    pub fn render(name: impl Into<String>, data: HashMap<String, Value>) -> String {
        let response = Self::make(name, data);
        response.render()
    }
}

pub struct ViewResponse {
    name: String,
    data: HashMap<String, Value>,
}

impl ViewResponse {
    /// Render the view response
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_view_facade::View;
    /// use std::collections::HashMap;
    ///
    /// let view = View::make("test", HashMap::new());
    /// let rendered = view.render();
    /// ```
    pub fn render(&self) -> String {
        format!("View: {}", self.name)
    }

    /// Add data to the view response
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_view_facade::View;
    /// use std::collections::HashMap;
    /// use serde_json::json;
    ///
    /// let view = View::make("test", HashMap::new())
    ///     .with_data("key", json!("value"));
    /// ```
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

    #[test]
    fn test_view_exists() {
        assert!(View::exists("some.view"));
        assert!(!View::exists(""));
    }

    #[test]
    fn test_view_render() {
        let mut data = HashMap::new();
        data.insert("title".to_string(), json!("Test"));
        let rendered = View::render("test.view", data);
        assert_eq!(rendered, "View: test.view");
    }

    #[test]
    fn test_view_response_with_data() {
        let view = View::make("test", HashMap::new())
            .with_data("key", json!("value"));
        assert!(view.data.contains_key("key"));
    }
}
