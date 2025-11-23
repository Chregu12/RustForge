# Foundry Observability

Complete observability solution for RustForge applications featuring OpenTelemetry distributed tracing, Prometheus metrics, structured logging, and health checks.

## Features

- **OpenTelemetry Integration**: Full distributed tracing with W3C Trace Context propagation
- **Prometheus Metrics**: Comprehensive metrics collection for commands, HTTP, database, cache, and queues
- **Structured Logging**: JSON and human-readable logging with trace correlation
- **Health Checks**: Liveness and readiness probes for orchestrators like Kubernetes
- **Grafana Dashboards**: Pre-built performance monitoring dashboards
- **Alerting Rules**: Production-ready Prometheus alerting rules

## Quick Start

### Basic Usage

```rust
use foundry_observability::{init_observability, ObservabilityConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize with default configuration
    let config = ObservabilityConfig::default();
    init_observability(config).await?;

    // Your application code here

    Ok(())
}
```

### With Axum HTTP Server

```rust
use axum::{Router, routing::get, middleware};
use foundry_observability::{
    init_observability, ObservabilityConfig, TracingMiddleware,
    metrics_handler, health_check,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize observability
    let config = ObservabilityConfig::from_env();
    init_observability(config).await?;

    // Build router with tracing middleware
    let app = Router::new()
        .route("/api/users", get(list_users))
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_check))
        .layer(middleware::from_fn(TracingMiddleware::middleware));

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;

    Ok(())
}
```

## Configuration

### Environment Variables

```bash
# OpenTelemetry
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=rustforge
OTEL_TRACES_SAMPLER_ARG=1.0  # Sample rate (0.0 to 1.0)

# Prometheus
PROMETHEUS_ENABLED=true
PROMETHEUS_ENDPOINT=/metrics

# Logging
RUST_LOG=info
LOG_JSON=false  # Set to true for production

# Environment
ENVIRONMENT=production
```

### Programmatic Configuration

```rust
use foundry_observability::{ObservabilityConfig, OtelConfig};

let config = ObservabilityConfig {
    otel: OtelConfig {
        enabled: true,
        endpoint: "http://localhost:4317".to_string(),
        sample_rate: 1.0,
        ..Default::default()
    },
    log_level: "info".to_string(),
    log_json: false,
    service_name: "my-app".to_string(),
    environment: "production".to_string(),
    ..Default::default()
};
```

## Metrics

### Available Metrics

#### Command Metrics
- `rustforge_commands_total`: Total commands executed
- `rustforge_commands_success_total`: Successful commands
- `rustforge_commands_failed_total`: Failed commands
- `rustforge_command_duration_seconds`: Command execution duration (histogram)

#### HTTP Metrics
- `rustforge_http_requests_total`: Total HTTP requests
- `rustforge_http_request_duration_seconds`: Request duration (histogram)
- `rustforge_http_requests_in_flight`: Current in-flight requests

#### Database Metrics
- `rustforge_db_connections_active`: Active database connections
- `rustforge_db_connections_idle`: Idle database connections
- `rustforge_db_query_duration_seconds`: Query duration (histogram)
- `rustforge_db_queries_total`: Total database queries

#### Cache Metrics
- `rustforge_cache_hits_total`: Cache hits
- `rustforge_cache_misses_total`: Cache misses
- `rustforge_cache_size_bytes`: Cache size in bytes

#### Queue Metrics
- `rustforge_queue_size`: Current queue size
- `rustforge_queue_messages_processed_total`: Processed messages
- `rustforge_queue_processing_duration_seconds`: Processing duration (histogram)

### Recording Metrics

```rust
use foundry_observability::METRICS;
use std::time::{Duration, Instant};

// Record command execution
let start = Instant::now();
// ... execute command ...
let duration = start.elapsed();
METRICS.record_command("my_command", duration, true);

// Record HTTP request
METRICS.record_http_request("GET", "/api/users", 200, duration);

// Record cache operations
METRICS.record_cache_hit("user_cache");
METRICS.record_cache_miss("user_cache");
METRICS.set_cache_size("user_cache", 1024000);
```

## Distributed Tracing

### W3C Trace Context

The `TracingMiddleware` automatically extracts and injects W3C Trace Context headers:

```
traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
```

### Creating Spans

```rust
use foundry_observability::SpanBuilder;

// Async operation
let result = SpanBuilder::new("process_user")
    .with_attribute("user_id", "123")
    .with_int_attribute("retry_count", 3)
    .in_span(async {
        // Your async code here
        42
    })
    .await;

// Sync operation
let result = SpanBuilder::new("calculate")
    .with_attribute("operation", "sum")
    .in_span_sync(|| {
        // Your sync code here
        42
    });
```

### Trace ID Correlation

