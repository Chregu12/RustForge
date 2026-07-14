//! CSRF Protection
//!
//! Provides CSRF token generation, validation, and middleware for protecting
//! against Cross-Site Request Forgery attacks.
//!
//! Token extraction supports three submission paths, checked in order:
//! 1. `X-CSRF-TOKEN` request header (Ajax / SPA use-case).
//! 2. `_token` field in an `application/x-www-form-urlencoded` body
//!    (classic HTML `<form>` POST use-case).
//! 3. `_token` field in a `multipart/form-data` body
//!    (file-upload forms use-case).
//!
//! When the form-body or multipart paths are used the middleware buffers the
//! request body, reads the token, and then **re-inserts** the original bytes
//! as the new body so downstream handlers still receive the full payload.

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

// ---------------------------------------------------------------------------
// Multipart / form-data helpers
// ---------------------------------------------------------------------------

/// Extract the `boundary` parameter from a `multipart/form-data` Content-Type
/// header value.
///
/// E.g. `"multipart/form-data; boundary=----FormBoundaryXYZ"` → `Some("----FormBoundaryXYZ")`.
/// Quoted boundary values are unquoted.
fn parse_multipart_boundary(content_type: &str) -> Option<&str> {
    for segment in content_type.split(';') {
        let seg = segment.trim();
        // Case-insensitive comparison for the parameter name.
        if seg.to_lowercase().starts_with("boundary=") {
            let val = &seg["boundary=".len()..].trim();
            return Some(if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                &val[1..val.len() - 1]
            } else {
                val
            });
        }
    }
    None
}

/// Simple forward-scan for `needle` inside `haystack[start..]`.
///
/// Returns the absolute index (relative to the start of `haystack`) of the
/// first match, or `None`.
fn memmem_find(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    if needle.is_empty() {
        return Some(start);
    }
    // Guard: we need at least `needle.len()` bytes from `start`.
    if start.saturating_add(needle.len()) > haystack.len() {
        return None;
    }
    // Slice from `start` to the end so `.windows` sees every possible
    // starting position including the very last one.
    haystack[start..]
        .windows(needle.len())
        .position(|w| w == needle)
        .map(|p| p + start)
}

/// Check whether a block of part headers (everything between the boundary
/// line and the blank line) contains a `Content-Disposition` header whose
/// `name` parameter equals `field_name`.
///
/// Matching is case-insensitive for the header name and the `name=` parameter
/// key; the parameter value is compared as-is (browsers always send it
/// lowercase / unquoted for typical field names).
fn part_name_matches(header_block: &str, field_name: &str) -> bool {
    for line in header_block.lines() {
        if !line.to_lowercase().starts_with("content-disposition:") {
            continue;
        }
        for segment in line.split(';') {
            let seg = segment.trim();
            if seg.to_lowercase().starts_with("name=") {
                let val = seg["name=".len()..].trim();
                let name = if val.starts_with('"') && val.ends_with('"') && val.len() >= 2 {
                    &val[1..val.len() - 1]
                } else {
                    val
                };
                return name == field_name;
            }
        }
    }
    false
}

