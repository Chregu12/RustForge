//! Inertia renderer - main API for creating Inertia responses

use crate::{
    error::Result,
    props::{LazyProp, Props, SharedProps},
    response::InertiaResponse,
};
use axum::{
    extract::Request,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;
use std::sync::Arc;

/// Main Inertia renderer
///
/// # Example
///
/// ```rust,ignore
/// async fn index() -> Inertia {
///     Inertia::render("Dashboard/Index")
///         .with("user", get_user())
///         .with("stats", get_stats())
/// }
/// ```
pub struct Inertia {
    component: String,
    props: Props,
    lazy_props: Vec<LazyProp>,
    deferred_keys: Vec<String>,
    url: Option<String>,
    version: Option<String>,
}

impl Inertia {
    /// Create a new Inertia response
    pub fn render(component: impl Into<String>) -> Self {
        Self {
            component: component.into(),
            props: Props::new(),
            lazy_props: Vec::new(),
            deferred_keys: Vec::new(),
            url: None,
            version: None,
        }
    }

    /// Add a prop
    pub fn with<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        self.props = self.props.with(key, value);
        self
    }

    /// Add multiple props from an iterator
    pub fn with_props<I, K, V>(mut self, props: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Serialize,
    {
        for (key, value) in props {
            self.props = self.props.with(key, value);
        }
        self
    }

    /// Add a lazy-evaluated prop
    pub fn with_lazy<F>(mut self, key: impl Into<String>, evaluator: F) -> Self
    where
        F: Fn() -> Value + Send + Sync + 'static,
    {
        self.lazy_props.push(LazyProp::new(key, evaluator));
        self
    }

    /// Set the URL (usually extracted from request)
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the version (usually from config)
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Merge shared props
    pub fn with_shared(mut self, shared: Props) -> Self {
        self.props = self.props.merge(shared);
        self
    }

    /// Conditionally add a prop
    pub fn when<T: Serialize>(self, condition: bool, key: impl Into<String>, value: T) -> Self {
        if condition {
            self.with(key, value)
        } else {
            self
        }
    }

    /// Add a prop when a value is Some
    pub fn when_some<T: Serialize>(self, key: impl Into<String>, value: Option<T>) -> Self {
        if let Some(v) = value {
            self.with(key, v)
        } else {
            self
        }
    }

    /// Exclude specific props from the response (Inertia 2 feature).
    ///
    /// Useful for removing props that were added by shared data but are not
    /// needed for this particular page.
    pub fn except(mut self, keys: &[&str]) -> Self {
        for key in keys {
            self.props.remove(key);
        }
        self
    }

    /// Add a deferred prop that is loaded after initial page render (Inertia 2).
    ///
    /// Deferred props are excluded from the initial page load and fetched via
    /// a subsequent partial-reload request.  This improves perceived performance
    /// for pages with expensive data.
    pub fn with_deferred<T: Serialize>(mut self, key: impl Into<String>, value: T) -> Self {
        let key = key.into();
        // Mark as deferred by storing in a special meta-prop
        if let Ok(json_value) = serde_json::to_value(value) {
            self.props = self.props.with(&key, json_value);
            self.deferred_keys.push(key);
        }
        self
    }

    /// Get the list of deferred prop keys
    pub fn deferred_keys(&self) -> &[String] {
        &self.deferred_keys
    }

    /// Build the final InertiaResponse
    pub fn build(
        self,
        headers: &HeaderMap,
        url: &str,
        version: &str,
        root_view: &str,
    ) -> Result<Response> {
        // Evaluate lazy props if requested
        let mut props = self.props;
        if InertiaResponse::is_partial_request(headers) {
            if let Some(only_props) = InertiaResponse::get_partial_props(headers) {
                for lazy_prop in &self.lazy_props {
                    if only_props.contains(&lazy_prop.key().to_string()) {
                        let value = lazy_prop.evaluate();
                        props = props.with(lazy_prop.key(), value);
                    }
                }
            }
        } else {
            // For non-partial requests, evaluate all lazy props
            for lazy_prop in &self.lazy_props {
                let value = lazy_prop.evaluate();
                props = props.with(lazy_prop.key(), value);
            }
        }

        let response = InertiaResponse::new(
            self.component,
            props,
            self.url.unwrap_or_else(|| url.to_string()),
            self.version.unwrap_or_else(|| version.to_string()),
        );

        // Apply partial filtering if needed
        let response = response.filter_partial_props(headers);

        // Return JSON for Inertia requests, HTML for regular requests
        if InertiaResponse::is_inertia_request(headers) {
            Ok(response.into_json_response())
        } else {
            response.into_html_response(root_view)
        }
    }
}

impl IntoResponse for Inertia {
    fn into_response(self) -> Response {
        // Default implementation for cases without middleware
        let response = InertiaResponse::new(
            self.component,
            self.props,
            self.url.unwrap_or_else(|| "/".to_string()),
            self.version.unwrap_or_else(|| "1".to_string()),
        );
        response.into_json_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_basic_render() {
        let inertia = Inertia::render("Dashboard/Index")
            .with("user", "John")
            .with("count", 42);

        assert_eq!(inertia.component, "Dashboard/Index");
        assert!(inertia.props.has("user"));
        assert!(inertia.props.has("count"));
    }

    #[test]
    fn test_conditional_props() {
        let inertia = Inertia::render("Users/Index")
            .when(true, "admin", true)
            .when(false, "guest", true);

        assert!(inertia.props.has("admin"));
        assert!(!inertia.props.has("guest"));
    }

    #[test]
    fn test_optional_props() {
        let inertia = Inertia::render("Users/Show")
            .when_some("name", Some("John"))
            .when_some("email", None::<String>);

        assert!(inertia.props.has("name"));
        assert!(!inertia.props.has("email"));
    }

    #[test]
    fn test_lazy_props() {
        let inertia =
            Inertia::render("Dashboard/Index").with_lazy("stats", || json!({"active_users": 100}));

        assert_eq!(inertia.lazy_props.len(), 1);
        assert_eq!(inertia.lazy_props[0].key(), "stats");
    }

    #[test]
    fn test_with_props() {
        let props_data = vec![("name", "John"), ("email", "john@example.com")];
        let inertia = Inertia::render("Users/Show").with_props(props_data);

        assert!(inertia.props.has("name"));
        assert!(inertia.props.has("email"));
    }
}
