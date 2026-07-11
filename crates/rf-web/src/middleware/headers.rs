//! Security response-headers middleware
//!
//! Adds security-related HTTP response headers to protect against common web
//! vulnerabilities. This middleware is **opt-in** — routers that do not add
//! it are completely unaffected.
//!
//! # Quick start
//!
//! ```rust,no_run
//! use rf_web::{security_headers_layer, SecurityHeadersConfig};
//! use axum::Router;
//!
//! let app: Router = Router::new()
//!     .layer(security_headers_layer(SecurityHeadersConfig::default()));
//! ```

use axum::body::Body;
use axum::http::{HeaderName, HeaderValue, Request, Response};
use futures::future::BoxFuture;
use std::sync::Arc;
use std::task::{Context, Poll};
use tower::{Layer, Service};

// ---------------------------------------------------------------------------
// HSTS configuration
// ---------------------------------------------------------------------------

/// Configuration for the `Strict-Transport-Security` (HSTS) header.
///
/// Only send HSTS on HTTPS endpoints — browsers will refuse future plain-HTTP
/// connections to the origin for the duration of `max_age_secs`.
///
/// # Example
///
/// ```rust
/// use rf_web::HstsConfig;
///
/// let hsts = HstsConfig::default(); // 1 year, includeSubDomains, no preload
/// let hsts = HstsConfig { preload: true, ..HstsConfig::default() };
/// ```
#[derive(Debug, Clone)]
pub struct HstsConfig {
    /// `max-age` directive in seconds (default: 31 536 000 = 1 year).
    pub max_age_secs: u64,
    /// Include the `includeSubDomains` directive (default: `true`).
    pub include_subdomains: bool,
    /// Include the `preload` directive (default: `false`).
    pub preload: bool,
}

impl Default for HstsConfig {
    fn default() -> Self {
        Self {
            max_age_secs: 31_536_000,
            include_subdomains: true,
            preload: false,
        }
    }
}

impl HstsConfig {
    fn header_value(&self) -> String {
        let mut value = format!("max-age={}", self.max_age_secs);
        if self.include_subdomains {
            value.push_str("; includeSubDomains");
        }
        if self.preload {
            value.push_str("; preload");
        }
        value
    }
}

// ---------------------------------------------------------------------------
// SecurityHeadersConfig
// ---------------------------------------------------------------------------

/// Configuration for the security-headers middleware.
///
/// Secure defaults are applied by [`Default`] / [`SecurityHeadersConfig::new`]:
///
/// | Header | Default |
/// |--------|---------|
/// | `X-Content-Type-Options` | `nosniff` |
/// | `X-Frame-Options` | `DENY` |
/// | `Referrer-Policy` | `no-referrer` |
/// | `Strict-Transport-Security` | *(disabled — opt-in)* |
/// | `Content-Security-Policy` | *(disabled — caller provides)* |
///
/// Each header can be customised or disabled independently using the builder
/// methods.
///
/// # Examples
///
/// ```rust
/// use rf_web::{SecurityHeadersConfig, HstsConfig};
///
/// // Secure defaults
/// let config = SecurityHeadersConfig::default();
///
/// // Customise for a production HTTPS API
/// let config = SecurityHeadersConfig::new()
///     .x_frame_options("SAMEORIGIN")
///     .hsts(HstsConfig::default())
///     .content_security_policy("default-src 'self'");
///
/// // Strip X-Frame-Options for a known-iframe context
/// let config = SecurityHeadersConfig::new().no_x_frame_options();
/// ```
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// `X-Content-Type-Options` value, or `None` to omit the header.
    pub x_content_type_options: Option<String>,
    /// `X-Frame-Options` value, or `None` to omit the header.
    pub x_frame_options: Option<String>,
    /// `Referrer-Policy` value, or `None` to omit the header.
    pub referrer_policy: Option<String>,
    /// `Strict-Transport-Security` configuration, or `None` to omit the header.
    pub hsts: Option<HstsConfig>,
    /// `Content-Security-Policy` value, or `None` to omit the header.
    pub content_security_policy: Option<String>,
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            x_content_type_options: Some("nosniff".to_string()),
            x_frame_options: Some("DENY".to_string()),
            referrer_policy: Some("no-referrer".to_string()),
            hsts: None,
            content_security_policy: None,
        }
    }
}

