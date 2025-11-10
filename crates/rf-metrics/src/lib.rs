//! Metrics and monitoring for RustForge
//!
//! This crate provides Prometheus-compatible metrics:
//! - HTTP request metrics (duration, count, status codes)
//! - Custom counters and gauges
//! - Histograms for timing
//! - Metrics endpoint for Prometheus scraping

use axum::{
    extract::MatchedPath,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Router,
};
use prometheus::{
    register_counter_vec, register_gauge, register_histogram_vec, CounterVec, Encoder, Gauge,
    HistogramVec, TextEncoder,
};
use std::time::Instant;

lazy_static::lazy_static! {
    /// HTTP request duration histogram
    pub static ref HTTP_REQUEST_DURATION: HistogramVec = register_histogram_vec!(
        "http_request_duration_seconds",
        "HTTP request duration in seconds",
        &["method", "path", "status"]
    ).unwrap();

    /// HTTP request counter
    pub static ref HTTP_REQUEST_COUNT: CounterVec = register_counter_vec!(
        "http_requests_total",
        "Total number of HTTP requests",
        &["method", "path", "status"]
    ).unwrap();

    /// Active connections gauge
    pub static ref ACTIVE_CONNECTIONS: Gauge = register_gauge!(
        "active_connections",
        "Number of active connections"
    ).unwrap();
}

/// Metrics middleware for Axum
pub async fn metrics_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let start = Instant::now();
    let method = req.method().clone();
    let path = req
        .extensions()
        .get::<MatchedPath>()
        .map(|p| p.as_str().to_string())
        .unwrap_or_else(|| req.uri().path().to_string());

    ACTIVE_CONNECTIONS.inc();

    let response = next.run(req).await;

    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16().to_string();

    HTTP_REQUEST_DURATION
        .with_label_values(&[method.as_str(), &path, &status])
        .observe(duration);

    HTTP_REQUEST_COUNT
        .with_label_values(&[method.as_str(), &path, &status])
        .inc();

    ACTIVE_CONNECTIONS.dec();

    Ok(response)
}

/// Metrics endpoint handler
pub async fn metrics_handler() -> impl IntoResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        eprintln!("Failed to encode metrics: {}", e);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Failed to encode metrics".to_string(),
        );
    }

    let output = String::from_utf8(buffer).unwrap_or_else(|e| {
        eprintln!("Failed to convert metrics to string: {}", e);
        String::new()
    });

    (StatusCode::OK, output)
}

/// Create a router with metrics endpoint
pub fn metrics_router() -> Router {
    Router::new().route("/metrics", axum::routing::get(metrics_handler))
}

/// Custom counter for application-specific metrics
pub struct Counter {
    inner: prometheus::Counter,
}

impl Counter {
    /// Create a new counter
    pub fn new(name: &str, help: &str) -> Result<Self, prometheus::Error> {
        let counter = prometheus::register_counter!(name, help)?;
        Ok(Self { inner: counter })
    }

    /// Increment the counter by 1
    pub fn inc(&self) {
        self.inner.inc();
    }

    /// Increment the counter by a specific amount
    pub fn inc_by(&self, v: f64) {
        self.inner.inc_by(v);
    }

    /// Get the current value
    pub fn get(&self) -> f64 {
        self.inner.get()
    }
}

/// Custom gauge for application-specific metrics
pub struct CustomGauge {
    inner: Gauge,
}

impl CustomGauge {
    /// Create a new gauge
    pub fn new(name: &str, help: &str) -> Result<Self, prometheus::Error> {
        let gauge = prometheus::register_gauge!(name, help)?;
        Ok(Self { inner: gauge })
    }

    /// Set the gauge to a specific value
    pub fn set(&self, v: f64) {
        self.inner.set(v);
    }

    /// Increment the gauge
    pub fn inc(&self) {
        self.inner.inc();
    }

    /// Decrement the gauge
    pub fn dec(&self) {
        self.inner.dec();
    }

    /// Add to the gauge
    pub fn add(&self, v: f64) {
        self.inner.add(v);
    }

    /// Subtract from the gauge
    pub fn sub(&self, v: f64) {
        self.inner.sub(v);
    }

    /// Get the current value
    pub fn get(&self) -> f64 {
        self.inner.get()
    }
}

/// Histogram for timing operations
pub struct Histogram {
    inner: prometheus::Histogram,
}

impl Histogram {
    /// Create a new histogram
    pub fn new(name: &str, help: &str) -> Result<Self, prometheus::Error> {
        let histogram = prometheus::register_histogram!(name, help)?;
        Ok(Self { inner: histogram })
    }

    /// Observe a value
    pub fn observe(&self, v: f64) {
        self.inner.observe(v);
    }

    /// Time an operation
    pub fn start_timer(&self) -> prometheus::HistogramTimer {
        self.inner.start_timer()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, middleware};

    #[test]
    fn test_counter_creation() {
        let counter = Counter::new("test_counter_1", "Test counter").unwrap();
        assert_eq!(counter.get(), 0.0);

        counter.inc();
        assert_eq!(counter.get(), 1.0);

        counter.inc_by(5.0);
        assert_eq!(counter.get(), 6.0);
    }

    #[test]
    fn test_gauge_creation() {
        let gauge = CustomGauge::new("test_gauge_1", "Test gauge").unwrap();
        assert_eq!(gauge.get(), 0.0);

        gauge.set(10.0);
        assert_eq!(gauge.get(), 10.0);

        gauge.inc();
        assert_eq!(gauge.get(), 11.0);

        gauge.dec();
        assert_eq!(gauge.get(), 10.0);

        gauge.add(5.0);
        assert_eq!(gauge.get(), 15.0);

        gauge.sub(3.0);
        assert_eq!(gauge.get(), 12.0);
    }

    #[test]
    fn test_histogram_creation() {
        let histogram = Histogram::new("test_histogram_1", "Test histogram").unwrap();
        histogram.observe(0.5);
        histogram.observe(1.0);
        histogram.observe(1.5);
        // Histogram doesn't have a simple get() method, but we can verify it doesn't panic
    }

    #[tokio::test]
    async fn test_metrics_router() {
        let app = metrics_router();

        // Create a test request
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_handler() {
        let response = metrics_handler().await.into_response();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn test_active_connections_gauge() {
        let initial = ACTIVE_CONNECTIONS.get();
        ACTIVE_CONNECTIONS.inc();
        assert_eq!(ACTIVE_CONNECTIONS.get(), initial + 1.0);
        ACTIVE_CONNECTIONS.dec();
        assert_eq!(ACTIVE_CONNECTIONS.get(), initial);
    }

    #[test]
    fn test_http_request_metrics() {
        // Test that metrics don't panic when recording
        HTTP_REQUEST_COUNT
            .with_label_values(&["GET", "/test", "200"])
            .inc();

        HTTP_REQUEST_DURATION
            .with_label_values(&["POST", "/api", "201"])
            .observe(0.123);

        // Verify we can gather metrics without error
        let families = prometheus::gather();
        assert!(!families.is_empty());
    }
}
