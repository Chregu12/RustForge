//! CSRF Protection
//!
//! Provides CSRF token generation, validation, and middleware for protecting
//! against Cross-Site Request Forgery attacks.

use axum::{
    body::Body,
    extract::{FromRequestParts, Request},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::Layer;

/// CSRF token with creation timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrfToken {
    token: String,
    created_at: DateTime<Utc>,
}

impl CsrfToken {
    /// Generate a new CSRF token with cryptographically secure random bytes
    pub fn generate() -> Self {
        use base64::Engine;
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);

        Self {
            token: base64::engine::general_purpose::URL_SAFE_NO_PAD
                .encode(&bytes),
            created_at: Utc::now(),
        }
    }

    /// Verify a token using constant-time comparison
    pub fn verify(&self, input: &str) -> bool {
        use subtle::ConstantTimeEq;

        if self.is_expired() {
            return false;
        }

        self.token.as_bytes().ct_eq(input.as_bytes()).into()
    }

    /// Check if the token has expired (default: 2 hours)
    pub fn is_expired(&self) -> bool {
        self.is_expired_with_duration(Duration::hours(2))
    }

    /// Check if the token has expired with custom duration
    pub fn is_expired_with_duration(&self, duration: Duration) -> bool {
        Utc::now() > self.created_at + duration
    }

    /// Get the token value
    pub fn token(&self) -> &str {
        &self.token
    }

    /// Get the creation timestamp
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Regenerate the token (creates a new one)
    pub fn regenerate() -> Self {
        Self::generate()
    }
}

impl std::fmt::Display for CsrfToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.token)
    }
}

/// Configuration for CSRF protection
#[derive(Debug, Clone)]
pub struct CsrfConfig {
    /// Routes to exempt from CSRF protection
    pub exempt_routes: Vec<String>,
    /// Token lifetime in hours
    pub token_lifetime_hours: i64,
    /// Form field name for token
    pub field_name: String,
    /// Header name for token
    pub header_name: String,
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self {
            exempt_routes: Vec::new(),
            token_lifetime_hours: 2,
            field_name: "_token".to_string(),
            header_name: "X-CSRF-TOKEN".to_string(),
        }
    }
}

impl CsrfConfig {
    /// Create a new CSRF configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an exempt route pattern
    pub fn exempt(mut self, route: impl Into<String>) -> Self {
        self.exempt_routes.push(route.into());
        self
    }

    /// Set token lifetime in hours
    pub fn lifetime_hours(mut self, hours: i64) -> Self {
        self.token_lifetime_hours = hours;
        self
    }

    /// Set the form field name
    pub fn field_name(mut self, name: impl Into<String>) -> Self {
        self.field_name = name.into();
        self
    }

    /// Set the header name
    pub fn header_name(mut self, name: impl Into<String>) -> Self {
        self.header_name = name.into();
        self
    }
}

/// CSRF middleware for protecting routes
#[derive(Clone)]
pub struct CsrfMiddleware {
    config: Arc<CsrfConfig>,
}

impl CsrfMiddleware {
    /// Create a new CSRF middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: Arc::new(CsrfConfig::default()),
        }
    }

    /// Create a new CSRF middleware with custom configuration
    pub fn with_config(config: CsrfConfig) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    /// Check if a route is exempt from CSRF protection
    fn is_exempt(&self, path: &str) -> bool {
        for pattern in &self.config.exempt_routes {
            if path.starts_with(pattern) {
                return true;
            }
        }
        false
    }

    /// Extract token from request (form field or header)
    async fn extract_token(&self, req: &mut Request) -> Option<String> {
        // First try to get from header
        if let Some(header_value) = req.headers().get(&self.config.header_name) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }

        // Then try to get from form data
        // Note: This is simplified - in production, you'd need to properly parse the body
        // while preserving it for the handler
        None
    }

    /// Handle the CSRF validation
    pub async fn handle(&self, mut req: Request, next: Next) -> Response {
        let method = req.method().clone();
        let path = req.uri().path().to_string();

        // Skip CSRF for safe methods
        if method == axum::http::Method::GET
            || method == axum::http::Method::HEAD
            || method == axum::http::Method::OPTIONS
        {
            return next.run(req).await;
        }

        // Skip CSRF for exempt routes
        if self.is_exempt(&path) {
            return next.run(req).await;
        }

        // Extract token from request
        let token = self.extract_token(&mut req).await;

        // TODO: Validate token against session token
        // For now, we'll just check if a token exists
        if token.is_none() {
            return (
                StatusCode::FORBIDDEN,
                "CSRF token mismatch",
            ).into_response();
        }

        next.run(req).await
    }
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF middleware layer for tower
#[derive(Clone)]
pub struct CsrfLayer {
    middleware: CsrfMiddleware,
}

