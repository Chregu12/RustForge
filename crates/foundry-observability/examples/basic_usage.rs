//! Basic usage example of the observability system

use foundry_observability::{
    init_observability, shutdown_observability, ObservabilityConfig, OtelConfig, METRICS,
};
use std::time::{Duration, Instant};
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize observability with default config
    let config = ObservabilityConfig {
        otel: OtelConfig {
            enabled: true,
            endpoint: "http://localhost:4317".to_string(),
            ..Default::default()
        },
        log_level: "info".to_string(),
        log_json: false,
        service_name: "example-app".to_string(),
        environment: "development".to_string(),
        ..Default::default()
    };

    init_observability(config).await?;

    info!("Application started");

    // Simulate some work with metrics
    simulate_commands().await;
    simulate_http_requests().await;
    simulate_cache_operations().await;

    info!("Application shutting down");

    // Gracefully shutdown observability
    shutdown_observability().await?;

    Ok(())
}

async fn simulate_commands() {
    info!("Simulating command execution");

    for i in 0..10 {
        let start = Instant::now();
        let command_name = format!("command_{}", i % 3);

        // Simulate work
        tokio::time::sleep(Duration::from_millis(50 + i * 10)).await;

        let duration = start.elapsed();
        let success = i % 5 != 0; // Simulate 20% failure rate

        // Record metrics
        METRICS.record_command(&command_name, duration, success);

        if success {
            info!(command = %command_name, duration_ms = duration.as_millis(), "Command completed");
        } else {
            METRICS.record_command_error(&command_name, "validation_error");
            error!(command = %command_name, "Command failed");
        }
    }
}

async fn simulate_http_requests() {
    info!("Simulating HTTP requests");

    let endpoints = ["/api/users", "/api/posts", "/api/comments"];
    let methods = ["GET", "POST"];

    for i in 0..15 {
        let start = Instant::now();
        let path = endpoints[i % endpoints.len()];
        let method = methods[i % methods.len()];

        // Simulate request processing
        tokio::time::sleep(Duration::from_millis(20 + i * 5)).await;

        let duration = start.elapsed();
        let status = if i % 10 == 0 { 500 } else { 200 }; // 10% error rate

        METRICS.record_http_request(method, path, status, duration);

        if status == 200 {
            info!(method = %method, path = %path, status = status, duration_ms = duration.as_millis(), "HTTP request completed");
        } else {
            warn!(method = %method, path = %path, status = status, "HTTP request failed");
        }
    }
}

async fn simulate_cache_operations() {
    info!("Simulating cache operations");

    for i in 0..20 {
        let cache_name = "user_cache";

        if i % 3 == 0 {
            METRICS.record_cache_miss(cache_name);
            info!(cache = cache_name, "Cache miss");
        } else {
            METRICS.record_cache_hit(cache_name);
            info!(cache = cache_name, "Cache hit");
        }

        // Update cache size
        let size = 1024 * (100 + i);
        METRICS.set_cache_size(cache_name, size);
    }
}
