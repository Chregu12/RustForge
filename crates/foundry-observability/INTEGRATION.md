# Observability Integration Guide

Complete guide for integrating the observability system into your RustForge application.

## Table of Contents

1. [Initial Setup](#initial-setup)
2. [HTTP Server Integration](#http-server-integration)
3. [Command Instrumentation](#command-instrumentation)
4. [Database Monitoring](#database-monitoring)
5. [Cache Monitoring](#cache-monitoring)
6. [Queue Monitoring](#queue-monitoring)
7. [Custom Metrics](#custom-metrics)
8. [Testing](#testing)

## Initial Setup

### 1. Add Dependencies

Add to your `Cargo.toml`:

```toml
[dependencies]
foundry-observability = { path = "../foundry-observability" }
tracing = "0.1"
```

### 2. Initialize in main.rs

```rust
use foundry_observability::{init_observability, shutdown_observability, ObservabilityConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration from environment
    let config = ObservabilityConfig::from_env();

    // Initialize observability stack
    init_observability(config).await?;

    // Run your application
    run_application().await?;

    // Graceful shutdown
    shutdown_observability().await?;

    Ok(())
}
```

### 3. Configure Environment

Create `.env` file:

```bash
OTEL_ENABLED=true
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317
OTEL_SERVICE_NAME=my-app
RUST_LOG=info
```

## HTTP Server Integration

### Add Tracing Middleware

```rust
use axum::{Router, routing::get, middleware};
use foundry_observability::{TracingMiddleware, metrics_handler, health_check};

fn create_app() -> Router {
    Router::new()
        // Your routes
        .route("/api/users", get(list_users))

        // Observability endpoints
        .route("/metrics", get(metrics_handler))
        .route("/health", get(health_check))
        .route("/health/live", get(liveness_check))
        .route("/health/ready", get(readiness_check))

        // Add tracing middleware (must be last)
        .layer(middleware::from_fn(TracingMiddleware::middleware))
}
```

### Instrument Handlers

```rust
use foundry_observability::{METRICS, SpanBuilder};
use std::time::Instant;
use tracing::{info, error};

async fn list_users() -> impl IntoResponse {
    // Create span for this operation
    SpanBuilder::new("list_users")
        .with_attribute("operation", "database_query")
        .in_span(async {
            let start = Instant::now();

            // Execute database query
            let users = match query_users().await {
                Ok(users) => {
                    // Record successful query
                    let duration = start.elapsed();
                    METRICS.db_query_duration_seconds
                        .with_label_values(&["select_users"])
                        .observe(duration.as_secs_f64());

                    METRICS.db_queries_total
                        .with_label_values(&["select_users", "success"])
                        .inc();

                    info!(count = users.len(), "Users retrieved");
                    users
                },
                Err(e) => {
                    METRICS.db_queries_total
                        .with_label_values(&["select_users", "error"])
                        .inc();

                    METRICS.errors_total
                        .with_label_values(&["database_error", "users"])
                        .inc();

                    error!(error = %e, "Failed to query users");
                    return Err(AppError::DatabaseError(e));
                }
            };

            Ok(Json(users))
        })
        .await
}
```

## Command Instrumentation

### Instrument Command Execution

```rust
use foundry_observability::{METRICS, SpanBuilder};
use std::time::Instant;
use tracing::{info, warn};

pub async fn execute_command(name: &str) -> Result<()> {
    SpanBuilder::new("execute_command")
        .with_attribute("command_name", name)
        .in_span(async {
            let start = Instant::now();

            info!(command = %name, "Starting command execution");

            let result = match run_command_internal(name).await {
                Ok(output) => {
                    let duration = start.elapsed();

                    // Record success metrics
                    METRICS.record_command(name, duration, true);

                    info!(
                        command = %name,
                        duration_ms = duration.as_millis(),
                        "Command completed successfully"
                    );

                    Ok(output)
                },
                Err(e) => {
                    let duration = start.elapsed();

                    // Record failure metrics
                    METRICS.record_command(name, duration, false);
                    METRICS.record_command_error(name, e.error_type());

                    warn!(
                        command = %name,
                        error = %e,
                        duration_ms = duration.as_millis(),
                        "Command failed"
                    );

                    Err(e)
                }
            };

            result
        })
        .await
}
```

## Database Monitoring

### Track Connection Pool

```rust
use foundry_observability::METRICS;
use sea_orm::DatabaseConnection;

pub async fn monitor_db_pool(db: &DatabaseConnection) {
    // Update connection pool metrics
    // Note: Actual implementation depends on your DB pool
    let pool_status = db.get_pool_status();

    METRICS.db_connections_active.set(pool_status.active as i64);
    METRICS.db_connections_idle.set(pool_status.idle as i64);
}
```

### Instrument Queries

```rust
use foundry_observability::{METRICS, SpanBuilder};
use std::time::Instant;

pub async fn execute_query<T>(
    db: &DatabaseConnection,
    operation: &str,
    query: impl Future<Output = Result<T>>
) -> Result<T> {
    SpanBuilder::new("database_query")
        .with_attribute("operation", operation)
        .in_span(async {
            let start = Instant::now();

            match query.await {
                Ok(result) => {
                    let duration = start.elapsed();

                    METRICS.db_query_duration_seconds
                        .with_label_values(&[operation])
                        .observe(duration.as_secs_f64());

                    METRICS.db_queries_total
                        .with_label_values(&[operation, "success"])
                        .inc();

                    Ok(result)
                },
                Err(e) => {
                    METRICS.db_queries_total
                        .with_label_values(&[operation, "error"])
                        .inc();

                    Err(e)
                }
            }
        })
        .await
}
```

## Cache Monitoring

### Instrument Cache Operations

```rust
use foundry_observability::METRICS;

pub struct MonitoredCache {
    cache: Cache,
    name: String,
}

impl MonitoredCache {
    pub async fn get(&self, key: &str) -> Option<Value> {
        match self.cache.get(key).await {
            Some(value) => {
                METRICS.record_cache_hit(&self.name);
                Some(value)
            },
            None => {
                METRICS.record_cache_miss(&self.name);
                None
            }
        }
    }

    pub async fn set(&self, key: &str, value: Value) {
        self.cache.set(key, value).await;

        // Update cache size
        let size = self.cache.estimated_size().await;
        METRICS.set_cache_size(&self.name, size as i64);
    }
}
```

## Queue Monitoring

### Instrument Message Processing

```rust
use foundry_observability::{METRICS, SpanBuilder};
use std::time::Instant;

pub async fn process_message(queue_name: &str, message: Message) -> Result<()> {
    // Update queue size
    let queue_size = get_queue_size(queue_name).await;
    METRICS.queue_size
        .with_label_values(&[queue_name])
        .set(queue_size as i64);

    SpanBuilder::new("process_queue_message")
        .with_attribute("queue", queue_name)
        .with_attribute("message_id", &message.id)
        .in_span(async {
            let start = Instant::now();

            let result = handle_message(message).await;
            let duration = start.elapsed();

            // Record processing metrics
            METRICS.queue_processing_duration_seconds
                .with_label_values(&[queue_name])
                .observe(duration.as_secs_f64());

            let status = if result.is_ok() { "success" } else { "error" };
            METRICS.queue_messages_processed
                .with_label_values(&[queue_name, status])
                .inc();

            result
        })
        .await
}
```

## Custom Metrics

### Define Custom Metrics

```rust
use prometheus::{IntCounter, Registry};
use lazy_static::lazy_static;
use foundry_observability::metrics::REGISTRY;

lazy_static! {
    pub static ref CUSTOM_COUNTER: IntCounter = {
        let counter = IntCounter::new(
            "myapp_custom_total",
            "Custom counter for my app"
        ).unwrap();

        REGISTRY.register(Box::new(counter.clone())).unwrap();
        counter
    };
}

// Usage
pub fn record_custom_event() {
    CUSTOM_COUNTER.inc();
}
```

### Custom Histogram

```rust
use prometheus::{Histogram, HistogramOpts};

lazy_static! {
    pub static ref CUSTOM_DURATION: Histogram = {
        let opts = HistogramOpts::new(
            "myapp_custom_duration_seconds",
            "Custom operation duration"
        )
        .buckets(vec![0.001, 0.01, 0.1, 1.0, 10.0]);

        let histogram = Histogram::with_opts(opts).unwrap();
        REGISTRY.register(Box::new(histogram.clone())).unwrap();
        histogram
    };
}

// Usage with timer
pub async fn timed_operation() {
    let timer = CUSTOM_DURATION.start_timer();

    // Your operation here
    do_work().await;

    timer.observe_duration(); // Automatically records duration
}
```

## Testing

### Test Metrics Recording

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use foundry_observability::METRICS;

    #[tokio::test]
    async fn test_command_metrics() {
        let start_count = METRICS.commands_total
            .with_label_values(&["test_cmd", "success"])
            .get();

        // Execute command
        execute_command("test_cmd").await.unwrap();

        let end_count = METRICS.commands_total
            .with_label_values(&["test_cmd", "success"])
            .get();

        assert_eq!(end_count, start_count + 1);
    }
}
```

### Integration Tests

```rust
#[cfg(test)]
mod integration_tests {
    use axum_test::TestServer;

    #[tokio::test]
    async fn test_metrics_endpoint() {
        let app = create_app();
        let server = TestServer::new(app).unwrap();

        // Make some requests
        server.get("/api/users").await;

        // Check metrics endpoint
        let response = server.get("/metrics").await;
        assert_eq!(response.status_code(), 200);

        let body = response.text();
        assert!(body.contains("rustforge_http_requests_total"));
    }
}
```

## Best Practices

### 1. Consistent Naming

Use consistent metric and label names:
- `_total` suffix for counters
- `_seconds` suffix for durations
- `_bytes` suffix for sizes
- Snake_case for names

### 2. Label Cardinality

Avoid high-cardinality labels:
```rust
// ❌ Bad - user_id has high cardinality
METRICS.requests.with_label_values(&[user_id]).inc();

// ✅ Good - use user_type instead
METRICS.requests.with_label_values(&[user_type]).inc();
```

### 3. Structured Logging

Always include context in logs:
```rust
info!(
    user_id = %user.id,
    action = "login",
    duration_ms = duration.as_millis(),
    "User logged in successfully"
);
```

### 4. Error Handling

Always record errors in metrics:
```rust
match operation().await {
    Ok(result) => { /* ... */ },
    Err(e) => {
        METRICS.errors_total
            .with_label_values(&[e.error_type(), "component_name"])
            .inc();
        error!(error = %e, "Operation failed");
        return Err(e);
    }
}
```

### 5. Trace Correlation

Use trace IDs in error messages:
```rust
use foundry_observability::tracing_middleware::current_trace_id;

if let Some(trace_id) = current_trace_id() {
    error!(trace_id = %trace_id, error = %e, "Request failed");
}
```

## Troubleshooting

### Metrics Not Appearing

1. Verify observability is initialized before recording metrics
2. Check `/metrics` endpoint is accessible
3. Ensure Prometheus is scraping your application
4. Check for metric registration errors in logs

### Traces Not Visible

1. Verify OTLP endpoint is reachable
2. Check trace sampling rate (should be > 0)
3. Ensure `TracingMiddleware` is added to router
4. Check OpenTelemetry Collector is running

### High Memory Usage

1. Reduce trace sampling rate
2. Decrease metric cardinality
3. Lower scrape intervals
4. Implement metric cleanup for temporary labels

## Next Steps

1. Set up alerts based on your SLIs/SLOs
2. Create custom Grafana dashboards for your domain metrics
3. Configure alert notification channels (Slack, PagerDuty)
4. Implement custom health checks for critical dependencies
5. Set up log aggregation (ELK, Loki, CloudWatch)
