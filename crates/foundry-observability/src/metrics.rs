//! Prometheus metrics collection and exposition

use lazy_static::lazy_static;
use prometheus::{
    Counter, CounterVec, Gauge, GaugeVec, Histogram, HistogramOpts, HistogramVec, IntCounter,
    IntCounterVec, IntGauge, IntGaugeVec, Opts, Registry,
};
use std::time::Instant;

lazy_static! {
    /// Global metrics registry
    pub static ref REGISTRY: Registry = Registry::new();

    /// Global metrics instance
    pub static ref METRICS: Metrics = Metrics::new().expect("Failed to create metrics");
}

/// Central metrics collection
pub struct Metrics {
    // Command metrics
    pub commands_total: IntCounterVec,
    pub commands_success: IntCounterVec,
    pub commands_failed: IntCounterVec,
    pub command_duration_seconds: HistogramVec,

    // Cache metrics
    pub cache_hits: IntCounterVec,
    pub cache_misses: IntCounterVec,
    pub cache_size: IntGaugeVec,

    // Database metrics
    pub db_connections_active: IntGauge,
    pub db_connections_idle: IntGauge,
    pub db_query_duration_seconds: HistogramVec,
    pub db_queries_total: IntCounterVec,

    // HTTP metrics
    pub http_requests_total: IntCounterVec,
    pub http_request_duration_seconds: HistogramVec,
    pub http_requests_in_flight: IntGauge,

    // Queue metrics
    pub queue_size: IntGaugeVec,
    pub queue_messages_processed: IntCounterVec,
    pub queue_processing_duration_seconds: HistogramVec,

    // Application metrics
    pub app_info: IntCounterVec,
    pub app_uptime_seconds: IntGauge,

    // Error metrics
    pub errors_total: IntCounterVec,
}

impl Metrics {
    /// Create new metrics instance and register all collectors
    pub fn new() -> prometheus::Result<Self> {
        let metrics = Self {
            // Command metrics
            commands_total: IntCounterVec::new(
                Opts::new("rustforge_commands_total", "Total number of commands executed"),
                &["command_name", "status"],
            )?,
            commands_success: IntCounterVec::new(
                Opts::new(
                    "rustforge_commands_success_total",
                    "Total number of successful commands",
                ),
                &["command_name"],
            )?,
            commands_failed: IntCounterVec::new(
                Opts::new(
                    "rustforge_commands_failed_total",
                    "Total number of failed commands",
                ),
                &["command_name", "error_type"],
            )?,
            command_duration_seconds: HistogramVec::new(
                HistogramOpts::new(
                    "rustforge_command_duration_seconds",
                    "Command execution duration in seconds",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]),
                &["command_name"],
            )?,

            // Cache metrics
            cache_hits: IntCounterVec::new(
                Opts::new("rustforge_cache_hits_total", "Total number of cache hits"),
                &["cache_name"],
            )?,
            cache_misses: IntCounterVec::new(
                Opts::new("rustforge_cache_misses_total", "Total number of cache misses"),
                &["cache_name"],
            )?,
            cache_size: IntGaugeVec::new(
                Opts::new("rustforge_cache_size_bytes", "Current cache size in bytes"),
                &["cache_name"],
            )?,

            // Database metrics
            db_connections_active: IntGauge::new(
                "rustforge_db_connections_active",
                "Number of active database connections",
            )?,
            db_connections_idle: IntGauge::new(
                "rustforge_db_connections_idle",
                "Number of idle database connections",
            )?,
            db_query_duration_seconds: HistogramVec::new(
                HistogramOpts::new(
                    "rustforge_db_query_duration_seconds",
                    "Database query duration in seconds",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]),
                &["operation"],
            )?,
            db_queries_total: IntCounterVec::new(
                Opts::new(
                    "rustforge_db_queries_total",
                    "Total number of database queries",
                ),
                &["operation", "status"],
            )?,