impl SecurityHeadersConfig {
    /// Create configuration with secure defaults (same as [`Default`]).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the `X-Content-Type-Options` header value.
    pub fn x_content_type_options(mut self, value: impl Into<String>) -> Self {
        self.x_content_type_options = Some(value.into());
        self
    }

    /// Disable the `X-Content-Type-Options` header.
    pub fn no_x_content_type_options(mut self) -> Self {
        self.x_content_type_options = None;
        self
    }

    /// Set the `X-Frame-Options` header value (e.g. `"DENY"`, `"SAMEORIGIN"`).
    pub fn x_frame_options(mut self, value: impl Into<String>) -> Self {
        self.x_frame_options = Some(value.into());
        self
    }

    /// Disable the `X-Frame-Options` header.
    pub fn no_x_frame_options(mut self) -> Self {
        self.x_frame_options = None;
        self
    }

    /// Set the `Referrer-Policy` header value.
    pub fn referrer_policy(mut self, value: impl Into<String>) -> Self {
        self.referrer_policy = Some(value.into());
        self
    }

    /// Disable the `Referrer-Policy` header.
    pub fn no_referrer_policy(mut self) -> Self {
        self.referrer_policy = None;
        self
    }

    /// Enable `Strict-Transport-Security` with the given configuration.
    pub fn hsts(mut self, config: HstsConfig) -> Self {
        self.hsts = Some(config);
        self
    }

    /// Disable `Strict-Transport-Security` (the default).
    pub fn no_hsts(mut self) -> Self {
        self.hsts = None;
        self
    }

    /// Set the `Content-Security-Policy` header value.
    pub fn content_security_policy(mut self, policy: impl Into<String>) -> Self {
        self.content_security_policy = Some(policy.into());
        self
    }

    /// Disable the `Content-Security-Policy` header (the default).
    pub fn no_content_security_policy(mut self) -> Self {
        self.content_security_policy = None;
        self
    }

    /// Build the flat list of `(HeaderName, HeaderValue)` pairs that this
    /// configuration produces.  Header values that fail to parse are silently
    /// dropped (this can only happen with programmer-supplied CSP strings
    /// containing control characters).
    pub(crate) fn build_headers(&self) -> Vec<(HeaderName, HeaderValue)> {
        let mut out: Vec<(HeaderName, HeaderValue)> = Vec::new();

        macro_rules! push {
            ($name:literal, $value:expr) => {
                if let Ok(hv) = HeaderValue::from_str($value) {
                    out.push((HeaderName::from_static($name), hv));
                }
            };
        }

        if let Some(ref v) = self.x_content_type_options {
            push!("x-content-type-options", v);
        }
        if let Some(ref v) = self.x_frame_options {
            push!("x-frame-options", v);
        }
        if let Some(ref v) = self.referrer_policy {
            push!("referrer-policy", v);
        }
        if let Some(ref hsts) = self.hsts {
            push!("strict-transport-security", &hsts.header_value());
        }
        if let Some(ref csp) = self.content_security_policy {
            push!("content-security-policy", csp);
        }

        out
    }
}

// ---------------------------------------------------------------------------
// SecurityHeadersLayer  (implements tower::Layer<S>)
// ---------------------------------------------------------------------------

/// Tower [`Layer`] that injects security response headers.
///
/// Constructed via [`security_headers_layer`].
///
/// [`Layer`]: tower::Layer
#[derive(Clone)]
pub struct SecurityHeadersLayer {
    headers: Arc<Vec<(HeaderName, HeaderValue)>>,
}

