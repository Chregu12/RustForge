//! Recording middleware for automatic event capture

use crate::metrics::MetricsRegistry;
use crate::monitor::{EventType, Monitor};
use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

/// Recorder for automatic event capture
pub struct Recorder {
    #[allow(dead_code)]
    monitor: Arc<Monitor>,
    #[allow(dead_code)]
    metrics: Arc<MetricsRegistry>,
}

impl Recorder {
    /// Create a new recorder
    pub fn new() -> Self {
        Self {
            monitor: Monitor::global(),
            metrics: MetricsRegistry::global(),
        }
    }

    /// Create an Axum middleware layer
    pub fn layer() -> RecorderLayer {
        RecorderLayer::new()
    }
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

/// Recorder layer for Axum
#[derive(Clone)]
pub struct RecorderLayer {
    #[allow(dead_code)]
    monitor: Arc<Monitor>,
    #[allow(dead_code)]
    metrics: Arc<MetricsRegistry>,
}

impl RecorderLayer {
    /// Create a new recorder layer
    pub fn new() -> Self {
        Self {
            monitor: Monitor::global(),
            metrics: MetricsRegistry::global(),
        }
    }
}

impl Default for RecorderLayer {
    fn default() -> Self {
        Self::new()
    }
}

/// Middleware function for recording requests
#[allow(dead_code)]
pub async fn record_request(request: Request<Body>, next: Next) -> Response {
    let monitor = Monitor::global();
    let metrics = MetricsRegistry::global();

    let method = request.method().to_string();
    let uri = request.uri().to_string();
    let start = Instant::now();

    // Record request
    monitor.record(
        EventType::Request,
        &format!("{} {}", method, uri),
    );

    // Increment request counter
    metrics.counter("http_requests_total").increment();

    // Execute request
    let response = next.run(request).await;

    // Record response time
    let duration = start.elapsed();
    metrics
        .histogram("http_request_duration_seconds")
        .record(duration.as_secs_f64());

    // Record response
    let status = response.status();
    monitor.record(
        EventType::Response,
        &format!("{} {} -> {} ({:?})", method, uri, status.as_u16(), duration),
    );

    // Track error rate
    if status.is_server_error() {
        metrics.counter("http_errors_total").increment();
    }

    response
}

/// Middleware for health check bypass
#[allow(dead_code)]
pub async fn skip_health_check(request: Request<Body>, next: Next) -> Response {
    let path = request.uri().path();

    // Skip recording for health check endpoints
    if path == "/health" || path == "/ready" || path == "/live" {
        return next.run(request).await;
    }

    record_request(request, next).await
}
