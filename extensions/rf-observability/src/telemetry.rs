//! OpenTelemetry telemetry initialization and management

use std::time::Duration;

use crate::config::OtelConfig;
use anyhow::{Context, Result};
use opentelemetry::{global, trace::TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    error::OTelSdkError,
    trace::{Sampler, SdkTracerProvider},
    Resource,
};
use parking_lot::Mutex;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

// Retain the provider for explicit shutdown and serialize subscriber installation.
static TELEMETRY_PROVIDER: Mutex<Option<SdkTracerProvider>> = Mutex::new(None);

/// Initialize a real OpenTelemetry OTLP tracing pipeline.
///
/// This constructs a batch [`SdkTracerProvider`]
/// with an OTLP/tonic span exporter pointed at `config.endpoint`, applies the configured
/// sampling ratio, installs it as the process-wide global tracer provider, and wires the
/// resulting tracer into the `tracing` subscriber via a `tracing-opentelemetry` layer so
/// that `tracing` spans are actually exported over OTLP.
///
/// After this returns, spans created through either the `tracing` macros or the
/// OpenTelemetry API are recorded and batched for export to the collector.
///
/// # Idempotency
///
/// Repeated calls (e.g. during tests or hot-reload) are a graceful no-op:
/// only the first invocation installs the global subscriber.  Subsequent calls
/// return `Ok(())` immediately without re-building the OTLP pipeline.
///
/// # Runtime
///
/// Must be called from within a Tokio runtime for the OTLP gRPC transport.
pub fn init_telemetry(config: &OtelConfig) -> Result<()> {
    let mut installed = TELEMETRY_PROVIDER.lock();
    if installed.is_some() {
        return Ok(());
    }
    // Clamp the sample rate into the valid [0.0, 1.0] probability range so a
    // misconfigured value can never panic or produce an invalid sampler.
    let sample_rate = config.sample_rate.clamp(0.0, 1.0);

    let service_name =
        std::env::var("OTEL_SERVICE_NAME").unwrap_or_else(|_| "rustforge".to_string());

    // Build the OTLP/tonic exporter targeting the configured collector endpoint.
    // The tonic transport connects lazily, so construction succeeds even when no
    // collector is currently reachable.
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(config.endpoint.clone())
        .with_timeout(Duration::from_secs(config.timeout_seconds))
        .build()
        .context("Failed to build OTLP span exporter")?;

    let provider = SdkTracerProvider::builder()
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            sample_rate,
        ))))
        .with_resource(
            Resource::builder_empty()
                .with_service_name(service_name)
                .build(),
        )
        .with_batch_exporter(exporter)
        .build();

    // Bridge `tracing` spans into the OpenTelemetry pipeline and install a
    // subscriber that both prints logs and exports spans over OTLP.
    let otel_layer = tracing_opentelemetry::layer().with_tracer(provider.tracer("rustforge"));
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(env_filter)
        .with(otel_layer)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .context("Failed to install OpenTelemetry tracing subscriber")?;

    global::set_tracer_provider(provider.clone());
    *installed = Some(provider);
    drop(installed);

    info!(
        endpoint = %config.endpoint,
        sample_rate,
        "OpenTelemetry OTLP tracing pipeline installed"
    );

    Ok(())
}

/// Shutdown OpenTelemetry and flush all pending spans
pub async fn shutdown_telemetry() -> Result<()> {
    info!("Shutting down OpenTelemetry...");

    let provider = TELEMETRY_PROVIDER.lock().clone();
    if let Some(provider) = provider {
        match tokio::task::spawn_blocking(move || provider.shutdown()).await? {
            Ok(()) | Err(OTelSdkError::AlreadyShutdown) => {}
            Err(error) => return Err(error).context("Failed to shut down OpenTelemetry"),
        }
    }

    info!("OpenTelemetry shutdown complete");
    Ok(())
}

/// Initialize tracing subscriber with OpenTelemetry layer
pub fn init_tracing_subscriber(env_filter: &str, json_format: bool) -> Result<()> {
    let filter = EnvFilter::try_new(env_filter).unwrap_or_else(|_| EnvFilter::new("info"));

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
