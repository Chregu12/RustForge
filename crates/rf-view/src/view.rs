use crate::engine::ViewEngine;
use crate::error::ViewResult;
use crate::response::ViewResponse;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use tera::Context;

/// A view that can be rendered
///
/// # Example
///
/// ```rust,no_run
/// use rf_view::View;
/// use serde_json::json;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Simple view
/// let html = View::make("welcome", json!({"title": "Welcome"}))
///     .render()
///     .await?;
///
/// // With layout
/// let html = View::make("pages.home", json!({"user": "John"}))
///     .layout("layouts.app")
///     .render()
///     .await?;
///
/// // As HTTP response
/// let response = View::make("home", json!({"title": "Home"}))
///     .into_response();
/// # Ok(())
/// # }
/// ```
pub struct View {
    /// Template name (e.g., "welcome.tera" or "pages/home")
    template: String,

    /// Template data
    data: Context,

    /// Optional layout template
    layout: Option<String>,

    /// Additional sections for layouts
    sections: HashMap<String, String>,
}

impl View {
    /// Create a new view
    ///
    /// # Arguments
    ///
    /// * `template` - Template name (e.g., "welcome", "pages.home")
    /// * `data` - Template data as JSON value
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::View;
    /// use serde_json::json;
    ///
    /// let view = View::make("welcome", json!({
    ///     "title": "Welcome",
    ///     "user": {
    ///         "name": "John Doe",
    ///         "email": "john@example.com"
    ///     }
    /// }));
    /// ```
    pub fn make<T: Serialize>(template: impl Into<String>, data: T) -> Self {
        let template = template.into();
        let data_value = serde_json::to_value(data).unwrap_or_else(|_| json!({}));

        let mut context = Context::new();
        if let Value::Object(map) = data_value {
            for (key, value) in map {
                context.insert(&key, &value);
            }
        }

        Self {
            template: Self::normalize_template_name(&template),
            data: context,
            layout: None,
            sections: HashMap::new(),
        }
    }

    /// Set the layout template
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::View;
    /// use serde_json::json;
    ///
    /// let view = View::make("home", json!({}))
    ///     .layout("layouts.app");
    /// ```
    pub fn layout(mut self, layout: impl Into<String>) -> Self {
        self.layout = Some(Self::normalize_template_name(&layout.into()));
        self
    }

    /// Add data to the view
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::View;
    /// use serde_json::json;
    ///
    /// let view = View::make("home", json!({"title": "Home"}))
    ///     .with("user", json!({"name": "John"}))
    ///     .with("posts", json!([]));
    /// ```
    pub fn with<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        let value = serde_json::to_value(value).unwrap_or_else(|_| json!(null));
        self.data.insert(&key.into(), &value);
        self
    }

    /// Add a named section (for layouts)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::View;
    /// use serde_json::json;
    ///
    /// let view = View::make("home", json!({}))
    ///     .section("scripts", "<script>console.log('Hello');</script>");
    /// ```
    pub fn section(mut self, name: impl Into<String>, content: impl Into<String>) -> Self {
        self.sections.insert(name.into(), content.into());
        self
    }

    /// Render the view to HTML string
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::View;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let html = View::make("welcome", json!({"title": "Welcome"}))
    ///     .render()
    ///     .await?;
    ///
    /// println!("{}", html);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn render(&self) -> ViewResult<String> {
        // Add sections to context
        let mut context = self.data.clone();
        for (name, content) in &self.sections {
            context.insert(name, content);
        }

        // Render the template
        let html = ViewEngine::render(&self.template, &context)?;

        // If there's a layout, wrap the content
        if let Some(layout) = &self.layout {
            let mut layout_context = context.clone();
            layout_context.insert("content", &html);

            ViewEngine::render(layout, &layout_context)
        } else {
            Ok(html)
        }
    }

    /// Convert to an Axum HTTP response
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_view::View;
    /// use serde_json::json;
    /// use axum::response::IntoResponse;
    ///
    /// async fn handler() -> impl IntoResponse {
    ///     View::make("home", json!({"title": "Home"}))
    /// }
    /// ```
    pub fn into_response(self) -> ViewResponse {
        ViewResponse { view: self }
    }

    /// Normalize template name (convert dots to slashes, add .tera extension if needed)
    ///
    /// # Examples
    ///
    /// - "welcome" -> "welcome.tera"
    /// - "pages.home" -> "pages/home.tera"
    /// - "layouts.app" -> "layouts/app.tera"
    /// - "welcome.tera" -> "welcome.tera" (unchanged)
    fn normalize_template_name(name: &str) -> String {
        // If already has .tera extension, return as-is
        if name.ends_with(".tera") {
            return name.to_string();
        }

        // Replace dots with slashes and add .tera extension
        let name = name.replace('.', "/");
        format!("{}.tera", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_template_name() {
        assert_eq!(View::normalize_template_name("welcome"), "welcome.tera");
        assert_eq!(View::normalize_template_name("pages.home"), "pages/home.tera");
        assert_eq!(View::normalize_template_name("layouts.app"), "layouts/app.tera");
        assert_eq!(View::normalize_template_name("welcome.tera"), "welcome.tera");
    }

    #[test]
    fn test_view_creation() {
        let view = View::make("welcome", json!({
            "title": "Welcome",
            "user": "John"
        }));

        assert_eq!(view.template, "welcome.tera");
        assert!(view.layout.is_none());
    }

    #[test]
    fn test_view_with_layout() {
        let view = View::make("home", json!({}))
            .layout("layouts.app");

        assert_eq!(view.layout, Some("layouts/app.tera".to_string()));
    }

    #[test]
    fn test_view_with_data() {
        let view = View::make("home", json!({"title": "Home"}))
            .with("user", json!({"name": "John"}))
            .with("count", json!(42));

        // Check that data was added (would need actual rendering to verify)
        assert_eq!(view.template, "home.tera");
    }
}