            // HTTP metrics
            http_requests_total: IntCounterVec::new(
                Opts::new(
                    "rustforge_http_requests_total",
                    "Total number of HTTP requests",
                ),
                &["method", "path", "status"],
            )?,
            http_request_duration_seconds: HistogramVec::new(
                HistogramOpts::new(
                    "rustforge_http_request_duration_seconds",
                    "HTTP request duration in seconds",
                )
                .buckets(vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
                &["method", "path"],
            )?,
            http_requests_in_flight: IntGauge::new(
                "rustforge_http_requests_in_flight",
                "Number of HTTP requests currently being processed",
            )?,

            // Queue metrics
            queue_size: IntGaugeVec::new(
                Opts::new("rustforge_queue_size", "Current queue size"),
                &["queue_name"],
            )?,
            queue_messages_processed: IntCounterVec::new(
                Opts::new(
                    "rustforge_queue_messages_processed_total",
                    "Total number of queue messages processed",
                ),
                &["queue_name", "status"],
            )?,
            queue_processing_duration_seconds: HistogramVec::new(
                HistogramOpts::new(
                    "rustforge_queue_processing_duration_seconds",
                    "Queue message processing duration in seconds",
                )
                .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0, 30.0]),
                &["queue_name"],
            )?,

            // Application metrics
            app_info: IntCounterVec::new(
                Opts::new("rustforge_app_info", "Application information"),
                &["version", "environment"],
            )?,
            app_uptime_seconds: IntGauge::new(
                "rustforge_app_uptime_seconds",
                "Application uptime in seconds",
            )?,

            // Error metrics
            errors_total: IntCounterVec::new(
                Opts::new("rustforge_errors_total", "Total number of errors"),
                &["error_type", "component"],
            )?,
        };

        // Register all metrics
        REGISTRY.register(Box::new(metrics.commands_total.clone()))?;
        REGISTRY.register(Box::new(metrics.commands_success.clone()))?;
        REGISTRY.register(Box::new(metrics.commands_failed.clone()))?;
        REGISTRY.register(Box::new(metrics.command_duration_seconds.clone()))?;

        REGISTRY.register(Box::new(metrics.cache_hits.clone()))?;
        REGISTRY.register(Box::new(metrics.cache_misses.clone()))?;
        REGISTRY.register(Box::new(metrics.cache_size.clone()))?;

        REGISTRY.register(Box::new(metrics.db_connections_active.clone()))?;
        REGISTRY.register(Box::new(metrics.db_connections_idle.clone()))?;
        REGISTRY.register(Box::new(metrics.db_query_duration_seconds.clone()))?;
        REGISTRY.register(Box::new(metrics.db_queries_total.clone()))?;

        REGISTRY.register(Box::new(metrics.http_requests_total.clone()))?;
        REGISTRY.register(Box::new(metrics.http_request_duration_seconds.clone()))?;
        REGISTRY.register(Box::new(metrics.http_requests_in_flight.clone()))?;

        REGISTRY.register(Box::new(metrics.queue_size.clone()))?;
        REGISTRY.register(Box::new(metrics.queue_messages_processed.clone()))?;
        REGISTRY.register(Box::new(metrics.queue_processing_duration_seconds.clone()))?;

        REGISTRY.register(Box::new(metrics.app_info.clone()))?;
        REGISTRY.register(Box::new(metrics.app_uptime_seconds.clone()))?;

        REGISTRY.register(Box::new(metrics.errors_total.clone()))?;

        // Set application info
        metrics
            .app_info
            .with_label_values(&[
                env!("CARGO_PKG_VERSION"),
                &std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string()),
            ])
            .inc();

        Ok(metrics)
    }

    /// Record a command execution
    pub fn record_command(&self, name: &str, duration: std::time::Duration, success: bool) {
        self.commands_total
            .with_label_values(&[name, if success { "success" } else { "failed" }])
            .inc();

        if success {
            self.commands_success.with_label_values(&[name]).inc();
        }

        self.command_duration_seconds
            .with_label_values(&[name])
            .observe(duration.as_secs_f64());
    }

    /// Record a command failure
    pub fn record_command_error(&self, name: &str, error_type: &str) {
        self.commands_failed
            .with_label_values(&[name, error_type])
            .inc();
        self.errors_total
            .with_label_values(&[error_type, "command"])
            .inc();
    }

    /// Record cache hit
    pub fn record_cache_hit(&self, cache_name: &str) {
        self.cache_hits.with_label_values(&[cache_name]).inc();
    }

    /// Record cache miss
    pub fn record_cache_miss(&self, cache_name: &str) {
        self.cache_misses.with_label_values(&[cache_name]).inc();
    }

    /// Update cache size
    pub fn set_cache_size(&self, cache_name: &str, size: i64) {
        self.cache_size.with_label_values(&[cache_name]).set(size);
    }

    /// Record HTTP request
    pub fn record_http_request(
        &self,
        method: &str,
        path: &str,
        status: u16,
        duration: std::time::Duration,
    ) {
        self.http_requests_total
            .with_label_values(&[method, path, &status.to_string()])
            .inc();

        self.http_request_duration_seconds
            .with_label_values(&[method, path])
            .observe(duration.as_secs_f64());
    }
}

/// Timer guard for automatic duration recording
pub struct TimerGuard {
    start: Instant,
    histogram: Histogram,
}

impl TimerGuard {
    pub fn new(histogram: Histogram) -> Self {
        Self {
            start: Instant::now(),
            histogram,
        }
    }
}

impl Drop for TimerGuard {
    fn drop(&mut self) {
        let duration = self.start.elapsed();
        self.histogram.observe(duration.as_secs_f64());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        assert!(metrics.is_ok());
    }

    #[test]
    fn test_record_command() {
        let metrics = METRICS.clone();
        metrics.record_command("test_command", std::time::Duration::from_millis(100), true);

        let total = metrics
            .commands_total
            .with_label_values(&["test_command", "success"])
            .get();
        assert!(total > 0);
    }
}
