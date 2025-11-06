//! Prometheus metrics HTTP endpoint

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use prometheus::{Encoder, TextEncoder};

/// Metrics endpoint handler for Prometheus
///
/// Returns metrics in Prometheus text exposition format.
/// Mount this at `/metrics` in your router:
///
/// ```rust,no_run
/// use axum::{Router, routing::get};
/// use foundry_api::metrics_endpoint::metrics_handler;
///
/// let app = Router::new()
///     .route("/metrics", get(metrics_handler));
/// ```
pub async fn metrics_handler() -> Response {
    match gather_metrics() {
        Ok(metrics) => (StatusCode::OK, metrics).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to gather metrics: {}", e),
        )
            .into_response(),
    }
}

/// Gather all registered Prometheus metrics
fn gather_metrics() -> Result<String, String> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    encoder
        .encode(&metric_families, &mut buffer)
        .map_err(|e| format!("Failed to encode metrics: {}", e))?;

    String::from_utf8(buffer).map_err(|e| format!("Failed to convert metrics to string: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_handler() {
        let response = metrics_handler().await;
        let status = response.status();
        assert_eq!(status, StatusCode::OK);
    }

    #[test]
    fn test_gather_metrics() {
        let result = gather_metrics();
        assert!(result.is_ok());
    }
}
