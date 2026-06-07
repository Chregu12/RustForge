//! CSRF Protection
//!
//! Provides CSRF token generation, validation, and middleware for protecting
//! against Cross-Site Request Forgery attacks.

use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
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
            token: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes),
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

/// Server-side store for valid CSRF tokens.
///
/// Tokens are stored by their string value and automatically expired during
/// validation to prevent memory growth.
#[derive(Clone, Default)]
pub struct CsrfTokenStore {
    tokens: Arc<RwLock<HashMap<String, CsrfToken>>>,
}

impl CsrfTokenStore {
    /// Create a new empty token store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a newly generated token in the store.
    pub async fn register(&self, token: &CsrfToken) {
        let mut store = self.tokens.write().await;
        store.insert(token.token().to_string(), token.clone());
    }

    /// Validate a submitted token string against the store.
    ///
    /// Uses constant-time comparison to prevent timing attacks.
    /// Removes the token after successful validation (one-time use).
    /// Also purges expired tokens on every call.
    pub async fn validate(&self, submitted: &str, lifetime_hours: i64) -> bool {
        let mut store = self.tokens.write().await;
        let duration = Duration::hours(lifetime_hours);

        // Purge expired tokens
        store.retain(|_, t| !t.is_expired_with_duration(duration));

        // Look up the submitted token
        if let Some(stored) = store.get(submitted) {
            if stored.verify(submitted) {
                // One-time use: remove after successful validation
                store.remove(submitted);
                return true;
            }
        }

        false
    }
}

/// CSRF middleware for protecting routes
#[derive(Clone)]
pub struct CsrfMiddleware {
    config: Arc<CsrfConfig>,
    token_store: CsrfTokenStore,
}

impl CsrfMiddleware {
    /// Create a new CSRF middleware with default configuration
    pub fn new() -> Self {
        Self {
            config: Arc::new(CsrfConfig::default()),
            token_store: CsrfTokenStore::new(),
        }
    }

    /// Create a new CSRF middleware with custom configuration
    pub fn with_config(config: CsrfConfig) -> Self {
        Self {
            config: Arc::new(config),
            token_store: CsrfTokenStore::new(),
        }
    }

    /// Create a new CSRF middleware sharing an existing token store.
    ///
    /// Use this when you need to register tokens (e.g. from a handler) and
    /// validate them in the middleware within the same request lifecycle.
    pub fn with_store(config: CsrfConfig, token_store: CsrfTokenStore) -> Self {
        Self {
            config: Arc::new(config),
            token_store,
        }
    }

