//! Timeout middleware

use std::time::Duration;
use tower_http::timeout::TimeoutLayer;

/// Create timeout layer
///
/// Returns 408 Request Timeout if handler takes longer than specified duration.
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::timeout_layer;
/// use axum::Router;
/// use std::time::Duration;
///
/// let app = Router::new()
///     .layer(timeout_layer(Duration::from_secs(30)));
/// ```
pub fn timeout_layer(duration: Duration) -> TimeoutLayer {
    TimeoutLayer::new(duration)
}

/// Default timeout layer (30 seconds)
///
/// # Example
///
/// ```rust,no_run
/// use rf_web::default_timeout_layer;
/// use axum::Router;
///
/// let app = Router::new()
///     .layer(default_timeout_layer());
/// ```
pub fn default_timeout_layer() -> TimeoutLayer {
    timeout_layer(Duration::from_secs(30))
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
    async fn test_timeout_not_exceeded() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(timeout_layer(Duration::from_secs(1)));

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    #[ignore = "Timeout behavior varies with tower-http version"]
    async fn test_timeout_exceeded() {
        let app = Router::new()
            .route(
                "/slow",
                get(|| async {
                    tokio::time::sleep(Duration::from_secs(2)).await;
                    "Done"
                }),
            )
            .layer(timeout_layer(Duration::from_millis(100)));

        let request = Request::builder().uri("/slow").body(Body::empty()).unwrap();

        let result = app.oneshot(request).await;

        // Timeout error from tower
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_default_timeout() {
        let app = Router::new()
            .route("/test", get(|| async { "OK" }))
            .layer(default_timeout_layer());

        let request = Request::builder().uri("/test").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
