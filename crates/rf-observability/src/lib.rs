//! # Foundry Observability
//!
//! Comprehensive observability system for RustForge applications featuring:
//! - OpenTelemetry integration for distributed tracing
//! - Prometheus metrics collection and exposition
//! - Structured logging with trace correlation
//! - Health check endpoints
//! - Performance monitoring
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_observability::{ObservabilityConfig, init_observability};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = ObservabilityConfig::default();
//!     init_observability(config).await?;
//!
//!     // Your application code here
//!
//!     Ok(())
//! }
//! ```

pub mod config;
pub mod health;
pub mod logging;
pub mod metrics;
pub mod span_builder;
pub mod telemetry;
pub mod tracing_middleware;

pub use config::{ObservabilityConfig, OtelConfig, PrometheusConfig};
pub use health::{
    HealthCheckProvider, HealthCheckRegistry, HealthCheckResult, HealthState, HealthStatusReport,
};
pub use logging::{init_logging, LogEntry, StructuredLogger};
pub use metrics::{Metrics, METRICS};
pub use span_builder::SpanBuilder;
pub use telemetry::{init_telemetry, shutdown_telemetry};
pub use tracing_middleware::TracingMiddleware;

use anyhow::Result;
use tracing::info;

/// Initialize complete observability stack
pub async fn init_observability(config: ObservabilityConfig) -> Result<()> {
    // 1. Initialize tracing.
    //
    // When OpenTelemetry is enabled, `init_telemetry` installs a combined
    // subscriber (console formatting + OTLP span export), so it fully owns the
    // global tracing subscriber. Otherwise fall back to plain structured logging.
    // Only one global subscriber may be installed per process, so these paths
    // are mutually exclusive.
    if config.otel.enabled {
        init_telemetry(&config.otel)?;
        info!(
            "OpenTelemetry initialized with endpoint: {}",
            config.otel.endpoint
        );
    } else {
        init_logging(&config.log_level, config.log_json)?;
    }

    info!("Initializing observability system...");

    // 2. Prometheus metrics.
    //
    // The METRICS collectors are registered on the prometheus *default* global
    // registry (via lazy_static) so that `prometheus::gather()` — used by
    // rf-metrics' /metrics handler — automatically includes them.
    //
    // When `config.prometheus.enabled` is false we do not touch the collectors
    // (they are initialised lazily on first use), and we log a reminder that
    // the caller should not mount a /metrics endpoint.
    if config.prometheus.enabled {
        // Force lazy_static initialisation now so any registration error is
        // surfaced at startup rather than on first request.
        let _ = &*metrics::METRICS;
        info!(
            endpoint = %config.prometheus.endpoint_path,
            "Prometheus metrics registered; mount the /metrics handler at this path"
        );
    } else {
        info!(
            "Prometheus metrics disabled by config; \
             no /metrics endpoint will be served"
        );
    }

    info!("Observability system ready");
    Ok(())
}

/// Shutdown observability system gracefully
pub async fn shutdown_observability() -> Result<()> {
    info!("Shutting down observability system...");
    shutdown_telemetry().await?;
    info!("Observability system shutdown complete");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_observability_config_default() {
        // Test that default config can be created
        let config = ObservabilityConfig::default();
        // Default has otel enabled
        assert!(config.otel.enabled);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_otel_config_default() {
        // Test OpenTelemetry config defaults
        let otel_config = OtelConfig::default();
        // Default has otel enabled with localhost endpoint
        assert!(otel_config.enabled);
        assert_eq!(otel_config.endpoint, "http://localhost:4317");
    }
}