    /// Get a reference to the token store so handlers can register tokens.
    pub fn token_store(&self) -> &CsrfTokenStore {
        &self.token_store
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
        let submitted = self.extract_token(&mut req).await;

        // Validate submitted token against the server-side token store
        let valid = match submitted {
            Some(ref value) => {
                self.token_store
                    .validate(value, self.config.token_lifetime_hours)
                    .await
            }
            None => false,
        };

        if !valid {
            return (StatusCode::FORBIDDEN, "CSRF token mismatch").into_response();
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

    /// Get the shared token store so handlers can register generated tokens.
    pub fn token_store(&self) -> &CsrfTokenStore {
        self.middleware.token_store()
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
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
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

            // Extract and validate CSRF token
            let submitted = req
                .headers()
                .get(&middleware.config.header_name)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            let valid = match submitted {
                Some(ref value) => {
                    middleware
                        .token_store
                        .validate(value, middleware.config.token_lifetime_hours)
                        .await
                }
                None => false,
            };

            if !valid {
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
    format!(
        r#"<input type="hidden" name="_token" value="{}">"#,
        token.token()
    )
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

    #[tokio::test]
    async fn test_csrf_token_store_validate_valid() {
        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let value = token.token().to_string();

        store.register(&token).await;
        assert!(store.validate(&value, 2).await);
    }

    #[tokio::test]
    async fn test_csrf_token_store_validate_invalid() {
        let store = CsrfTokenStore::new();
        assert!(!store.validate("bogus_token", 2).await);
    }

    #[tokio::test]
    async fn test_csrf_token_store_one_time_use() {
        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let value = token.token().to_string();

        store.register(&token).await;
        // First use succeeds
        assert!(store.validate(&value, 2).await);
        // Second use fails (token was consumed)
        assert!(!store.validate(&value, 2).await);
    }

    #[tokio::test]
    async fn test_csrf_token_store_expired_token_rejected() {
        let store = CsrfTokenStore::new();
        let mut token = CsrfToken::generate();
        let value = token.token().to_string();

        // Backdate the token so it is already expired
        token.created_at = Utc::now() - Duration::hours(3);
        store.register(&token).await;

        // Validate with a 2-hour lifetime → expired
        assert!(!store.validate(&value, 2).await);
    }

    #[test]
    fn test_csrf_token_display() {
        let token = CsrfToken::generate();
        let displayed = format!("{}", token);
        assert_eq!(displayed, token.token());
    }

    #[test]
    fn test_csrf_token_length_is_sufficient() {
        let token = CsrfToken::generate();
        // 32 bytes base64-encoded should be at least 40 characters (URL_SAFE_NO_PAD)
        assert!(token.token().len() >= 40);
    }

    #[test]
    fn test_csrf_token_verify_rejects_empty_string() {
        let token = CsrfToken::generate();
        assert!(!token.verify(""));
    }

    #[test]
    fn test_csrf_token_verify_rejects_partial_match() {
        let token = CsrfToken::generate();
        let value = token.token();
        // Truncate the value — should not verify
        assert!(!token.verify(&value[..5]));
    }

    #[test]
    fn test_csrf_config_defaults() {
        let config = CsrfConfig::default();
        assert_eq!(config.token_lifetime_hours, 2);
        assert_eq!(config.field_name, "_token");
        assert_eq!(config.header_name, "X-CSRF-TOKEN");
        assert!(config.exempt_routes.is_empty());
    }

    #[test]
    fn test_csrf_middleware_not_exempt_for_non_matching_routes() {
        let config = CsrfConfig::new().exempt("/api/webhooks");
        let middleware = CsrfMiddleware::with_config(config);

        assert!(!middleware.is_exempt("/users"));
        assert!(!middleware.is_exempt("/admin/dashboard"));
    }

    #[test]
    fn test_csrf_middleware_exempt_prefix_matching() {
        let config = CsrfConfig::new().exempt("/api/");
        let middleware = CsrfMiddleware::with_config(config);

        assert!(middleware.is_exempt("/api/users"));
        assert!(middleware.is_exempt("/api/posts/123"));
        assert!(!middleware.is_exempt("/web/home"));
    }

    #[tokio::test]
    async fn test_csrf_token_store_multiple_tokens() {
        let store = CsrfTokenStore::new();
        let token1 = CsrfToken::generate();
        let token2 = CsrfToken::generate();
        let value1 = token1.token().to_string();
        let value2 = token2.token().to_string();

        store.register(&token1).await;
        store.register(&token2).await;

        // Both should be valid
        assert!(store.validate(&value1, 2).await);
        assert!(store.validate(&value2, 2).await);
    }

    #[tokio::test]
    async fn test_csrf_token_store_empty_string_rejected() {
        let store = CsrfTokenStore::new();
        assert!(!store.validate("", 2).await);
    }

    #[test]
    fn test_csrf_field_contains_value() {
        let token = CsrfToken::generate();
        let field = csrf_field(&token);
        assert!(field.contains(&format!("value=\"{}\"", token.token())));
    }

    #[test]
    fn test_csrf_meta_contains_value() {
        let token = CsrfToken::generate();
        let meta = csrf_meta(&token);
        assert!(meta.contains(&format!("content=\"{}\"", token.token())));
    }
}