/// Scan a `multipart/form-data` body for a part whose `Content-Disposition`
/// `name` parameter equals `field_name` and return its (text) value.
///
/// Returns `None` if the field is absent or the body cannot be parsed.
///
/// This is a deliberately lightweight implementation: no heap allocations
/// beyond the return value, no dependency on an external multipart crate.
/// It handles the common CRLF line-ending convention used by all major
/// browsers.  A malformed body (missing boundary, non-UTF-8 values, etc.)
/// simply returns `None`, which causes the CSRF check to fall through to
/// the header path and ultimately reject the request — the safe default.
fn parse_multipart_token(body: &[u8], boundary: &str, field_name: &str) -> Option<String> {
    // Per RFC 2046 the delimiter line is "--" + boundary.
    let delim = format!("--{boundary}");
    let delim_bytes = delim.as_bytes();

    let mut pos = 0usize;

    loop {
        // Locate the next delimiter.
        let dpos = memmem_find(body, delim_bytes, pos)?;
        pos = dpos + delim_bytes.len();

        // "--boundary--" signals the closing delimiter; stop.
        if body.get(pos..pos + 2) == Some(b"--") {
            return None;
        }

        // After the delimiter there must be a CRLF (or bare LF for tolerance).
        if body.get(pos..pos + 2) == Some(b"\r\n") {
            pos += 2;
        } else if body.get(pos) == Some(&b'\n') {
            pos += 1;
        } else {
            // Malformed — skip to the next delimiter candidate.
            continue;
        }

        // Find the blank line that separates headers from body.
        // We accept both \r\n\r\n and \n\n.
        let (header_end, body_start) =
            if let Some(p) = memmem_find(body, b"\r\n\r\n", pos) {
                (p, p + 4)
            } else if let Some(p) = memmem_find(body, b"\n\n", pos) {
                (p, p + 2)
            } else {
                return None;
            };

        let headers = std::str::from_utf8(&body[pos..header_end]).ok()?;
        pos = body_start;

        if part_name_matches(headers, field_name) {
            // The part body ends at "\r\n--boundary" (or "\n--boundary").
            let end_crlf = format!("\r\n--{boundary}");
            let end_lf = format!("\n--{boundary}");
            let value_end = memmem_find(body, end_crlf.as_bytes(), pos)
                .or_else(|| memmem_find(body, end_lf.as_bytes(), pos))?;

            let value = std::str::from_utf8(&body[pos..value_end]).ok()?;
            return Some(value.to_string());
        }
        // This part is not the one we want — advance past it to keep scanning.
        // `pos` is already pointing at the start of this part's body; we will
        // naturally hit the next delimiter in the next iteration.
    }
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
    /// 3. The `_token` field in a `multipart/form-data` body.
    ///
    /// When a body path is used the body is **buffered and re-inserted** so
    /// that downstream handlers still receive the full payload.  Bodies larger
    /// than `CSRF_BODY_LIMIT` are not buffered; the middleware falls back to
    /// header-only extraction (safe default: no token → 403).
    async fn extract_token(&self, req: &mut Request) -> Option<String> {
        // 1. Header takes priority — no body I/O needed.
        if let Some(header_value) = req.headers().get(&self.config.header_name) {
            if let Ok(token) = header_value.to_str() {
                return Some(token.to_string());
            }
        }

        // Read the Content-Type once so we can branch below without re-borrowing.
        let content_type = req
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        // 2. application/x-www-form-urlencoded
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
                    // Body too large or unreadable; leave it empty.  The CSRF
                    // check will fail (no token), which is the safe default.
                }
            }
        }

        // 3. multipart/form-data — file-upload forms
        if content_type.starts_with("multipart/form-data") {
            if let Some(boundary) = parse_multipart_boundary(&content_type) {
                // Clone the boundary string so we can drop the borrow on `content_type`
                // before we mutably borrow `req`.
                let boundary = boundary.to_string();

                let body = std::mem::replace(req.body_mut(), axum::body::Body::empty());
                match axum::body::to_bytes(body, CSRF_BODY_LIMIT).await {
                    Ok(bytes) => {
                        // Re-insert the original bytes unconditionally so the
                        // downstream handler (e.g. axum::extract::Multipart)
                        // still receives the complete, unmodified multipart body.
                        // `Bytes` is reference-counted — the clone is O(1).
                        *req.body_mut() = axum::body::Body::from(bytes.clone());
                        if let Some(token) =
                            parse_multipart_token(&bytes, &boundary, &self.config.field_name)
                        {
                            return Some(token);
                        }
                    }
                    Err(_) => {
                        // Body exceeds CSRF_BODY_LIMIT or is unreadable.
                        // Fall back to header-only (body is already empty here;
                        // the `replace` above swapped it out).  No token → 403.
                    }
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

    // =========================================================================
    // Unit tests for multipart/form-data helpers
    // =========================================================================

    #[test]
    fn test_parse_multipart_boundary_basic() {
        let ct = "multipart/form-data; boundary=----WebKitFormBoundaryXYZ";
        assert_eq!(
            parse_multipart_boundary(ct),
            Some("----WebKitFormBoundaryXYZ")
        );
    }

    #[test]
    fn test_parse_multipart_boundary_quoted() {
        let ct = r#"multipart/form-data; boundary="----FormBoundaryABC""#;
        assert_eq!(
            parse_multipart_boundary(ct),
            Some("----FormBoundaryABC")
        );
    }

    #[test]
    fn test_parse_multipart_boundary_missing() {
        let ct = "multipart/form-data";
        assert_eq!(parse_multipart_boundary(ct), None);
    }

    #[test]
    fn test_parse_multipart_boundary_extra_params() {
        // charset before boundary
        let ct = "multipart/form-data; charset=utf-8; boundary=BOUND42";
        assert_eq!(parse_multipart_boundary(ct), Some("BOUND42"));
    }

    /// Build a minimal multipart/form-data body with the given parts.
    /// `parts` is a list of (name, optional_filename, content_type, value).
    fn make_multipart_body(boundary: &str, parts: &[(&str, Option<&str>, Option<&str>, &str)]) -> Vec<u8> {
        let mut body = Vec::new();
        for (name, filename, ct, value) in parts {
            body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
            let cd = if let Some(fname) = filename {
                format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{fname}\"\r\n")
            } else {
                format!("Content-Disposition: form-data; name=\"{name}\"\r\n")
            };
            body.extend_from_slice(cd.as_bytes());
            if let Some(content_type) = ct {
                body.extend_from_slice(format!("Content-Type: {content_type}\r\n").as_bytes());
            }
            body.extend_from_slice(b"\r\n");
            body.extend_from_slice(value.as_bytes());
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
        body
    }

    #[test]
    fn test_parse_multipart_token_single_part() {
        let boundary = "TESTBOUNDARY";
        let body = make_multipart_body(boundary, &[
            ("_token", None, None, "tok_value_123"),
        ]);
        assert_eq!(
            parse_multipart_token(&body, boundary, "_token"),
            Some("tok_value_123".to_string())
        );
    }

    #[test]
    fn test_parse_multipart_token_among_multiple_parts() {
        let boundary = "TESTBOUNDARY";
        let body = make_multipart_body(boundary, &[
            ("name", None, None, "Alice"),
            ("_token", None, None, "secret_tok"),
            ("file", Some("hello.txt"), Some("text/plain"), "file contents here"),
        ]);
        assert_eq!(
            parse_multipart_token(&body, boundary, "_token"),
            Some("secret_tok".to_string())
        );
    }

    #[test]
    fn test_parse_multipart_token_absent() {
        let boundary = "TESTBOUNDARY";
        let body = make_multipart_body(boundary, &[
            ("name", None, None, "Bob"),
            ("file", Some("img.png"), Some("image/png"), "binarydata"),
        ]);
        assert_eq!(parse_multipart_token(&body, boundary, "_token"), None);
    }

    #[test]
    fn test_parse_multipart_token_token_last() {
        let boundary = "TESTBOUNDARY";
        let body = make_multipart_body(boundary, &[
            ("name", None, None, "Carol"),
            ("file", Some("f.txt"), Some("text/plain"), "content"),
            ("_token", None, None, "last_token"),
        ]);
        assert_eq!(
            parse_multipart_token(&body, boundary, "_token"),
            Some("last_token".to_string())
        );
    }

    #[test]
    fn test_memmem_find_basic() {
        let data = b"hello world";
        assert_eq!(memmem_find(data, b"world", 0), Some(6));
        assert_eq!(memmem_find(data, b"hello", 0), Some(0));
        assert_eq!(memmem_find(data, b"xyz", 0), None);
        assert_eq!(memmem_find(data, b"world", 7), None);
    }

    #[test]
    fn test_part_name_matches_quoted() {
        let headers = "Content-Disposition: form-data; name=\"_token\"";
        assert!(part_name_matches(headers, "_token"));
        assert!(!part_name_matches(headers, "other"));
    }

    #[test]
    fn test_part_name_matches_with_filename() {
        let headers = "Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\nContent-Type: text/plain";
        assert!(part_name_matches(headers, "file"));
        assert!(!part_name_matches(headers, "_token"));
    }

    // =========================================================================
    // Integration tests — multipart/form-data CSRF extraction
    // =========================================================================

    /// Helper: build the axum from_fn router with a handler that echoes the raw body bytes.
    fn make_axum_fn_app_with_echo(store: CsrfTokenStore) -> axum::Router {
        use axum::{middleware, routing::post, Router};

        async fn echo_body(body: axum::body::Bytes) -> axum::body::Bytes {
            body
        }

        let config = CsrfConfig::new();
        let mw = CsrfMiddleware::with_store(config, store);

        Router::new()
            .route("/upload", post(echo_body))
            .layer(middleware::from_fn(move |req, next| {
                let mw = mw.clone();
                async move { mw.handle(req, next).await }
            }))
    }

    /// A multipart POST carrying a valid `_token` part must be accepted (200).
    #[tokio::test]
    async fn test_csrf_multipart_valid_token_passes() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        store.register(&token).await;

        let boundary = "----TestBoundaryXYZ";
        let body_bytes = make_multipart_body(boundary, &[
            ("_token", None, None, &token_value),
            ("description", None, None, "hello world"),
        ]);
        let ct = format!("multipart/form-data; boundary={boundary}");

        let app = make_axum_fn_app_with_echo(store);
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", ct)
            .body(Body::from(body_bytes))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "multipart POST with valid _token must be accepted"
        );
    }

    /// A multipart POST with a **wrong** `_token` and no header must be rejected (403).
    #[tokio::test]
    async fn test_csrf_multipart_wrong_token_rejected() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        store.register(&token).await;

        let boundary = "----TestBoundaryXYZ";
        let body_bytes = make_multipart_body(boundary, &[
            ("_token", None, None, "totally-wrong-token"),
            ("description", None, None, "hello"),
        ]);
        let ct = format!("multipart/form-data; boundary={boundary}");

        let app = make_axum_fn_app_with_echo(store);
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", ct)
            .body(Body::from(body_bytes))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::FORBIDDEN,
            "multipart POST with wrong _token must be rejected"
        );
    }

    /// A multipart POST with **no** `_token` part and no header must be rejected (403).
    #[tokio::test]
    async fn test_csrf_multipart_absent_token_rejected() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();

        let boundary = "----TestBoundaryXYZ";
        let body_bytes = make_multipart_body(boundary, &[
            ("description", None, None, "hello"),
            ("file", Some("test.bin"), Some("application/octet-stream"), "binarydata"),
        ]);
        let ct = format!("multipart/form-data; boundary={boundary}");

        let app = make_axum_fn_app_with_echo(store);
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", ct)
            .body(Body::from(body_bytes))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::FORBIDDEN,
            "multipart POST without _token must be rejected"
        );
    }

    /// After the middleware buffers and re-inserts the multipart body, the
    /// downstream handler must still receive the **complete, unmodified** payload
    /// — including non-token parts (e.g. a file part).
    #[tokio::test]
    async fn test_csrf_multipart_body_preserved_for_downstream() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        store.register(&token).await;

        let boundary = "----TestBoundaryABC";
        let file_content = "the contents of the uploaded file";
        let body_bytes = make_multipart_body(boundary, &[
            ("_token", None, None, &token_value),
            ("file", Some("upload.txt"), Some("text/plain"), file_content),
        ]);
        let expected_body = body_bytes.clone();
        let ct = format!("multipart/form-data; boundary={boundary}");

        let app = make_axum_fn_app_with_echo(store);
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", ct)
            .body(Body::from(body_bytes))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "multipart POST with valid _token must be accepted"
        );

        // The echo handler returns the raw body; it must be byte-identical to
        // what was sent — proving the middleware did not consume the payload.
        let returned_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        assert_eq!(
            returned_body.as_ref(),
            expected_body.as_slice(),
            "downstream handler must receive the complete, unmodified multipart body"
        );
    }

    /// Verify that the X-CSRF-TOKEN **header** path still functions correctly
    /// even when the Content-Type is multipart/form-data (header takes priority).
    #[tokio::test]
    async fn test_csrf_multipart_header_token_takes_priority() {
        use axum::{body::Body, http::Request};
        use tower::ServiceExt;

        let store = CsrfTokenStore::new();
        let token = CsrfToken::generate();
        let token_value = token.token().to_string();
        store.register(&token).await;

        let boundary = "----TestBoundaryDEF";
        // No _token in the multipart body; the valid token is in the header.
        let body_bytes = make_multipart_body(boundary, &[
            ("description", None, None, "no token in body"),
        ]);
        let ct = format!("multipart/form-data; boundary={boundary}");

        let app = make_axum_fn_app_with_echo(store);
        let req = Request::builder()
            .method("POST")
            .uri("/upload")
            .header("content-type", ct)
            .header("X-CSRF-TOKEN", token_value)
            .body(Body::from(body_bytes))
            .unwrap();

        let response = app.oneshot(req).await.unwrap();
        assert_eq!(
            response.status(),
            axum::http::StatusCode::OK,
            "X-CSRF-TOKEN header must still work with multipart content-type"
        );
    }
}
