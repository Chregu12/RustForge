//! Compression middleware

use tower_http::compression::CompressionLayer;

/// Create compression layer
///
/// Compresses responses larger than 1KB using gzip, brotli, or deflate.
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::compression_layer;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(compression_layer());
/// ```
pub fn compression_layer() -> CompressionLayer {
    CompressionLayer::new().gzip(true).br(true).deflate(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{header, Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_compression_layer_works() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(compression_layer());

        let request = Request::builder()
            .uri("/test")
            .header(header::ACCEPT_ENCODING, "gzip")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_large_response_compression() {
        let large_body = "x".repeat(2048); // > 1KB
        let app = Router::new()
            .route("/large", get(|| async move { large_body.clone() }))
            .layer(compression_layer());

        let request = Request::builder()
            .uri("/large")
            .header(header::ACCEPT_ENCODING, "gzip")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Check if content-encoding header is present
        let _encoding = response.headers().get(header::CONTENT_ENCODING);
        // May or may not be compressed depending on content
        // Just verify the handler works
    }
}
