//! OpenTelemetry telemetry initialization and management

use crate::config::OtelConfig;
use anyhow::{Context, Result};
use opentelemetry::{
    global,
    trace::TracerProvider as _,
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{Config, RandomIdGenerator, Sampler},
    Resource,
};
use std::time::Duration;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Initialize OpenTelemetry telemetry
pub fn init_telemetry(config: &OtelConfig) -> Result<()> {
    // TODO: Full OpenTelemetry integration requires API compatibility updates
    // For now, telemetry is handled via tracing macros and Prometheus metrics

    info!(
        endpoint = %config.endpoint,
        sample_rate = config.sample_rate,
        "OpenTelemetry configuration loaded (telemetry via tracing)"
    );

    Ok(())
}

/// Shutdown OpenTelemetry and flush all pending spans
pub async fn shutdown_telemetry() -> Result<()> {
    info!("Shutting down OpenTelemetry...");

    // Shutdown global tracer provider
    global::shutdown_tracer_provider();

    info!("OpenTelemetry shutdown complete");
    Ok(())
}

/// Initialize tracing subscriber with OpenTelemetry layer
pub fn init_tracing_subscriber(env_filter: &str, json_format: bool) -> Result<()> {
    let filter = EnvFilter::try_new(env_filter)
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Simplified: JSON/text logging without OpenTelemetry layer for now
    // Full integration requires compatible OpenTelemetry tracing-opentelemetry versions
    if json_format {
        Registry::default()
            .with(filter)
            .with(tracing_subscriber::fmt::layer().json())
            .try_init()
            .context("Failed to initialize tracing subscriber")?;
    } else {
        Registry::default()
            .with(filter)
            .with(tracing_subscriber::fmt::layer())
            .try_init()
            .context("Failed to initialize tracing subscriber")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_telemetry_config() {
        let config = OtelConfig {
            enabled: true,
            endpoint: "http://localhost:4317".to_string(),
            use_tls: false,
            sample_rate: 1.0,
            timeout_seconds: 10,
            batch_config: Default::default(),
        };

        assert_eq!(config.endpoint, "http://localhost:4317");
        assert_eq!(config.sample_rate, 1.0);
    }
}
