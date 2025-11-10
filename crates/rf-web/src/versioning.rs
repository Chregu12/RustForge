//! API Versioning support
//!
//! Provides multiple strategies for API versioning:
//! - URL-based versioning (e.g., /v1/users, /v2/users)
//! - Header-based versioning (e.g., Api-Version: 1.0)
//! - Accept header versioning (e.g., application/vnd.api.v1+json)
//!
//! # Examples
//!
//! ## URL Versioning
//!
//! ```no_run
//! use rf_web::versioning::*;
//! use axum::{Router, routing::get};
//!
//! async fn users_v1() -> &'static str { "Users API v1" }
//! async fn users_v2() -> &'static str { "Users API v2" }
//!
//! let app = Router::new()
//!     .nest("/v1", Router::new().route("/users", get(users_v1)))
//!     .nest("/v2", Router::new().route("/users", get(users_v2)));
//! ```
//!
//! ## Header Versioning
//!
//! ```no_run
//! use rf_web::versioning::*;
//! use axum::routing::get;
//!
//! async fn handler(version: ApiVersion) -> String {
//!     format!("API version: {}", version.as_str())
//! }
//!
//! let app = axum::Router::new()
//!     .route("/users", get(handler));
//! ```

use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header, request::Parts, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use std::fmt;

/// API version
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiVersion {
    version: String,
}

impl ApiVersion {
    /// Create new API version
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    /// Get version as string
    pub fn as_str(&self) -> &str {
        &self.version
    }

    /// Check if version matches
    pub fn matches(&self, version: &str) -> bool {
        self.version == version
    }

    /// Parse version from string (e.g., "v1", "1.0", "2.0")
    pub fn parse(s: &str) -> Result<Self, VersionError> {
        let version = s.trim().trim_start_matches('v');
        if version.is_empty() {
            return Err(VersionError::Invalid(s.to_string()));
        }
        Ok(Self {
            version: version.to_string(),
        })
    }

    /// Create version not supported response
    pub fn not_supported(&self) -> Response {
        VersionError::NotSupported(self.version.clone()).into_response()
    }
}

impl fmt::Display for ApiVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl From<&str> for ApiVersion {
    fn from(s: &str) -> Self {
        Self::parse(s).unwrap_or_else(|_| Self::new("1.0"))
    }
}

/// Version extraction strategy
#[derive(Debug, Clone, Copy)]
pub enum VersionStrategy {
    /// Extract from Api-Version header
    Header,
    /// Extract from Accept header (application/vnd.api.v1+json)
    Accept,
}

/// Version errors
#[derive(Debug)]
pub enum VersionError {
    /// Version not found in request
    NotFound,
    /// Invalid version format
    Invalid(String),
    /// Version not supported
    NotSupported(String),
}

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VersionError::NotFound => write!(f, "API version not specified"),
            VersionError::Invalid(v) => write!(f, "Invalid API version: {}", v),
            VersionError::NotSupported(v) => write!(f, "API version {} not supported", v),
        }
    }
}

impl std::error::Error for VersionError {}

impl IntoResponse for VersionError {
    fn into_response(self) -> Response {
        let status = match &self {
            VersionError::NotFound => StatusCode::BAD_REQUEST,
            VersionError::Invalid(_) => StatusCode::BAD_REQUEST,
            VersionError::NotSupported(_) => StatusCode::NOT_ACCEPTABLE,
        };

        (status, self.to_string()).into_response()
    }
}

/// Axum extractor for API version
///
/// Extracts version from Api-Version header by default
#[async_trait]
impl<S> FromRequestParts<S> for ApiVersion
where
    S: Send + Sync,
{
    type Rejection = VersionError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Try Api-Version header
        if let Some(version) = parts.headers.get("Api-Version") {
            let version_str = version
                .to_str()
                .map_err(|_| VersionError::Invalid("Invalid header value".to_string()))?;
            return ApiVersion::parse(version_str);
        }

        // Try X-Api-Version header (alternative)
        if let Some(version) = parts.headers.get("X-Api-Version") {
            let version_str = version
                .to_str()
                .map_err(|_| VersionError::Invalid("Invalid header value".to_string()))?;
            return ApiVersion::parse(version_str);
        }

        // Try Accept header (application/vnd.api.v1+json)
        if let Some(accept) = parts.headers.get(header::ACCEPT) {
            if let Ok(accept_str) = accept.to_str() {
                if let Some(version) = extract_version_from_accept(accept_str) {
                    return Ok(ApiVersion::new(version));
                }
            }
        }

        // Default to version 1.0 if not specified
        Ok(ApiVersion::new("1.0"))
    }
}

