//! Inertia response types

use crate::{error::Result, props::Props};
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Inertia response structure (Inertia 2 compatible)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InertiaResponse {
    /// Component name (e.g., "Dashboard/Index")
    pub component: String,

    /// Props data
    pub props: Props,

    /// URL of the current page
    pub url: String,

    /// Asset version
    pub version: String,

    /// Keys of deferred props (loaded after initial render)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deferred_props: Vec<String>,

    /// Whether to clear the navigation history (Inertia 2)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub clear_history: bool,

    /// Whether to encrypt history state (Inertia 2)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub encrypt_history: bool,
}

impl InertiaResponse {
    /// Create a new Inertia response
    pub fn new(
        component: impl Into<String>,
        props: Props,
        url: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            component: component.into(),
            props,
            url: url.into(),
            version: version.into(),
            deferred_props: Vec::new(),
            clear_history: false,
            encrypt_history: false,
        }
    }

    /// Mark specific props as deferred (Inertia 2)
    pub fn with_deferred_props(mut self, keys: Vec<String>) -> Self {
        self.deferred_props = keys;
        self
    }

    /// Clear navigation history on this response (Inertia 2)
    pub fn clear_history(mut self) -> Self {
        self.clear_history = true;
        self
    }

    /// Encrypt history state for this response (Inertia 2)
    pub fn encrypt_history(mut self) -> Self {
        self.encrypt_history = true;
        self
    }

    /// Convert to JSON response
    pub fn into_json_response(self) -> Response {
        (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(self),
        )
            .into_response()
    }

    /// Convert to HTML response with embedded JSON
    pub fn into_html_response(self, root_view: &str) -> Result<Response> {
        let json_data = serde_json::to_string(&self)?;

        // Escape for safe embedding in HTML attribute
        let escaped_json = json_data
            .replace('&', "&amp;")
            .replace('\'', "&#x27;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        let escaped_root_view = root_view
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;");

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=1.0" />
    <title>{}</title>
</head>
<body>
    <div id="{}" data-page='{}'></div>
</body>
</html>"#,
            escaped_root_view, escaped_root_view, escaped_json
        );

        Ok((StatusCode::OK, Html(html)).into_response())
    }

    /// Check if this is an Inertia request
    pub fn is_inertia_request(headers: &HeaderMap) -> bool {
        headers
            .get("X-Inertia")
            .and_then(|v| v.to_str().ok())
            .map(|v| v == "true")
            .unwrap_or(false)
    }

    /// Check if this is a partial reload request
    pub fn is_partial_request(headers: &HeaderMap) -> bool {
        headers.contains_key("X-Inertia-Partial-Component")
    }

    /// Get the requested partial props
    pub fn get_partial_props(headers: &HeaderMap) -> Option<Vec<String>> {
        headers
            .get("X-Inertia-Partial-Data")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
    }

    /// Filter props for partial reload
    pub fn filter_partial_props(mut self, headers: &HeaderMap) -> Self {
        if let Some(only_props) = Self::get_partial_props(headers) {
            let mut filtered_data = std::collections::HashMap::new();
            for key in only_props {
                if let Some(value) = self.props.get(&key) {
                    filtered_data.insert(key, value.clone());
                }
            }
            self.props = Props::from(filtered_data);
        }
        self
    }
}

impl InertiaResponse {
    /// Render this page via the SSR server and return a full HTML string.
    ///
    /// Requires the `ssr` Cargo feature.  Falls back to a client-side-only HTML
    /// shell when the feature is absent.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_inertia::{response::InertiaResponse, ssr::SsrConfig, props::Props};
    ///
    /// # async fn example() -> rf_inertia::error::Result<String> {
    /// let page = InertiaResponse::new("Home/Index", Props::new(), "/", "1.0");
    /// let html = page.into_ssr_response("app", SsrConfig::default()).await?;
    /// # Ok(html)
    /// # }
    /// ```
    pub async fn into_ssr_response(
        self,
        root_element_id: &str,
        ssr_config: crate::ssr::SsrConfig,
    ) -> crate::error::Result<String> {
        let client = crate::ssr::SsrClient::new(ssr_config);
        client.render_to_html(&self, root_element_id).await
    }
}

impl IntoResponse for InertiaResponse {
    fn into_response(self) -> Response {
        self.into_json_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inertia_response_creation() {
        let props = Props::new().with("user", "John");
        let response = InertiaResponse::new("Dashboard/Index", props, "/dashboard", "v1");

        assert_eq!(response.component, "Dashboard/Index");
        assert_eq!(response.url, "/dashboard");
        assert_eq!(response.version, "v1");
    }

    #[test]
    fn test_is_inertia_request() {
        let mut headers = HeaderMap::new();
        assert!(!InertiaResponse::is_inertia_request(&headers));

        headers.insert("X-Inertia", "true".parse().unwrap());
        assert!(InertiaResponse::is_inertia_request(&headers));
    }

    #[test]
    fn test_partial_props() {
        let mut headers = HeaderMap::new();
        headers.insert("X-Inertia-Partial-Data", "name,email".parse().unwrap());

        let partial_props = InertiaResponse::get_partial_props(&headers);
        assert_eq!(
            partial_props,
            Some(vec!["name".to_string(), "email".to_string()])
        );
    }

    #[test]
    fn test_filter_partial_props() {
        let props = Props::new()
            .with("name", "John")
            .with("email", "john@example.com")
            .with("password", "secret");

        let response = InertiaResponse::new("Users/Show", props, "/users/1", "v1");

        let mut headers = HeaderMap::new();
        headers.insert("X-Inertia-Partial-Data", "name,email".parse().unwrap());

        let filtered = response.filter_partial_props(&headers);

        assert!(filtered.props.has("name"));
        assert!(filtered.props.has("email"));
        assert!(!filtered.props.has("password"));
    }
}
