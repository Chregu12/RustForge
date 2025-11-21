# ADR-004: Observability & Tracing

**Status:** Accepted
**Date:** 2025-11-08
**Deciders:** Lead Architect

## Context

Production-APIs benötigen:
- Request-Tracing (trace_id, span_id)
- Structured Logging (JSON-Format)
- Performance Metrics (latency, throughput)
- Distributed Tracing (Service-übergreifend)

## Decision

**tracing + OpenTelemetry** als Observability-Stack

### Architektur:

```rust
use tracing::{info, instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use opentelemetry::sdk::trace::Tracer;

// Initialization
tracing_subscriber::registry()
    .with(tracing_subscriber::fmt::layer().json())
    .with(tracing_opentelemetry::layer().with_tracer(tracer))
    .init();

// Usage
#[instrument(skip(db))]
async fn create_user(db: &DbPool, email: &str) -> Result<User> {
    info!(email, "Creating user");
    // ... implementation
}
```

### Tower Integration:

```rust
use tower_http::trace::{TraceLayer, DefaultMakeSpan};

Router::new()
    .route("/users", post(create_user))
    .layer(
        TraceLayer::new_for_http()
            .make_span_with(DefaultMakeSpan::new().include_headers(true))
    )
```

### Alternativen (abgelehnt):

**log + env_logger:**
- ❌ Kein Span-Kontext
- ❌ Keine distributed tracing
- ❌ Schwieriger zu parsen

**slog:**
- ❌ Weniger aktive Community
- ❌ Kein OpenTelemetry-Support

## Consequences

**Positiv:**
- ✅ Request-Trace über Services hinweg
- ✅ Structured Logging (Splunk/ELK-ready)
- ✅ Zero-cost bei disabled spans
- ✅ Industry-Standard (OpenTelemetry)

**Negativ:**
- ❌ Async runtime required (Tokio)
- ❌ Setup-Komplexität (Subscriber-Layers)

## Implementation

```rust
// rf-observability/src/lib.rs
pub fn init_tracing(service_name: &str, env: &str) -> Result<()> {
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .install_simple()?;

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| format!("{}=debug,tower_http=debug", service_name).into())
        )
        .with(tracing_subscriber::fmt::layer().json())
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .init();

    Ok(())
}

// Middleware für trace_id injection
pub struct TraceIdLayer;

impl<S> tower::Layer<S> for TraceIdLayer {
    type Service = TraceIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        TraceIdMiddleware { inner }
    }
}
```

### Metrics (Prometheus):

```rust
use prometheus::{IntCounter, Histogram, register_int_counter, register_histogram};

lazy_static! {
    static ref HTTP_REQUESTS_TOTAL: IntCounter =
        register_int_counter!("http_requests_total", "Total HTTP requests").unwrap();

    static ref HTTP_REQUEST_DURATION: Histogram =
        register_histogram!("http_request_duration_seconds", "HTTP request duration").unwrap();
}
```
