//! Integration proof that `init_telemetry` installs a *real* OpenTelemetry
//! pipeline (not a no-op) and that spans are actually recorded through the
//! globally-installed tracer provider.
//!
//! This is an offline construction proof: the OTLP/tonic exporter connects
//! lazily, so no live collector is required. We assert that after init the
//! global tracer produces spans with a *valid* span context (non-zero trace
//! and span ids), which the default no-op provider never does.
//!
//! Runs in its own test binary so it owns a clean global tracing-subscriber
//! slot (unit tests that install their own subscriber live in a separate
//! process).

use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, Tracer};

use rf_observability::OtelConfig;

#[tokio::test(flavor = "multi_thread")]
async fn init_telemetry_installs_real_global_tracer_and_records_spans() {
    let config = OtelConfig {
        enabled: true,
        endpoint: "http://localhost:4317".to_string(),
        use_tls: false,
        sample_rate: 1.0,
        timeout_seconds: 3,
        batch_config: Default::default(),
    };

    // Real construction + global install. Must succeed against an offline
    // endpoint (tonic connects lazily).
    rf_observability::init_telemetry(&config).expect("init_telemetry should build a real pipeline");

    // The global provider must now be the real SDK provider: a tracer obtained
    // from it produces spans with a valid (sampled, non-empty) span context.
    let tracer = global::tracer("telemetry-pipeline-probe");

    let recorded_valid = tracer.in_span("probe-span", |cx| {
        let span_ctx = cx.span().span_context().clone();
        // A no-op tracer returns an invalid/empty span context; a real SDK
        // tracer with a ratio(1.0) sampler always yields a valid one.
        assert!(
            span_ctx.is_valid(),
            "expected a valid span context from a real installed tracer provider"
        );
        assert!(
            span_ctx.is_sampled(),
            "span should be sampled with sample_rate = 1.0"
        );
        assert_ne!(
            span_ctx.trace_id(),
            opentelemetry::trace::TraceId::INVALID,
            "trace id must be non-zero for a real tracer"
        );
        assert_ne!(
            span_ctx.span_id(),
            opentelemetry::trace::SpanId::INVALID,
            "span id must be non-zero for a real tracer"
        );
        span_ctx.is_valid()
    });

    assert!(recorded_valid, "a real span should have been recorded");

    // Flush + tear down the batch processor cleanly (export attempts against the
    // offline collector simply fail fast; no live collector is needed).
    global::shutdown_tracer_provider();
}