/// Extract version from Accept header
fn extract_version_from_accept(accept: &str) -> Option<String> {
    // Look for patterns like: application/vnd.api.v1+json
    for part in accept.split(',') {
        let part = part.trim();
        if part.contains("vnd.api.v") {
            if let Some(start) = part.find(".v") {
                let version_part = &part[start + 2..];
                if let Some(end) = version_part.find('+') {
                    return Some(version_part[..end].to_string());
                }
            }
        }
    }
    None
}

/// Helper for building versioned routers
pub struct VersionedRouter {
    versions: Vec<(String, axum::Router)>,
}

impl VersionedRouter {
    /// Create new versioned router
    pub fn new() -> Self {
        Self {
            versions: Vec::new(),
        }
    }

    /// Add a version
    pub fn version(mut self, version: impl Into<String>, router: axum::Router) -> Self {
        self.versions.push((version.into(), router));
        self
    }

    /// Build the final router with /v{version} prefixes
    pub fn build(self) -> axum::Router {
        let mut router = axum::Router::new();
        for (version, version_router) in self.versions {
            let path = format!("/v{}", version.trim_start_matches('v'));
            router = router.nest(&path, version_router);
        }
        router
    }
}

impl Default for VersionedRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    #[test]
    fn test_api_version_parse() {
        let v1 = ApiVersion::parse("1.0").unwrap();
        assert_eq!(v1.as_str(), "1.0");

        let v2 = ApiVersion::parse("v2").unwrap();
        assert_eq!(v2.as_str(), "2");

        let v3 = ApiVersion::parse("2.5").unwrap();
        assert_eq!(v3.as_str(), "2.5");
    }

    #[test]
    fn test_api_version_matches() {
        let version = ApiVersion::new("1.0");
        assert!(version.matches("1.0"));
        assert!(!version.matches("2.0"));
    }

    #[tokio::test]
    async fn test_version_from_header() {
        async fn handler(version: ApiVersion) -> String {
            format!("Version: {}", version.as_str())
        }

        let app = Router::new().route("/", get(handler));

        let request = Request::builder()
            .uri("/")
            .header("Api-Version", "2.0")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_version_from_x_api_version_header() {
        async fn handler(version: ApiVersion) -> String {
            format!("Version: {}", version.as_str())
        }

        let app = Router::new().route("/", get(handler));

        let request = Request::builder()
            .uri("/")
            .header("X-Api-Version", "1.5")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_extract_version_from_accept() {
        let accept = "application/vnd.api.v1+json";
        assert_eq!(extract_version_from_accept(accept), Some("1".to_string()));

        let accept = "application/vnd.api.v2.5+json";
        assert_eq!(
            extract_version_from_accept(accept),
            Some("2.5".to_string())
        );

        let accept = "application/json";
        assert_eq!(extract_version_from_accept(accept), None);
    }

    #[tokio::test]
    async fn test_version_from_accept_header() {
        async fn handler(version: ApiVersion) -> String {
            format!("Version: {}", version.as_str())
        }

        let app = Router::new().route("/", get(handler));

        let request = Request::builder()
            .uri("/")
            .header("Accept", "application/vnd.api.v2+json")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_default_version() {
        async fn handler(version: ApiVersion) -> String {
            format!("Version: {}", version.as_str())
        }

        let app = Router::new().route("/", get(handler));

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_versioned_router() {
        async fn v1_handler() -> &'static str {
            "API v1"
        }
        async fn v2_handler() -> &'static str {
            "API v2"
        }

        let app = VersionedRouter::new()
            .version("1", Router::new().route("/users", get(v1_handler)))
            .version("2", Router::new().route("/users", get(v2_handler)))
            .build();

        // Test v1
        let request = Request::builder()
            .uri("/v1/users")
            .body(Body::empty())
            .unwrap();
        let response = app.clone().oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Test v2
        let request = Request::builder()
            .uri("/v2/users")
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
