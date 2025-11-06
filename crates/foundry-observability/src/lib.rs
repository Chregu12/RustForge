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
//! use foundry_observability::{ObservabilityConfig, init_observability};
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
pub mod logging;
pub mod metrics;
pub mod telemetry;
pub mod tracing_middleware;
pub mod health;
pub mod span_builder;

pub use config::{ObservabilityConfig, OtelConfig, PrometheusConfig};
pub use logging::{init_logging, StructuredLogger, LogEntry};
pub use metrics::{Metrics, METRICS};
pub use telemetry::{init_telemetry, shutdown_telemetry};
pub use tracing_middleware::TracingMiddleware;
pub use health::{HealthCheck, HealthStatus, HealthChecker};
pub use span_builder::SpanBuilder;

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
        info!("OpenTelemetry initialized with endpoint: {}", config.otel.endpoint);
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

    #[tokio::test]
    async fn test_observability_init() {
        let config = ObservabilityConfig {
            otel: OtelConfig {
                enabled: false,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = init_observability(config).await;
        assert!(result.is_ok());
    }
}
