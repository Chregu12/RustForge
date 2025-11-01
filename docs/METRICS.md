# Performance Monitoring & Metrics

RustForge bietet ein umfassendes Performance Monitoring System zur Überwachung und Optimierung deiner Anwendung.

## Features

- **Metrics Collection**: Sammle und speichere Performance-Metriken
- **Aggregation**: Automatische Berechnung von Statistiken (Avg, Min, Max, Perzentile)
- **Timer**: Einfaches Messen von Operationsdauern
- **System Metrics**: CPU, Memory, Connections, Response Times
- **Reports**: Detaillierte Performance-Reports

## Quick Start

### 1. Metrics sammeln

```rust
use foundry_infra::{PerformanceMonitor, Metric};

let monitor = PerformanceMonitor::new();

// Einfache Metrik
let metric = Metric::new("api.requests", 42.0, "count");
monitor.collect(metric).await;

// Mit Tags
let metric = Metric::new("api.response_time", 125.5, "ms")
    .with_tag("endpoint", "/api/users")
    .with_tag("method", "GET");
monitor.collect(metric).await;
```

### 2. Timer verwenden

```rust
// Timer starten
let timer = monitor.timer("database.query");

// Operation ausführen
let result = database.query("SELECT * FROM users").await?;

// Timer stoppen und Metrik sammeln
let metric = timer.stop_as_metric();
monitor.collect(metric).await;
```

### 3. System Metrics

```rust
use foundry_infra::SystemMetrics;

let sys_metrics = SystemMetrics {
    cpu_usage_percent: 45.5,
    memory_usage_mb: 256.0,
    memory_available_mb: 1024.0,
    active_connections: 42,
    requests_per_second: 123.45,
    avg_response_time_ms: 25.5,
};

monitor.collect_system_metrics(sys_metrics).await;
```

### 4. Report generieren

```rust
let report = monitor.report().await;

println!("Total Metrics: {}", report.total_metrics);
println!("Metric Names: {:?}", report.metric_names);

for (name, agg) in &report.aggregates {
    println!("\nMetric: {}", name);
    println!("  Average: {}", agg.average);
    println!("  Min: {}", agg.min);
    println!("  Max: {}", agg.max);
    println!("  Count: {}", agg.count);
    if let Some(p95) = agg.p95 {
        println!("  P95: {}", p95);
    }
}
```

## CLI Commands

### Performance Report

```bash
rustforge metrics:report
```

### Metrics löschen

```bash
rustforge metrics:clear
```

## Metriken-Typen

### Counter

```rust
// Request-Counter
let metric = Metric::new("http.requests.total", 1.0, "count")
    .with_tag("status", "200");
monitor.collect(metric).await;
```

### Gauge

```rust
// Aktuelle Anzahl
let metric = Metric::new("websocket.connections", 42.0, "count");
monitor.collect(metric).await;
```

### Histogram/Timing

```rust
// Response-Zeit
let timer = monitor.timer("api.request.duration");
// ... operation ...
let metric = timer.stop_as_metric()
    .with_tag("endpoint", "/api/users");
monitor.collect(metric).await;
```

## Aggregationen

```rust
let collector = monitor.collector();

// Statistiken für eine Metrik
if let Some(agg) = collector.get_aggregate("api.response_time").await {
    println!("Average: {} ms", agg.average);
    println!("P50: {:?} ms", agg.p50);
    println!("P95: {:?} ms", agg.p95);
    println!("P99: {:?} ms", agg.p99);
    println!("Min: {} ms", agg.min);
    println!("Max: {} ms", agg.max);
}
```

## Middleware Integration

```rust
use axum::{
    middleware::{self, Next},
    body::Body,
    http::Request,
};

async fn metrics_middleware(
    req: Request<Body>,
    next: Next,
) -> impl IntoResponse {
    let monitor = get_performance_monitor(); // Shared instance
    let timer = monitor.timer("http.request");

    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let response = next.run(req).await;

    let metric = timer.stop_as_metric()
        .with_tag("method", method.as_str())
        .with_tag("path", &path)
        .with_tag("status", response.status().as_str());

    monitor.collect(metric).await;

    response
}

// In deinem Router
let app = Router::new()
    .route("/api/users", get(users_handler))
    .layer(middleware::from_fn(metrics_middleware));
```

## Database Query Monitoring

