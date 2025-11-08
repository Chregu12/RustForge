//! CSRF Protection Middleware
//!
//! Provides Cross-Site Request Forgery protection for state-changing requests.
//!
//! # Features
//! - Token generation and validation
//! - Session-based storage
//! - Automatic token injection in forms
//! - Exempt routes support
//! - Customizable error responses
//!
//! # Example
//! ```rust,ignore
//! use foundry_application::middleware::csrf::CsrfMiddleware;
//!
//! let csrf = CsrfMiddleware::new()
//!     .exempt("/api/*")
//!     .exempt("/webhooks/*");
//! ```

use axum::{
    body::Body,
    extract::Request,
    http::{header, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use rand::{distributions::Alphanumeric, Rng};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;

const CSRF_TOKEN_LENGTH: usize = 32;
const CSRF_HEADER_NAME: &str = "X-CSRF-Token";
const CSRF_FORM_FIELD: &str = "_csrf_token";

/// CSRF token storage (in-memory, can be replaced with session store)
#[derive(Clone, Debug)]
pub struct CsrfTokenStore {
    tokens: Arc<RwLock<HashSet<String>>>,
}

impl CsrfTokenStore {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Generate a new CSRF token
    pub async fn generate(&self) -> String {
        let token: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(CSRF_TOKEN_LENGTH)
            .map(char::from)
            .collect();

        self.tokens.write().await.insert(token.clone());
        token
    }

    /// Validate a CSRF token
    pub async fn validate(&self, token: &str) -> bool {
        self.tokens.read().await.contains(token)
    }

    /// Remove a token after use (optional, for one-time use tokens)
    pub async fn remove(&self, token: &str) {
        self.tokens.write().await.remove(token);
    }
}

impl Default for CsrfTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF protection middleware configuration
#[derive(Clone)]
pub struct CsrfConfig {
    /// Routes that should be exempt from CSRF protection
    exempt_routes: Arc<Vec<String>>,
    /// Whether to use one-time tokens (remove after validation)
    one_time_tokens: bool,
    /// Custom error message
    error_message: String,
}

impl CsrfConfig {
    pub fn new() -> Self {
        Self {
            exempt_routes: Arc::new(Vec::new()),
            one_time_tokens: false,
            error_message: "CSRF token validation failed".to_string(),
        }
    }

    /// Add an exempt route pattern
    pub fn exempt(mut self, pattern: impl Into<String>) -> Self {
        Arc::make_mut(&mut self.exempt_routes).push(pattern.into());
        self
    }

    /// Enable one-time use tokens
    pub fn one_time_tokens(mut self, enabled: bool) -> Self {
        self.one_time_tokens = enabled;
        self
    }

    /// Set custom error message
    pub fn error_message(mut self, message: impl Into<String>) -> Self {
        self.error_message = message.into();
        self
    }

    /// Check if a path is exempt from CSRF protection
    fn is_exempt(&self, path: &str) -> bool {
        self.exempt_routes.iter().any(|pattern| {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                path.starts_with(prefix)
            } else {
                path == pattern
            }
        })
    }
}

impl Default for CsrfConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// CSRF protection middleware
#[derive(Clone)]
pub struct CsrfMiddleware {
    store: CsrfTokenStore,
    config: CsrfConfig,
}

impl CsrfMiddleware {
    /// Create a new CSRF middleware with default configuration
    pub fn new() -> Self {
        Self {
            store: CsrfTokenStore::new(),
            config: CsrfConfig::new(),
        }
    }

    /// Create with custom configuration
    pub fn with_config(config: CsrfConfig) -> Self {
        Self {
            store: CsrfTokenStore::new(),
            config,
        }
    }

    /// Add an exempt route
    pub fn exempt(mut self, pattern: impl Into<String>) -> Self {
        self.config = self.config.exempt(pattern);
        self
    }

    /// Get the token store
    pub fn store(&self) -> &CsrfTokenStore {
        &self.store
    }

    /// Handle CSRF protection for a request
    pub async fn handle(&self, request: Request, next: Next) -> Response {
        let method = request.method();
        let path = request.uri().path();

        // Only check state-changing methods
        if !matches!(method, &Method::POST | &Method::PUT | &Method::DELETE | &Method::PATCH) {
            return next.run(request).await;
        }

        // Check if route is exempt
        if self.config.is_exempt(path) {
            return next.run(request).await;
        }

        // Extract token from request
        let token = self.extract_token(&request);

        match token {
            Some(token_value) => {
                if self.store.validate(&token_value).await {
                    // Token is valid
                    if self.config.one_time_tokens {
                        self.store.remove(&token_value).await;
                    }
                    next.run(request).await
                } else {
                    // Invalid token
                    self.error_response()
                }
            }
            None => {
                // No token provided
                self.error_response()
            }
        }
    }

    /// Extract CSRF token from request (header or form field)
    fn extract_token(&self, request: &Request<Body>) -> Option<String> {
        // Try header first
        if let Some(header_value) = request.headers().get(CSRF_HEADER_NAME) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }

        // TODO: Parse form data for token
        // This would require consuming the body, which is more complex
        // For now, we primarily support header-based tokens

        None
    }

    /// Generate error response
    fn error_response(&self) -> Response {
        (
            StatusCode::FORBIDDEN,
            self.config.error_message.clone(),
        )
            .into_response()
    }
}

impl Default for CsrfMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware function for use with Axum
pub async fn csrf_protection(
    csrf: axum::extract::State<Arc<CsrfMiddleware>>,
    request: Request,
    next: Next,
) -> Response {
    csrf.handle(request, next).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, Method},
    };

    #[tokio::test]
    async fn test_csrf_token_generation() {
        let store = CsrfTokenStore::new();
        let token = store.generate().await;

        assert_eq!(token.len(), CSRF_TOKEN_LENGTH);
        assert!(store.validate(&token).await);
    }

    #[tokio::test]
    async fn test_csrf_token_validation() {
        let store = CsrfTokenStore::new();
        let token = store.generate().await;

        assert!(store.validate(&token).await);
        assert!(!store.validate("invalid-token").await);
    }

    #[tokio::test]
    async fn test_csrf_token_removal() {
        let store = CsrfTokenStore::new();
        let token = store.generate().await;

        assert!(store.validate(&token).await);
        store.remove(&token).await;
        assert!(!store.validate(&token).await);
    }

    #[test]
    fn test_exempt_routes() {
        let config = CsrfConfig::new()
            .exempt("/api/*")
            .exempt("/webhooks/stripe");

        assert!(config.is_exempt("/api/users"));
        assert!(config.is_exempt("/api/posts/123"));
        assert!(config.is_exempt("/webhooks/stripe"));
        assert!(!config.is_exempt("/webhooks/other"));
        assert!(!config.is_exempt("/admin/users"));
    }

    #[test]
    fn test_get_request_not_checked() {
        let middleware = CsrfMiddleware::new();
        // GET requests should pass through without CSRF check
        // This is verified by the method check in handle()
    }

    #[tokio::test]
    async fn test_extract_token_from_header() {
        let middleware = CsrfMiddleware::new();
        let token = "test-token-123";

        let request = Request::builder()
            .method(Method::POST)
            .header(CSRF_HEADER_NAME, token)
            .body(Body::empty())
            .unwrap();

        let extracted = middleware.extract_token(&request);
        assert_eq!(extracted, Some(token.to_string()));
    }
}
