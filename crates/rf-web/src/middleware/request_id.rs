//! Request ID middleware for trace ID injection

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use rf_core::RequestContext;

/// Middleware to inject trace ID into request context and response headers
///
/// This middleware:
/// - Creates a unique RequestContext with trace ID for each request
/// - Stores it in request extensions for handlers to use
/// - Adds X-Trace-Id header to response
///
/// # Example
///
/// ```rust,no_run
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(axum::middleware::from_fn(rf_web::middleware::request_id::request_id_middleware));
/// ```
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    // Create RequestContext with unique trace ID
    let ctx = RequestContext::new(request.uri().path(), request.method().as_str());

    let trace_id = ctx.trace_id().to_string();

    // Store in extensions for handlers and other middleware
    request.extensions_mut().insert(ctx);

    // Process request
    let mut response = next.run(request).await;

    // Add trace ID to response headers
    if let Ok(header_value) = trace_id.parse() {
        response.headers_mut().insert("x-trace-id", header_value);
    }

    response
}

// Note: request_id_layer removed due to Axum 0.8 type complexity
// Use directly: router.layer(axum::middleware::from_fn(request_id_middleware))

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use axum::Router;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_trace_id_in_response_headers() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(axum::middleware::from_fn(request_id_middleware));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let trace_id = response.headers().get("x-trace-id");
        assert!(trace_id.is_some());
        assert!(!trace_id.unwrap().to_str().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_context_available_in_handler() {
        use axum::Extension;

        async fn handler(Extension(ctx): Extension<RequestContext>) -> String {
            format!("Trace: {}", ctx.trace_id())
        }

        let app = Router::new()
            .route("/test", get(handler))
            .layer(axum::middleware::from_fn(request_id_middleware));

        let request = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();

        assert!(body_str.starts_with("Trace: "));
        assert!(body_str.len() > 10);
    }

    #[tokio::test]
    async fn test_unique_trace_ids() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(axum::middleware::from_fn(request_id_middleware));

        let request1 = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let request2 = Request::builder()
            .uri("/test")
            .body(Body::empty())
            .unwrap();

        let response1 = app.clone().oneshot(request1).await.unwrap();
        let response2 = app.oneshot(request2).await.unwrap();

        let trace_id1 = response1
            .headers()
            .get("x-trace-id")
            .unwrap()
            .to_str()
            .unwrap();
        let trace_id2 = response2
            .headers()
            .get("x-trace-id")
            .unwrap()
            .to_str()
            .unwrap();

        assert_ne!(trace_id1, trace_id2);
    }
}