```rust
async fn execute_query<T>(
    db: &DatabaseConnection,
    query: &str,
    monitor: &PerformanceMonitor,
) -> Result<T> {
    let timer = monitor.timer("database.query");

    let result = db.execute(query).await?;

    let metric = timer.stop_as_metric()
        .with_tag("query_type", "SELECT");
    monitor.collect(metric).await;

    Ok(result)
}
```

## WebSocket Metrics

```rust
use foundry_api::websocket::WebSocketManager;

async fn track_websocket_metrics(
    ws_manager: &WebSocketManager,
    monitor: &PerformanceMonitor,
) {
    let conn_count = ws_manager.connection_count().await;

    let metric = Metric::new("websocket.connections.active", conn_count as f64, "count");
    monitor.collect(metric).await;
}
```

## Periodisches Monitoring

```rust
use tokio::time::{interval, Duration};

async fn start_periodic_monitoring(monitor: PerformanceMonitor) {
    let mut timer = interval(Duration::from_secs(60));

    loop {
        timer.tick().await;

        // System-Metriken sammeln
        let sys_metrics = SystemMetrics {
            cpu_usage_percent: get_cpu_usage(),
            memory_usage_mb: get_memory_usage(),
            // ...
        };

        monitor.collect_system_metrics(sys_metrics).await;
    }
}

// Im Main
tokio::spawn(start_periodic_monitoring(monitor.clone()));
```

## Export

### JSON Export

```rust
let report = monitor.report().await;
let json = serde_json::to_string_pretty(&report)?;
std::fs::write("metrics_report.json", json)?;
```

### Prometheus Format (Zukünftig)

```rust
// Planned:
let prometheus_metrics = monitor.export_prometheus().await;
```

## Best Practices

### 1. Tag-Konventionen

```rust
// Verwende konsistente Tag-Namen
metric.with_tag("environment", "production")
      .with_tag("service", "api")
      .with_tag("version", "1.0.0");
```

### 2. Metrik-Namen

```rust
// Verwende hierarchische Namen mit Punkten
"http.requests.total"
"database.queries.duration"
"websocket.connections.active"
"cache.hits.rate"
```

### 3. Historie-Limitierung

```rust
let collector = MetricsCollector::new()
    .with_max_history_size(10_000);
```

### 4. Batch Collection

```rust
let metrics = vec![
    Metric::new("metric1", 1.0, "count"),
    Metric::new("metric2", 2.0, "count"),
    Metric::new("metric3", 3.0, "count"),
];

collector.collect_batch(metrics).await;
```

## Debugging

### Alle Metriken anzeigen

```rust
let all_metrics = collector.get_all().await;
for metric in all_metrics {
    println!("{}: {} {}", metric.name, metric.value, metric.unit);
}
```

### Metriken nach Namen

```rust
let request_metrics = collector.get_by_name("http.requests").await;
println!("Found {} request metrics", request_metrics.len());
```

### Verfügbare Metriken

```rust
let names = collector.get_metric_names().await;
println!("Available metrics: {:?}", names);
```

## Beispiel: Complete Monitoring Setup

```rust
use foundry_infra::{PerformanceMonitor, SystemMetrics, Metric};
use std::sync::Arc;
use tokio::time::{interval, Duration};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Shared Monitor
    let monitor = Arc::new(PerformanceMonitor::new());

    // System Metrics Task
    let monitor_clone = monitor.clone();
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(30));
        loop {
            timer.tick().await;

            let sys = SystemMetrics {
                cpu_usage_percent: 45.0,
                memory_usage_mb: 512.0,
                memory_available_mb: 2048.0,
                active_connections: 100,
                requests_per_second: 50.0,
                avg_response_time_ms: 25.0,
            };

            monitor_clone.collect_system_metrics(sys).await;
        }
    });

    // Report Task
    let monitor_clone = monitor.clone();
    tokio::spawn(async move {
        let mut timer = interval(Duration::from_secs(300)); // 5 Minuten
        loop {
            timer.tick().await;

            let report = monitor_clone.report().await;
            println!("\n=== Performance Report ===");
            println!("Total Metrics: {}", report.total_metrics);

            for (name, agg) in &report.aggregates {
                if agg.count > 0 {
                    println!("\n{}: avg={:.2}, p95={:?}",
                        name, agg.average, agg.p95);
                }
            }
        }
    });

    // Your application
    // ...

    Ok(())
}
```

## Zukünftige Features

- [ ] Prometheus Export
- [ ] Grafana Integration
- [ ] Alert Thresholds
- [ ] Anomaly Detection
- [ ] Distributed Tracing Integration

---

**Version**: 0.1.0
**Letztes Update**: 2025-11-01