impl<S> Layer<S> for SecurityHeadersLayer {
    type Service = SecurityHeadersService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        SecurityHeadersService {
            inner,
            headers: Arc::clone(&self.headers),
        }
    }
}

// ---------------------------------------------------------------------------
// SecurityHeadersService  (implements tower::Service)
// ---------------------------------------------------------------------------

/// Tower [`Service`] produced by [`SecurityHeadersLayer`].
///
/// [`Service`]: tower::Service
#[derive(Clone)]
pub struct SecurityHeadersService<S> {
    inner: S,
    headers: Arc<Vec<(HeaderName, HeaderValue)>>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for SecurityHeadersService<S>
where
    S: Service<Request<ReqBody>, Response = Response<Body>> + Clone + Send + 'static,
    S::Error: Send,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = Response<Body>;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let headers = Arc::clone(&self.headers);
        let fut = self.inner.call(request);
        Box::pin(async move {
            let mut response = fut.await?;
            for (name, value) in headers.iter() {
                // `insert` intentionally overwrites any existing value so the
                // framework's opinion always wins over a handler-set value.
                response.headers_mut().insert(name.clone(), value.clone());
            }
            Ok(response)
        })
    }
}

// ---------------------------------------------------------------------------
// Public factory function
// ---------------------------------------------------------------------------

/// Create a security-headers [`Layer`] from a [`SecurityHeadersConfig`].
///
/// This is **opt-in**: only routers that explicitly add this layer will have
/// the security headers applied.  Routers without it are completely unaffected.
///
/// For the common case where secure defaults are sufficient, prefer the
/// zero-argument [`default_security_headers_layer`].
///
/// # Examples
///
/// ```rust,no_run
/// use rf_web::{security_headers_layer, SecurityHeadersConfig, HstsConfig};
/// use axum::Router;
///
/// // Secure defaults (X-Content-Type-Options, X-Frame-Options, Referrer-Policy)
/// let app: Router = Router::new()
///     .layer(security_headers_layer(SecurityHeadersConfig::default()));
///
/// // Production HTTPS — also enable HSTS + a restrictive CSP
/// let config = SecurityHeadersConfig::new()
///     .hsts(HstsConfig::default())
///     .content_security_policy("default-src 'self'");
/// let app: Router = Router::new()
///     .layer(security_headers_layer(config));
/// ```
///
/// [`Layer`]: tower::Layer
pub fn security_headers_layer(config: SecurityHeadersConfig) -> SecurityHeadersLayer {
    SecurityHeadersLayer {
        headers: Arc::new(config.build_headers()),
    }
}

