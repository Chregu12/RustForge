//! Router builder for ergonomic application setup

use crate::middleware::{compression_layer, cors_layer, timeout_layer, tracing_layer, CorsConfig};
use axum::{routing::MethodRouter, Router};
use std::time::Duration;

/// Builder for creating routers with standard middleware stack
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::RouterBuilder;
/// use axum::routing::get;
///
/// # async fn handler() -> &'static str { "OK" }
/// let app = RouterBuilder::new()
///     .route("/health", get(handler))
///     .with_tracing(true)
///     .with_cors(true)
///     .with_compression(true)
///     .build();
/// ```
pub struct RouterBuilder {
    router: Router,
    enable_tracing: bool,
    enable_cors: bool,
    enable_compression: bool,
    enable_timeout: bool,
    timeout_duration: Duration,
    cors_config: CorsConfig,
}

impl RouterBuilder {
    /// Create a new RouterBuilder
    pub fn new() -> Self {
        Self {
            router: Router::new(),
            enable_tracing: true,
            enable_cors: true,
            enable_compression: true,
            enable_timeout: true,
            timeout_duration: Duration::from_secs(30),
            cors_config: CorsConfig::default(),
        }
    }

    /// Add a route to the router
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_web::RouterBuilder;
    /// use axum::routing::get;
    ///
    /// # async fn handler() -> &'static str { "OK" }
    /// let app = RouterBuilder::new()
    ///     .route("/users", get(handler))
    ///     .build();
    /// ```
    pub fn route(mut self, path: &str, method_router: MethodRouter) -> Self {
        self.router = self.router.route(path, method_router);
        self
    }

    /// Nest routes under a prefix
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_web::RouterBuilder;
    /// use axum::{Router, routing::get};
    ///
    /// # async fn handler() -> &'static str { "OK" }
    /// let api_routes = Router::new()
    ///     .route("/users", get(handler));
    ///
    /// let app = RouterBuilder::new()
    ///     .nest("/api/v1", api_routes)
    ///     .build();
    /// ```
    pub fn nest(mut self, path: &str, router: Router) -> Self {
        self.router = self.router.nest(path, router);
        self
    }

    /// Enable or disable tracing middleware
    pub fn with_tracing(mut self, enable: bool) -> Self {
        self.enable_tracing = enable;
        self
    }

    /// Enable or disable CORS middleware
    pub fn with_cors(mut self, enable: bool) -> Self {
        self.enable_cors = enable;
        self
    }

    /// Configure CORS settings
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_web::{RouterBuilder, CorsConfig};
    /// use axum::http::Method;
    ///
    /// let cors_config = CorsConfig {
    ///     allowed_origins: vec!["https://example.com".to_string()],
    ///     allowed_methods: vec![Method::GET, Method::POST],
    ///     allowed_headers: vec!["content-type".to_string()],
    ///     max_age: None,
    /// };
    ///
    /// let app = RouterBuilder::new()
    ///     .cors_config(cors_config)
    ///     .build();
    /// ```
    pub fn cors_config(mut self, config: CorsConfig) -> Self {
        self.cors_config = config;
        self
    }

    /// Enable or disable compression middleware
    pub fn with_compression(mut self, enable: bool) -> Self {
        self.enable_compression = enable;
        self
    }

    /// Enable or disable timeout middleware
    pub fn with_timeout(mut self, enable: bool) -> Self {
        self.enable_timeout = enable;
        self
    }

    /// Set timeout duration
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_web::RouterBuilder;
    /// use std::time::Duration;
    ///
    /// let app = RouterBuilder::new()
    ///     .timeout_duration(Duration::from_secs(60))
    ///     .build();
    /// ```
    pub fn timeout_duration(mut self, duration: Duration) -> Self {
        self.timeout_duration = duration;
        self
    }

    /// Build the router with configured middleware
    ///
    /// Middleware is applied in the following order (outermost to innermost):
    /// 1. RequestId (trace ID injection)
    /// 2. Tracing (logging)
    /// 3. Timeout
    /// 4. CORS
    /// 5. Compression
    pub fn build(self) -> Router {
        let mut router = self.router;

        // Apply layers individually (innermost to outermost)
        // Compression (innermost - compresses after handler)
        if self.enable_compression {
            router = router.layer(compression_layer());
        }

        // CORS
        if self.enable_cors {
            router = router.layer(cors_layer(self.cors_config));
        }

        // Timeout
        if self.enable_timeout {
            router = router.layer(timeout_layer(self.timeout_duration));
        }

        // Tracing
        if self.enable_tracing {
            router = router.layer(tracing_layer());
        }

        // RequestId (outermost - trace ID available to all layers)
        router = router.layer(axum::middleware::from_fn(
            crate::middleware::request_id::request_id_middleware,
        ));

        router
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_router_builder_basic() {
        let app = RouterBuilder::new()
            .route("/test", get(|| async { "OK" }))
            .build();

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_router_builder_with_tracing() {
        let app = RouterBuilder::new()
            .route("/test", get(|| async { "OK" }))
            .with_tracing(true)
            .build();

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Verify trace ID header is present
        let trace_id = response.headers().get("x-trace-id");
        assert!(trace_id.is_some());
    }

    #[tokio::test]
    async fn test_router_builder_nested_routes() {
        let api_routes = Router::new().route("/users", get(|| async { "Users" }));

        let app = RouterBuilder::new()
            .nest("/api/v1", api_routes)
            .route("/health", get(|| async { "OK" }))
            .build();

        let request = Request::builder()
            .uri("/api/v1/users")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        assert_eq!(body, "Users");
    }

    #[tokio::test]
    async fn test_router_builder_disabled_middleware() {
        let app = RouterBuilder::new()
            .route("/test", get(|| async { "OK" }))
            .with_tracing(false)
            .with_cors(false)
            .with_compression(false)
            .with_timeout(false)
            .build();

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Request ID middleware is always enabled
        let trace_id = response.headers().get("x-trace-id");
        assert!(trace_id.is_some());
    }

    #[tokio::test]
    async fn test_router_builder_custom_timeout() {
        let app = RouterBuilder::new()
            .route("/test", get(|| async { "OK" }))
            .timeout_duration(Duration::from_secs(60))
            .build();

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
