//! CORS middleware configuration

use axum::http::Method;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};

/// CORS configuration
///
/// # Example
///
/// ```rust
/// use rf_web::CorsConfig;
/// use axum::http::Method;
/// use std::time::Duration;
///
/// let config = CorsConfig {
///     allowed_origins: vec!["https://app.example.com".to_string()],
///     allowed_methods: vec![Method::GET, Method::POST],
///     allowed_headers: vec!["content-type".to_string(), "authorization".to_string()],
///     max_age: Some(Duration::from_secs(3600)),
/// };
/// ```
#[derive(Debug, Clone)]
pub struct CorsConfig {
    /// Allowed origins (use "*" for any origin)
    pub allowed_origins: Vec<String>,

    /// Allowed HTTP methods
    pub allowed_methods: Vec<Method>,

    /// Allowed request headers
    pub allowed_headers: Vec<String>,

    /// Max age for preflight cache
    pub max_age: Option<Duration>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::DELETE,
                Method::PATCH,
                Method::OPTIONS,
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-trace-id".to_string(),
            ],
            max_age: Some(Duration::from_secs(3600)),
        }
    }
}

/// Create CORS middleware layer
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::{cors_layer, CorsConfig};
/// use axum::Router;
///
/// let config = CorsConfig::default();
/// let app = Router::new()
///     .layer(cors_layer(config));
/// ```
pub fn cors_layer(config: CorsConfig) -> CorsLayer {
    let mut layer = CorsLayer::new();

    // Configure origins
    if config.allowed_origins.contains(&"*".to_string()) {
        layer = layer.allow_origin(Any);
    } else {
        let origins: Vec<_> = config
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        if !origins.is_empty() {
            layer = layer.allow_origin(origins);
        }
    }

    // Configure methods
    layer = layer.allow_methods(config.allowed_methods);

    // Configure headers
    let headers: Vec<_> = config
        .allowed_headers
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    if !headers.is_empty() {
        layer = layer.allow_headers(headers);
    }

    // Configure max age
    if let Some(max_age) = config.max_age {
        layer = layer.max_age(max_age);
    }

    layer
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{HeaderValue, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_cors_allow_all() {
        let config = CorsConfig::default();
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer(config));

        let request = Request::builder()
            .uri("/test")
            .header("origin", "https://example.com")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let allow_origin = response.headers().get("access-control-allow-origin");
        assert!(allow_origin.is_some());
    }

    #[tokio::test]
    async fn test_cors_specific_origins() {
        let config = CorsConfig {
            allowed_origins: vec!["https://app.example.com".to_string()],
            ..Default::default()
        };

        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer(config));

        let request = Request::builder()
            .uri("/test")
            .header("origin", "https://app.example.com")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_cors_preflight() {
        let config = CorsConfig::default();
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(cors_layer(config));

        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/test")
            .header("origin", "https://example.com")
            .header("access-control-request-method", "POST")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        let allow_methods = response.headers().get("access-control-allow-methods");
        assert!(allow_methods.is_some());
    }
}
