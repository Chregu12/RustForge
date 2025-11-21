//! Tracing middleware for request/response logging

use tower_http::classify::ServerErrorsAsFailures;
use tower_http::trace::{DefaultMakeSpan, DefaultOnResponse, TraceLayer};
use tracing::Level;

/// Create tracing middleware layer
///
/// Logs all HTTP requests and responses with:
/// - Request method, URI, version
/// - Response status, latency
/// - Request/response headers (in trace level)
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::tracing_layer;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(tracing_layer());
/// ```
pub fn tracing_layer() -> TraceLayer<tower_http::classify::SharedClassifier<ServerErrorsAsFailures>>
{
    TraceLayer::new_for_http()
        .make_span_with(
            DefaultMakeSpan::new()
                .level(Level::INFO)
                .include_headers(true),
        )
        .on_response(
            DefaultOnResponse::new()
                .level(Level::INFO)
                .include_headers(true),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_tracing_layer_works() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(tracing_layer());

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
