//! Example of integrating observability with Axum HTTP server

use axum::{
    middleware,
    response::{IntoResponse, Json},
    routing::{get, post},
    Router,
};
use foundry_observability::{
    health_check, init_observability, metrics_handler, shutdown_observability,
    ObservabilityConfig, TracingMiddleware, METRICS,
};
use serde_json::json;
use std::time::Instant;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize observability
    let config = ObservabilityConfig::from_env();
    init_observability(config).await?;

    info!("Starting HTTP server with observability");

    // Build application router
    let app = create_app();

    // Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    info!("Server listening on http://0.0.0.0:3000");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Cleanup
    shutdown_observability().await?;

    Ok(())
}

fn create_app() -> Router {
    Router::new()
        // Application routes
        .route("/", get(root_handler))
        .route("/api/users", get(list_users).post(create_user))
        .route("/api/heavy", get(heavy_operation))
        // Observability routes
        .route("/metrics", get(metrics_endpoint))
        .route("/health", get(health_endpoint))
        // Add tracing middleware
        .layer(middleware::from_fn(TracingMiddleware::middleware))
}

async fn root_handler() -> impl IntoResponse {
    Json(json!({
        "name": "RustForge Example API",
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn list_users() -> impl IntoResponse {
    let start = Instant::now();

    // Simulate database query
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let users = vec![
        json!({"id": 1, "name": "Alice"}),
        json!({"id": 2, "name": "Bob"}),
    ];

    // Record custom metrics
    METRICS
        .db_query_duration_seconds
        .with_label_values(&["select_users"])
        .observe(start.elapsed().as_secs_f64());

    METRICS
        .db_queries_total
        .with_label_values(&["select_users", "success"])
        .inc();

    Json(json!({ "users": users }))
}

async fn create_user() -> impl IntoResponse {
    let start = Instant::now();

    // Simulate database insert
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    METRICS
        .db_query_duration_seconds
        .with_label_values(&["insert_user"])
        .observe(start.elapsed().as_secs_f64());

    Json(json!({
        "id": 3,
        "name": "Charlie",
        "created": true
    }))
}

async fn heavy_operation() -> impl IntoResponse {
    // Simulate heavy computation
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    Json(json!({
        "result": "completed",
        "duration": "2s"
    }))
}

async fn metrics_endpoint() -> impl IntoResponse {
    metrics_handler().await
}

async fn health_endpoint() -> impl IntoResponse {
    health_check().await
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received");
}
