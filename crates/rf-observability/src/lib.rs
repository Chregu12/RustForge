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
pub use health::{HealthCheck, HealthChecker, HealthStatus};
pub use logging::{init_logging, LogEntry, StructuredLogger};
pub use metrics::{Metrics, METRICS};
pub use span_builder::SpanBuilder;
pub use telemetry::{init_telemetry, shutdown_telemetry};
pub use tracing_middleware::TracingMiddleware;

use anyhow::Result;
use tracing::info;

/// Initialize complete observability stack
pub async fn init_observability(config: ObservabilityConfig) -> Result<()> {
    // 1. Initialize structured logging
    init_logging(&config.log_level, config.log_json)?;

    info!("Initializing observability system...");

    // 2. Initialize OpenTelemetry if enabled
    if config.otel.enabled {
        init_telemetry(&config.otel)?;
        info!(
            "OpenTelemetry initialized with endpoint: {}",
            config.otel.endpoint
        );
    }

    // 3. Metrics are automatically initialized via lazy_static
    info!("Prometheus metrics initialized on /metrics endpoint");

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
