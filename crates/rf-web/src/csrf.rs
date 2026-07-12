//! CSRF Protection
//!
//! Provides CSRF token generation, validation, and middleware for protecting
//! against Cross-Site Request Forgery attacks.
//!
//! Token extraction supports two submission paths, checked in order:
//! 1. `X-CSRF-TOKEN` request header (Ajax / SPA use-case).
//! 2. `_token` field in an `application/x-www-form-urlencoded` body
//!    (classic HTML `<form>` POST use-case).
//!
//! When the form-body path is used the middleware buffers the request body,
//! reads the token, and then **re-inserts** the original bytes as the new
//! body so downstream handlers still receive the full payload.

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

/// Maximum body size the CSRF middleware will buffer when scanning for a
/// `_token` form field.  Requests with a body larger than this are still
/// accepted by the middleware (the body is not consumed), but the form-field
/// path returns `None` and the request will be rejected unless the header
/// path succeeds.
const CSRF_BODY_LIMIT: usize = 4 * 1024 * 1024; // 4 MiB

/// Decode a single `application/x-www-form-urlencoded` component.
///
/// Converts `+` → space and `%XX` → the corresponding byte, then returns the
/// result as a `String`.  Multi-byte UTF-8 sequences encoded as consecutive
/// `%XX` escapes are decoded one byte at a time; the resulting string is
/// constructed with `as char` casts, which is correct for ASCII values (the
/// only encoding that matters for a CSRF token or field name).
fn decode_form_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            // SAFETY: slice is valid bytes, from_utf8 checked below.
            if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte as char);
                    i += 3;
                    continue;
                }
            }
            // Not a valid percent-escape — emit literally.
            out.push('%');
            i += 1;
        } else if bytes[i] == b'+' {
            out.push(' ');
            i += 1;
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

/// Scan an `application/x-www-form-urlencoded` body for `field_name` and
/// return its percent-decoded value, or `None` if the field is not present.
fn parse_form_token(body_bytes: &[u8], field_name: &str) -> Option<String> {
    let body_str = std::str::from_utf8(body_bytes).ok()?;
    for pair in body_str.split('&') {
        if let Some((raw_key, raw_value)) = pair.split_once('=') {
            if decode_form_component(raw_key) == field_name {
                return Some(decode_form_component(raw_value));
            }
        }
    }
    None
}

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
            token: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes),
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

    /// Extract the CSRF token from the request.
    ///
    /// Checks, in order:
    /// 1. The configured header (e.g. `X-CSRF-TOKEN`).
    /// 2. The `_token` field in an `application/x-www-form-urlencoded` body.
    ///
    /// When the form-body path is used the body is **buffered and re-inserted**
    /// so that downstream handlers still receive the full payload.
    async fn extract_token(&self, req: &mut Request) -> Option<String> {
        // 1. Header takes priority — no body I/O needed.
        if let Some(header_value) = req.headers().get(&self.config.header_name) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }

        // 2. Form body — only for urlencoded content.
        let content_type = req
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        if content_type.starts_with("application/x-www-form-urlencoded") {
            // Swap out the body so we can read it asynchronously.  We replace
            // it with an empty body first; after reading we re-insert the
            // original bytes so the downstream handler still sees the payload.
            let body = std::mem::replace(req.body_mut(), axum::body::Body::empty());
            match axum::body::to_bytes(body, CSRF_BODY_LIMIT).await {
                Ok(bytes) => {
                    // Re-insert **before** returning so the body is always
                    // restored regardless of whether we found the token.
                    // `Bytes` is ref-counted; the clone here is cheap (O(1)).
                    *req.body_mut() = axum::body::Body::from(bytes.clone());
                    if let Some(token) = parse_form_token(&bytes, &self.config.field_name) {
                        return Some(token);
                    }
                }
                Err(_) => {
                    // Body could not be read; leave it empty.  The CSRF check
                    // will fail (no token), which is the safe default.
                }
            }
        }

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

            // Skip CSRF for safe methods.
            if method == axum::http::Method::GET
                || method == axum::http::Method::HEAD
                || method == axum::http::Method::OPTIONS
            {
                return inner.call(req).await;
            }

            // Skip CSRF for exempt routes.
            if middleware.is_exempt(&path) {
                return inner.call(req).await;
            }

            // Extract token via header or form body (body is re-inserted when
            // the form-body path is used so downstream sees the full payload).
            let mut req = req;
            let submitted = middleware.extract_token(&mut req).await;

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

    // -------------------------------------------------------------------------
    // Unit tests for the form-body helper functions
    // -------------------------------------------------------------------------

    #[test]
    fn test_decode_form_component_no_encoding() {
        // URL-safe base64 characters need no decoding at all.
        let raw = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
        assert_eq!(decode_form_component(raw), raw);
    }

    #[test]
    fn test_decode_form_component_plus_to_space() {
        assert_eq!(decode_form_component("hello+world"), "hello world");
    }

    #[test]
    fn test_decode_form_component_percent_escape() {
        assert_eq!(decode_form_component("hello%20world"), "hello world");
        assert_eq!(decode_form_component("%41"), "A"); // 0x41 = 'A'
    }

    #[test]
    fn test_decode_form_component_invalid_escape_emitted_literally() {
        // A lone '%' with no valid hex digits is kept as-is.
        assert_eq!(decode_form_component("%ZZ"), "%ZZ");
    }

    #[test]
    fn test_parse_form_token_middle_field() {
        let body = b"name=John&_token=abc123&age=30";
        assert_eq!(
            parse_form_token(body, "_token"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_parse_form_token_first_field() {
        let body = b"_token=abc123&name=John";
        assert_eq!(
            parse_form_token(body, "_token"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_parse_form_token_last_field() {
        let body = b"name=John&age=30&_token=abc123";
        assert_eq!(
            parse_form_token(body, "_token"),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn test_parse_form_token_not_found() {
        let body = b"name=John&age=30";
        assert_eq!(parse_form_token(body, "_token"), None);
    }

    #[test]
    fn test_parse_form_token_empty_body() {
        assert_eq!(parse_form_token(b"", "_token"), None);
    }

    // -------------------------------------------------------------------------
    // Integration tests — form-body vs header token paths
    //
    // These exercise `CsrfMiddleware::handle()` (the axum middleware function)
    // and `CsrfMiddlewareService::call()` (the tower service) independently.
    // -------------------------------------------------------------------------

    /// Build a minimal axum Router with the CSRF middleware (axum `from_fn` path).
    fn make_axum_fn_app(store: CsrfTokenStore) -> axum::Router {
        use axum::{middleware, routing::post, Router};

        let config = CsrfConfig::new();
        let mw = CsrfMiddleware::with_store(config, store);

        Router::new()
            .route("/submit", post(|| async { "OK" }))
            .layer(middleware::from_fn(move |req, next| {
                let mw = mw.clone();
                async move { mw.handle(req, next).await }
            }))
    }

    // -- axum `from_fn` path --------------------------------------------------

    #[tokio::test]
    async fn test_csrf_form_body_valid_token_passes() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        store.register(&token).await;

        let app = make_axum_fn_app(store);

        let body = format!("name=John&_token={token_value}");
        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_form_body_wrong_token_rejected() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        store.register(&token).await;

        let app = make_axum_fn_app(store);

        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("name=John&_token=totally-wrong-token"))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_csrf_form_body_absent_token_rejected() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let app = make_axum_fn_app(store);

        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("name=John&age=30"))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_csrf_header_token_still_works() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        store.register(&token).await;

        let app = make_axum_fn_app(store);

        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("X-CSRF-TOKEN", token_value)
            .header("content-type", "application/json")
            .body(Body::from(r#"{"name":"John"}"#))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    /// After the CSRF middleware buffers and re-inserts the form body, the
    /// downstream handler must still see the original, complete payload.
    #[tokio::test]
    async fn test_csrf_form_body_preserved_for_downstream_handler() {
        use axum::{body::Body, http::Request, middleware, routing::post, Router};
        use tower::ServiceExt;

        async fn echo_body(body: axum::body::Bytes) -> String {
            String::from_utf8_lossy(&body).into_owned()
        }

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        store.register(&token).await;

        let config = CsrfConfig::new();
        let mw = CsrfMiddleware::with_store(config, store);

        let app = Router::new()
            .route("/submit", post(echo_body))
            .layer(middleware::from_fn(move |req, next| {
                let mw = mw.clone();
                async move { mw.handle(req, next).await }
            }));

        let form_body = format!("name=John&_token={token_value}&age=30");
        let expected = form_body.clone();

        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(form_body))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);

        let body_bytes = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        assert_eq!(std::str::from_utf8(&body_bytes).unwrap(), expected);
    }

    // -- tower `CsrfLayer` path (exercises `CsrfMiddlewareService::call`) -----

    #[tokio::test]
    async fn test_csrf_tower_layer_form_body_valid_token_passes() {
        use axum::{body::Body, http::Request, routing::post, Router};
        use tower::ServiceExt;

        let layer = CsrfLayer::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        layer.token_store().register(&token).await;

        let app = Router::new()
            .route("/submit", post(|| async { "OK" }))
            .layer(layer);

        let body = format!("name=John&_token={token_value}");
        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from(body))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn test_csrf_tower_layer_form_body_wrong_token_rejected() {
        use axum::{body::Body, http::Request, routing::post, Router};
        use tower::ServiceExt;

        let layer = CsrfLayer::new();
        let token = CsrfToken::generate();
        layer.token_store().register(&token).await;

        let app = Router::new()
            .route("/submit", post(|| async { "OK" }))
            .layer(layer);

        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("content-type", "application/x-www-form-urlencoded")
            .body(Body::from("name=John&_token=wrong"))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_csrf_tower_layer_header_token_still_works() {
        use axum::{body::Body, http::Request, routing::post, Router};
        use tower::ServiceExt;

        let layer = CsrfLayer::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        layer.token_store().register(&token).await;

        let app = Router::new()
            .route("/submit", post(|| async { "OK" }))
            .layer(layer);

        let req = Request::builder()
            .method("POST")
            .uri("/submit")
            .header("X-CSRF-TOKEN", token_value)
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::OK);
    }
}
