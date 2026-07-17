//! OpenTelemetry telemetry initialization and management

use std::sync::OnceLock;
use std::time::Duration;

use crate::config::OtelConfig;
use anyhow::{Context, Result};
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Registry};

/// Guard that ensures `init_telemetry` is a no-op on repeated calls.
/// The first call sets this to `()` after the subscriber is installed.
static TELEMETRY_INIT: OnceLock<()> = OnceLock::new();

/// Initialize a real OpenTelemetry OTLP tracing pipeline.
///
/// This constructs a batch [`TracerProvider`](opentelemetry_sdk::trace::TracerProvider)
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
/// Must be called from within a Tokio runtime — the batch span processor spawns a
/// background export task (this mirrors the `opentelemetry-otlp` `install_batch` contract).
pub fn init_telemetry(config: &OtelConfig) -> Result<()> {
    // Idempotency guard – return immediately on repeated calls so the process
    // does not attempt to install a second global subscriber (which would
    // error or panic).
    if TELEMETRY_INIT.get().is_some() {
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
    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(config.endpoint.clone())
        .with_timeout(Duration::from_secs(config.timeout_seconds));

    let trace_config = sdktrace::config()
        .with_sampler(sdktrace::Sampler::ParentBased(Box::new(
            sdktrace::Sampler::TraceIdRatioBased(sample_rate),
        )))
        .with_resource(Resource::new(vec![KeyValue::new(SERVICE_NAME, service_name)]));

    // `install_batch` builds the provider, installs it as the global tracer
    // provider, and returns a tracer bound to it.
    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(trace_config)
        .install_batch(runtime::Tokio)
        .context("Failed to build OTLP batch tracing pipeline")?;

    // Bridge `tracing` spans into the OpenTelemetry pipeline and install a
    // subscriber that both prints logs and exports spans over OTLP.
    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    Registry::default()
        .with(env_filter)
        .with(otel_layer)
        .with(tracing_subscriber::fmt::layer())
        .try_init()
        .context("Failed to install OpenTelemetry tracing subscriber")?;

    // Mark as initialised so subsequent calls short-circuit cleanly.
    TELEMETRY_INIT.set(()).ok();

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

    // Shutdown global tracer provider
    global::shutdown_tracer_provider();

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