impl CsrfLayer {
    /// Create a new CSRF layer
    pub fn new() -> Self {
        Self {
            middleware: CsrfMiddleware::new(),
        }
    }

    /// Create a new CSRF layer with custom configuration
    pub fn with_config(config: CsrfConfig) -> Self {
        Self {
            middleware: CsrfMiddleware::with_config(config),
        }
    }
}

impl Default for CsrfLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for CsrfLayer {
    type Service = CsrfMiddlewareService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CsrfMiddlewareService {
            inner,
            middleware: self.middleware.clone(),
        }
    }
}

/// CSRF middleware service
#[derive(Clone)]
pub struct CsrfMiddlewareService<S> {
    inner: S,
    middleware: CsrfMiddleware,
}

impl<S> tower::Service<Request> for CsrfMiddlewareService<S>
where
    S: tower::Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let middleware = self.middleware.clone();
        let mut inner = self.inner.clone();

        Box::pin(async move {
            let method = req.method().clone();
            let path = req.uri().path().to_string();

            // Skip CSRF for safe methods
            if method == axum::http::Method::GET
                || method == axum::http::Method::HEAD
                || method == axum::http::Method::OPTIONS
            {
                return inner.call(req).await;
            }

            // Skip CSRF for exempt routes
            if middleware.is_exempt(&path) {
                return inner.call(req).await;
            }

            // Check for CSRF token in headers
            let has_token = req.headers().get(&middleware.config.header_name).is_some();

            if !has_token {
                let response = (StatusCode::FORBIDDEN, "CSRF token mismatch").into_response();
                return Ok(response);
            }

            inner.call(req).await
        })
    }
}

/// Helper function to generate CSRF token
pub fn csrf_token() -> CsrfToken {
    CsrfToken::generate()
}

/// Helper function to generate CSRF field HTML
pub fn csrf_field(token: &CsrfToken) -> String {
    format!(r#"<input type="hidden" name="_token" value="{}">"#, token.token())
}

/// Helper function to get CSRF meta tag HTML
pub fn csrf_meta(token: &CsrfToken) -> String {
    format!(r#"<meta name="csrf-token" content="{}">"#, token.token())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_csrf_token_generation() {
        let token = CsrfToken::generate();
        assert!(!token.token().is_empty());
        assert_eq!(token.created_at().date_naive(), Utc::now().date_naive());
    }

    #[test]
    fn test_csrf_token_verification() {
        let token = CsrfToken::generate();
        let value = token.token().to_string();

        assert!(token.verify(&value));
        assert!(!token.verify("invalid_token"));
    }

    #[test]
    fn test_csrf_token_expiration() {
        let mut token = CsrfToken::generate();
        assert!(!token.is_expired());

        // Simulate expired token
        token.created_at = Utc::now() - Duration::hours(3);
        assert!(token.is_expired());
    }

    #[test]
    fn test_csrf_token_custom_expiration() {
        let mut token = CsrfToken::generate();
        token.created_at = Utc::now() - Duration::minutes(30);

        assert!(!token.is_expired_with_duration(Duration::hours(1)));
        assert!(token.is_expired_with_duration(Duration::minutes(15)));
    }

    #[test]
    fn test_csrf_config_builder() {
        let config = CsrfConfig::new()
            .exempt("/api/webhook")
            .exempt("/health")
            .lifetime_hours(4)
            .field_name("csrf_token")
            .header_name("X-XSRF-TOKEN");

        assert_eq!(config.exempt_routes.len(), 2);
        assert_eq!(config.token_lifetime_hours, 4);
        assert_eq!(config.field_name, "csrf_token");
        assert_eq!(config.header_name, "X-XSRF-TOKEN");
    }

    #[test]
    fn test_csrf_middleware_exempt() {
        let config = CsrfConfig::new().exempt("/api/");
        let middleware = CsrfMiddleware::with_config(config);

        assert!(middleware.is_exempt("/api/users"));
        assert!(middleware.is_exempt("/api/posts"));
        assert!(!middleware.is_exempt("/users"));
    }

    #[test]
    fn test_csrf_field_generation() {
        let token = CsrfToken::generate();
        let field = csrf_field(&token);

        assert!(field.contains(r#"type="hidden""#));
        assert!(field.contains(r#"name="_token""#));
        assert!(field.contains(&token.token()));
    }

    #[test]
    fn test_csrf_meta_generation() {
        let token = CsrfToken::generate();
        let meta = csrf_meta(&token);

        assert!(meta.contains(r#"name="csrf-token""#));
        assert!(meta.contains(&token.token()));
    }

    #[test]
    fn test_csrf_token_regeneration() {
        let token1 = CsrfToken::generate();
        let token2 = CsrfToken::regenerate();

        assert_ne!(token1.token(), token2.token());
    }
}
