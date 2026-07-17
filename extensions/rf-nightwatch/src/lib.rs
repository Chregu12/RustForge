//! rf-nightwatch - Laravel-style application monitoring for RustForge
//!
//! Provides real-time application monitoring, health checks, metrics,
//! and alerting capabilities.
//!
//! # Example
//!
//! ```rust,ignore
//! use rf_nightwatch::{Nightwatch, Check, Alert};
//!
//! // Register health checks
//! Nightwatch::check("database", || async {
//!     // Check database connectivity
//!     Check::pass("Connected to database")
//! });
//!
//! Nightwatch::check("cache", || async {
//!     // Check cache connectivity
//!     Check::pass("Redis is available")
//! });
//!
//! // Register alerts
//! Nightwatch::alert("high_error_rate")
//!     .when(|metrics| metrics.error_rate() > 0.05)
//!     .notify("email", "admin@example.com");
//!
//! // Record metrics
//! Nightwatch::counter("requests_total").increment();
//! Nightwatch::gauge("active_connections").set(42.0);
//! Nightwatch::histogram("response_time").record(0.125);
//!
//! // Start the monitoring server
//! Nightwatch::serve("0.0.0.0:9090").await;
//! ```

mod alert;
mod check;
mod config;
mod dashboard;
mod errors;
mod metrics;
mod monitor;
mod recorder;
mod routes;

pub use alert::{Alert, AlertBuilder, AlertLevel, AlertRegistry, Notification};
pub use check::{Check, CheckBuilder, CheckRegistry, CheckResult, CheckStatus};
pub use config::{get_config, set_config, NightwatchConfig};
pub use dashboard::Dashboard;
pub use errors::{NightwatchError, NightwatchResult};
pub use metrics::{Counter, Gauge, Histogram, MetricsRegistry};
pub use monitor::{Event, EventType, Monitor};
pub use recorder::Recorder;
pub use routes::nightwatch_routes;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

/// Nightwatch facade - Laravel-style static interface
pub struct Nightwatch;

impl Nightwatch {
    /// Register a health check
    pub fn check<F, Fut>(name: &str, check_fn: F) -> CheckBuilder
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CheckResult> + Send + 'static,
    {
        CheckBuilder::new(name, move || {
            Box::pin(check_fn()) as Pin<Box<dyn Future<Output = CheckResult> + Send>>
        })
    }

    /// Register an alert
    pub fn alert(name: &str) -> AlertBuilder {
        AlertBuilder::new(name)
    }

    /// Get a counter metric
    pub fn counter(name: &str) -> Counter {
        MetricsRegistry::global().counter(name)
    }

    /// Get a gauge metric
    pub fn gauge(name: &str) -> Gauge {
        MetricsRegistry::global().gauge(name)
    }

    /// Get a histogram metric
    pub fn histogram(name: &str) -> Histogram {
        MetricsRegistry::global().histogram(name)
    }

    /// Record an event
    pub fn record(event_type: EventType, message: &str) {
        Monitor::global().record(event_type, message);
    }

    /// Get the check registry
    pub fn checks() -> Arc<CheckRegistry> {
        CheckRegistry::global()
    }

    /// Get the alert registry
    pub fn alerts() -> Arc<AlertRegistry> {
        AlertRegistry::global()
    }

    /// Get the metrics registry
    pub fn metrics() -> Arc<MetricsRegistry> {
        MetricsRegistry::global()
    }

    /// Get the monitor
    pub fn monitor() -> Arc<Monitor> {
        Monitor::global()
    }

    /// Run all health checks
    pub async fn run_checks() -> Vec<(String, CheckResult)> {
        CheckRegistry::global().run_all().await
    }

    /// Check if all services are healthy
    pub async fn is_healthy() -> bool {
        let results = Self::run_checks().await;
        results.iter().all(|(_, r)| r.status == CheckStatus::Pass)
    }

    /// Start the monitoring server
    pub async fn serve(addr: &str) -> NightwatchResult<()> {
        let app = nightwatch_routes();
        let listener = tokio::net::TcpListener::bind(addr).await?;
        tracing::info!("Nightwatch monitoring server listening on {}", addr);
        axum::serve(listener, app).await?;
        Ok(())
    }

    /// Get the Axum router for embedding in an existing app
    pub fn router() -> axum::Router {
        nightwatch_routes()
    }
}
