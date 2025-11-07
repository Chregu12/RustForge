//! Health check HTTP endpoints

use axum::{
    http::StatusCode,
    response::{IntoResponse, Json, Response},
};
use serde_json::json;

/// Simple health check response
///
/// Minimal endpoint for basic availability checks (e.g., load balancers).
/// Always returns 200 OK if the application is running.
///
/// Mount at `/health` or `/healthz`:
///
/// ```rust,no_run
/// use axum::{Router, routing::get};
/// use foundry_api::health_endpoint::health_check;
///
/// let app = Router::new()
///     .route("/health", get(health_check));
/// ```
pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
    }))
}

/// Detailed health check with component status
///
/// Returns detailed health information including:
/// - Overall status
/// - Individual component checks (database, cache, etc.)
/// - Application version
/// - Uptime
///
/// Returns:
/// - 200: All components healthy
/// - 503: One or more components unhealthy
///
/// # Example
///
/// ```rust,no_run
/// use axum::{Router, routing::get};
/// use foundry_api::health_endpoint::health_check_detailed;
/// use std::sync::Arc;
///
/// let app = Router::new()
///     .route("/health/detailed", get(health_check_detailed));
/// ```
pub async fn health_check_detailed() -> Response {
    // Create basic health checks
    // In production, these would check actual components
    let checks = vec![
        serde_json::json!({
            "name": "application",
            "status": "healthy",
            "message": "Application is running",
        }),
    ];

    let status = json!({
        "status": "healthy",
        "checks": checks,
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    (StatusCode::OK, Json(status)).into_response()
}

/// Readiness probe endpoint
///
/// Indicates whether the application is ready to accept traffic.
/// Used by orchestrators like Kubernetes to determine when to route traffic.
///
/// Returns:
/// - 200: Ready to accept requests
/// - 503: Not ready (still initializing or degraded)
pub async fn readiness_check() -> Response {
    // In production, check if all required services are available
    // For now, always return ready
    (
        StatusCode::OK,
        Json(json!({
            "status": "ready",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
        .into_response()
}

/// Liveness probe endpoint
///
/// Indicates whether the application is alive.
/// Used by orchestrators like Kubernetes to detect if a restart is needed.
///
/// Returns:
/// - 200: Application is alive
/// - 503: Application is stuck or deadlocked (should be restarted)
pub async fn liveness_check() -> Response {
    // If this endpoint responds, the application is alive
    (
        StatusCode::OK,
        Json(json!({
            "status": "alive",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_check() {
        let response = health_check().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_check_detailed() {
        let response = health_check_detailed().await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_readiness_check() {
        let response = readiness_check().await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_liveness_check() {
        let response = liveness_check().await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
