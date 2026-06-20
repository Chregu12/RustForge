//! URL generation helpers.

use crate::named_routes::{NamedRoute, ParamValue, RouteRegistry};
use crate::signed_urls::{SignedUrl, SignedUrlBuilder};
use std::collections::HashMap;

/// URL generator for creating URLs from routes.
pub struct UrlGenerator {
    registry: RouteRegistry,
    base_url: String,
    secret: String,
}

impl UrlGenerator {
    /// Create a new URL generator.
    pub fn new(base_url: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            registry: RouteRegistry::new(),
            base_url: base_url.into(),
            secret: secret.into(),
        }
    }

    /// Register a named route.
    pub fn register(&mut self, route: NamedRoute) {
        self.registry.register(route);
    }

    /// Generate a URL for a named route.
    pub fn route(&self, name: &str, params: HashMap<String, ParamValue>) -> Option<String> {
        self.registry.url(name, &params)
    }

    /// Generate a signed URL.
    pub fn signed_route(
        &self,
        name: &str,
        params: HashMap<String, ParamValue>,
        expires_in_minutes: Option<i64>,
    ) -> Option<SignedUrl> {
        let url = self.route(name, params)?;
        let full_url = format!("{}{}", self.base_url, url);

        let mut builder = SignedUrlBuilder::new(full_url, &self.secret);
        if let Some(minutes) = expires_in_minutes {
            builder = builder.expires_in_minutes(minutes);
        }

        Some(builder.build())
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Get the route registry.
    pub fn registry(&self) -> &RouteRegistry {
        &self.registry
    }
}

/// Helper macro for generating URLs.
#[macro_export]
macro_rules! route_params {
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut params: std::collections::HashMap<String, $crate::ParamValue> = std::collections::HashMap::new();
        $(
            params.insert($key.to_string(), $value.into());
        )*
        params
    }};
}

/// Query string builder.
#[derive(Debug, Clone, Default)]
pub struct QueryStringBuilder {
    params: HashMap<String, String>,
}

impl QueryStringBuilder {
    /// Create a new query string builder.
    pub fn new() -> Self {
        Self {
            params: HashMap::new(),
        }
    }

    /// Add a parameter.
    pub fn add(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Build the query string.
    pub fn build(self) -> String {
        if self.params.is_empty() {
            return String::new();
        }

        fn url_encode(s: &str) -> String {
            let mut encoded = String::with_capacity(s.len());
            for byte in s.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        encoded.push(byte as char);
                    }
                    _ => {
                        encoded.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
            encoded
        }

        let pairs: Vec<String> = self
            .params
            .iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect();

        format!("?{}", pairs.join("&"))
    }
}

/// URL builder for constructing complex URLs.
#[derive(Debug, Clone)]
pub struct UrlBuilder {
    base: String,
    path_segments: Vec<String>,
    query_params: HashMap<String, String>,
    fragment: Option<String>,
}

impl UrlBuilder {
    /// Create a new URL builder.
    pub fn new(base: impl Into<String>) -> Self {
        Self {
            base: base.into(),
            path_segments: Vec::new(),
            query_params: HashMap::new(),
            fragment: None,
        }
    }

    /// Add a path segment.
    pub fn segment(mut self, segment: impl Into<String>) -> Self {
        self.path_segments.push(segment.into());
        self
    }

    /// Add a query parameter.
    pub fn query(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.query_params.insert(key.into(), value.into());
        self
    }

    /// Set the fragment.
    pub fn fragment(mut self, fragment: impl Into<String>) -> Self {
        self.fragment = Some(fragment.into());
        self
    }

    /// Build the final URL.
    pub fn build(self) -> String {
        let mut url = self.base;

        // Add path segments
        for segment in self.path_segments {
            if !url.ends_with('/') && !segment.starts_with('/') {
                url.push('/');
            }
            url.push_str(&segment);
        }

        // Add query parameters
        if !self.query_params.is_empty() {
            let query = QueryStringBuilder::new();
            let query = self
                .query_params
                .into_iter()
                .fold(query, |q, (k, v)| q.add(k, v));
            url.push_str(&query.build());
        }

        // Add fragment
        if let Some(fragment) = self.fragment {
            url.push('#');
            url.push_str(&fragment);
        }

        url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::named_routes::NamedRoute;

    #[test]
    fn test_url_generator() {
        let mut generator = UrlGenerator::new("https://example.com", "secret");

        let route = NamedRoute::new("users.show", "/users/{id}");
        generator.register(route);

        let params = route_params! {
            "id" => 123
        };

        let url = generator.route("users.show", params);
        assert_eq!(url, Some("/users/123".to_string()));
    }

    #[test]
    fn test_query_string_builder() {
        let query = QueryStringBuilder::new()
            .add("page", "1")
            .add("per_page", "10")
            .add("sort", "name")
            .build();

        assert!(query.starts_with('?'));
        assert!(query.contains("page=1"));
        assert!(query.contains("per_page=10"));
        assert!(query.contains("sort=name"));
    }

    #[test]
    fn test_url_builder() {
        let url = UrlBuilder::new("https://example.com")
            .segment("api")
            .segment("users")
            .segment("123")
            .query("include", "posts")
            .query("fields", "name,email")
            .fragment("profile")
            .build();

        assert!(url.starts_with("https://example.com/api/users/123"));
        assert!(url.contains("include=posts"));
        // The query builder percent-encodes everything outside the RFC 3986
        // unreserved set, so the comma in the value is escaped as %2C.
        assert!(url.contains("fields=name%2Cemail"));
        assert!(url.ends_with("#profile"));
    }

    #[test]
    fn test_url_builder_no_query() {
        let url = UrlBuilder::new("https://example.com")
            .segment("api")
            .segment("users")
            .build();

        assert_eq!(url, "https://example.com/api/users");
    }

    #[test]
    fn test_route_params_macro() {
        let params = route_params! {
            "id" => 123,
            "slug" => "test-post",
        };

        assert_eq!(params.len(), 2);
        assert!(params.contains_key("id"));
        assert!(params.contains_key("slug"));
    }
}