Access current trace and span IDs for logging:

```rust
use foundry_observability::tracing_middleware::{current_trace_id, current_span_id};

if let Some(trace_id) = current_trace_id() {
    println!("Current trace: {}", trace_id);
}
```

## Health Checks

### Endpoints

```rust
use foundry_observability::{
    health_check,           // Simple health check
    health_check_detailed,  // Detailed component checks
    readiness_check,        // Kubernetes readiness probe
    liveness_check,         // Kubernetes liveness probe
};
```

### Custom Health Checkers

```rust
use foundry_observability::health::{HealthChecker, HealthCheck};
use async_trait::async_trait;

struct CustomChecker;

#[async_trait]
impl HealthChecker for CustomChecker {
    async fn check(&self) -> HealthCheck {
        // Perform your health check
        HealthCheck::healthy("custom", Duration::from_millis(10))
    }

    fn name(&self) -> &str {
        "custom"
    }
}
```

## Monitoring Stack Setup

### Docker Compose

Start the complete monitoring stack:

```bash
cd observability/
docker-compose up -d
```

This starts:
- **Jaeger** (http://localhost:16686) - Distributed tracing UI
- **Prometheus** (http://localhost:9090) - Metrics storage
- **Grafana** (http://localhost:3001) - Dashboards (admin/admin)
- **OpenTelemetry Collector** - Trace/metrics aggregation
- **Alertmanager** (http://localhost:9093) - Alert management

### Accessing Dashboards

1. **Jaeger UI**: http://localhost:16686
   - View distributed traces
   - Analyze request flows
   - Identify performance bottlenecks

2. **Grafana**: http://localhost:3001 (admin/admin)
   - Pre-built RustForge dashboard
   - Real-time metrics visualization
   - Custom queries and panels

3. **Prometheus**: http://localhost:9090
   - Query metrics directly
   - View alerting rules
   - Check targets health

## Alerting

### Alert Rules

Pre-configured alerts in `observability/prometheus/alerts.yml`:

- **HighCommandErrorRate**: Command error rate > 5%
- **SlowCommandExecution**: p95 latency > 10s
- **HighHTTPErrorRate**: HTTP 5xx rate > 5%
- **DatabaseConnectionPoolExhausted**: < 2 idle connections
- **QueueBacklogBuilding**: Queue size > 1000
- **ApplicationDown**: Service unreachable

### Configure Alertmanager

Edit `observability/alertmanager/config.yml` to configure:
- Slack notifications
- PagerDuty integration
- Email alerts
- Alert routing

## Examples

See the `examples/` directory:

- `basic_usage.rs`: Simple metrics and logging
- `axum_integration.rs`: Full HTTP server with observability

Run examples:

```bash
cargo run --example basic_usage
cargo run --example axum_integration
```

## Production Deployment

### Kubernetes

```yaml
apiVersion: v1
kind: Service
metadata:
  name: rustforge
  annotations:
    prometheus.io/scrape: "true"
    prometheus.io/port: "3000"
    prometheus.io/path: "/metrics"
spec:
  selector:
    app: rustforge
  ports:
    - port: 3000
---
apiVersion: v1
kind: Pod
metadata:
  name: rustforge
spec:
  containers:
    - name: rustforge
      image: rustforge:latest
      ports:
        - containerPort: 3000
      env:
        - name: OTEL_EXPORTER_OTLP_ENDPOINT
          value: "http://otel-collector:4317"
        - name: RUST_LOG
          value: "info"
        - name: LOG_JSON
          value: "true"
      livenessProbe:
        httpGet:
          path: /health/live
          port: 3000
      readinessProbe:
        httpGet:
          path: /health/ready
          port: 3000
```

### Best Practices

1. **Enable JSON logging in production**: `LOG_JSON=true`
2. **Adjust sample rates for high traffic**: `OTEL_TRACES_SAMPLER_ARG=0.1`
3. **Use separate collectors per environment**
4. **Configure appropriate alert thresholds**
5. **Set resource limits on metrics retention**
6. **Enable authentication for dashboards**

## Architecture

```
┌─────────────────┐
│   Application   │
│   (RustForge)   │
└────────┬────────┘
         │
         ├─────────────┐
         │             │
         v             v
    ┌────────┐    ┌─────────────┐
    │ Traces │    │   Metrics   │
    │ (OTLP) │    │ (Prometheus)│
    └───┬────┘    └──────┬──────┘
        │                │
        v                v
┌───────────────┐  ┌──────────┐
│ OTel Collector│  │Prometheus│
└───────┬───────┘  └────┬─────┘
        │               │
        v               v
    ┌───────┐      ┌─────────┐
    │Jaeger │      │ Grafana │
    └───────┘      └─────────┘
```

## License

MIT OR Apache-2.0