/// Create a security-headers [`Layer`] with secure defaults — no configuration required.
///
/// This is equivalent to `security_headers_layer(SecurityHeadersConfig::default())` and
/// covers the most common case: add `X-Content-Type-Options: nosniff`,
/// `X-Frame-Options: DENY`, and `Referrer-Policy: no-referrer` to every response.
/// HSTS and CSP are opt-in via [`security_headers_layer`] with a customised config.
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::default_security_headers_layer;
/// use axum::Router;
///
/// let app: Router = Router::new()
///     .layer(default_security_headers_layer());
/// ```
///
/// [`Layer`]: tower::Layer
pub fn default_security_headers_layer() -> SecurityHeadersLayer {
    security_headers_layer(SecurityHeadersConfig::default())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    async fn build_app(config: SecurityHeadersConfig) -> Router {
        Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(security_headers_layer(config))
    }

    async fn build_app_no_security() -> Router {
        Router::new().route("/test", get(|| async { "OK" }))
    }

    fn get_request() -> Request<Body> {
        Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap()
    }

    // ------------------------------------------------------------------
    // Default config injects the three mandatory secure headers
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_default_x_content_type_options() {
        let app = build_app(SecurityHeadersConfig::default()).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers()
                .get("x-content-type-options")
                .and_then(|v| v.to_str().ok()),
            Some("nosniff"),
            "X-Content-Type-Options must be nosniff by default"
        );
    }

    #[tokio::test]
    async fn test_default_x_frame_options() {
        let app = build_app(SecurityHeadersConfig::default()).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert_eq!(
            resp.headers()
                .get("x-frame-options")
                .and_then(|v| v.to_str().ok()),
            Some("DENY"),
            "X-Frame-Options must be DENY by default"
        );
    }

    #[tokio::test]
    async fn test_default_referrer_policy() {
        let app = build_app(SecurityHeadersConfig::default()).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert_eq!(
            resp.headers()
                .get("referrer-policy")
                .and_then(|v| v.to_str().ok()),
            Some("no-referrer"),
            "Referrer-Policy must be no-referrer by default"
        );
    }

    // ------------------------------------------------------------------
    // Opt-in: without the layer the security headers are absent
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_no_security_headers_without_layer() {
        let app = build_app_no_security().await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        assert!(
            resp.headers().get("x-content-type-options").is_none(),
            "X-Content-Type-Options must NOT be present without the layer"
        );
        assert!(
            resp.headers().get("x-frame-options").is_none(),
            "X-Frame-Options must NOT be present without the layer"
        );
        assert!(
            resp.headers().get("referrer-policy").is_none(),
            "Referrer-Policy must NOT be present without the layer"
        );
    }

    // ------------------------------------------------------------------
    // HSTS is off by default
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_hsts_absent_by_default() {
        let app = build_app(SecurityHeadersConfig::default()).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert!(
            resp.headers().get("strict-transport-security").is_none(),
            "HSTS must NOT be present by default (opt-in)"
        );
    }

    // ------------------------------------------------------------------
    // HSTS with explicit config
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_hsts_with_config() {
        let config = SecurityHeadersConfig::new().hsts(HstsConfig {
            max_age_secs: 63_072_000,
            include_subdomains: true,
            preload: true,
        });
        let app = build_app(config).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        let hsts = resp
            .headers()
            .get("strict-transport-security")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            hsts.contains("max-age=63072000"),
            "HSTS must contain the configured max-age"
        );
        assert!(
            hsts.contains("includeSubDomains"),
            "HSTS must contain includeSubDomains"
        );
        assert!(hsts.contains("preload"), "HSTS must contain preload");
    }

    // ------------------------------------------------------------------
    // CSP is off by default; can be enabled
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_csp_absent_by_default() {
        let app = build_app(SecurityHeadersConfig::default()).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert!(
            resp.headers().get("content-security-policy").is_none(),
            "CSP must NOT be present by default (opt-in)"
        );
    }

    #[tokio::test]
    async fn test_csp_with_policy() {
        let config =
            SecurityHeadersConfig::new().content_security_policy("default-src 'self'");
        let app = build_app(config).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert_eq!(
            resp.headers()
                .get("content-security-policy")
                .and_then(|v| v.to_str().ok()),
            Some("default-src 'self'"),
            "CSP must match the configured policy"
        );
    }

    // ------------------------------------------------------------------
    // Builder: X-Frame-Options customised to SAMEORIGIN
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_x_frame_options_sameorigin() {
        let config = SecurityHeadersConfig::new().x_frame_options("SAMEORIGIN");
        let app = build_app(config).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert_eq!(
            resp.headers()
                .get("x-frame-options")
                .and_then(|v| v.to_str().ok()),
            Some("SAMEORIGIN")
        );
    }

    // ------------------------------------------------------------------
    // Builder: disable individual headers
    // ------------------------------------------------------------------

    #[tokio::test]
    async fn test_disable_x_frame_options() {
        let config = SecurityHeadersConfig::new().no_x_frame_options();
        let app = build_app(config).await;
        let resp = app.oneshot(get_request()).await.unwrap();
        assert!(
            resp.headers().get("x-frame-options").is_none(),
            "X-Frame-Options must be absent when disabled"
        );
        // Other headers still present
        assert!(resp.headers().get("x-content-type-options").is_some());
    }
}
