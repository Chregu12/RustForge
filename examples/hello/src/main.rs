//! Hello World - Minimal RustForge Phase 2 Application
//!
//! Demonstrates integration of:
//! - rf-core: Error handling and request context
//! - rf-web: Axum integration with middleware
//! - rf-config: Type-safe configuration
//! - rf-container: Dependency injection
//!
//! ## Running
//!
//! ```bash
//! cargo run -p hello
//! ```
//!
//! ## Endpoints
//!
//! - `GET /` - Hello world
//! - `GET /health` - Health check
//! - `GET /ready` - Readiness probe
//! - `GET /metrics` - Metrics (placeholder)
//! - `POST /echo` - Echo request body

use axum::{
    extract::{Extension, Json},
    response::IntoResponse,
    routing::{get, post},
};
use rf_config::{AppConfig, ConfigLoader};
use rf_container::{Scope, ServiceRegistry};
use rf_core::{AppError, AppResult, RequestContext};
use rf_web::{
    middleware::CorsConfig,
    RouterBuilder,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    config: Arc<AppConfig>,
    container: ServiceRegistry,
}

/// Echo request/response
#[derive(Debug, Serialize, Deserialize)]
struct EchoMessage {
    message: String,
}

/// Health check response
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
}

/// Metrics response (placeholder)
#[derive(Debug, Serialize)]
struct MetricsResponse {
    requests_total: u64,
    uptime_seconds: u64,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Hello World application...");

    // Load configuration
    let config = match ConfigLoader::new().load::<AppConfig>() {
        Ok(cfg) => {
            info!("Configuration loaded successfully");
            cfg
        }
        Err(e) => {
            info!("Failed to load config: {}, using defaults", e);
            AppConfig {
                server: rf_config::ServerConfig::default(),
                database: rf_config::DatabaseConfig::default(),
                auth: rf_config::AuthConfig::default(),
            }
        }
    };

    // Validate configuration
    if let Err(e) = config.validate() {
        return Err(anyhow::anyhow!("Invalid configuration: {}", e));
    }

    info!("Server config: {}:{}", config.server.host, config.server.port);

    // Setup dependency injection container
    let mut container = ServiceRegistry::new();

    // Register AppConfig as singleton
    let config_clone = config.clone();
    container.register(Scope::Singleton, move || Arc::new(config_clone.clone()));

    // Create application state
    let state = AppState {
        config: Arc::new(config.clone()),
        container,
    };

    // Build router with middleware
    let cors_config = CorsConfig {
        allowed_origins: vec!["*".to_string()],
        allowed_methods: vec![
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::OPTIONS,
        ],
        allowed_headers: vec![
            "content-type".to_string(),
            "authorization".to_string(),
        ],
        max_age: Some(std::time::Duration::from_secs(3600)),
    };

    let app = RouterBuilder::new()
        .with_tracing(true)
        .with_cors(true)
        .cors_config(cors_config)
        .with_compression(true)
        .with_timeout(true)
        .timeout_duration(std::time::Duration::from_secs(30))
        .route("/", get(hello_handler))
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        .route("/metrics", get(metrics_handler))
        .route("/echo", post(echo_handler))
        .build()
        .layer(Extension(state));

    // Start server
    let addr = format!("{}:{}", config.server.host, config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Server listening on {}", addr);
    info!("Health check: http://{}/health", addr);
    info!("Ready check: http://{}/ready", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

/// Hello world handler
async fn hello_handler(Extension(state): Extension<AppState>) -> impl IntoResponse {
    let ctx = RequestContext::new("/", "GET");

    info!(
        trace_id = %ctx.trace_id(),
        "Hello handler called"
    );

    Json(serde_json::json!({
        "message": "Hello from RustForge Phase 2!",
        "trace_id": ctx.trace_id(),
        "version": env!("CARGO_PKG_VERSION"),
        "config": {
            "server": {
                "host": state.config.server.host,
                "port": state.config.server.port,
            }
        }
    }))
}

/// Health check handler
async fn health_handler() -> AppResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Readiness probe handler
async fn ready_handler() -> AppResult<Json<serde_json::Value>> {
    // In a real app, check database connections, etc.
    Ok(Json(serde_json::json!({
        "ready": true,
        "checks": {
            "database": "ok",
            "cache": "ok"
        }
    })))
}

/// Metrics handler (placeholder)
async fn metrics_handler() -> AppResult<Json<MetricsResponse>> {
    Ok(Json(MetricsResponse {
        requests_total: 0,
        uptime_seconds: 0,
    }))
}

/// Echo handler
async fn echo_handler(
    Json(payload): Json<EchoMessage>,
) -> AppResult<Json<EchoMessage>> {
    info!("Echo: {}", payload.message);

    if payload.message.is_empty() {
        return Err(AppError::BadRequest {
            message: "Message cannot be empty".to_string(),
        });
    }

    Ok(Json(EchoMessage {
        message: format!("Echo: {}", payload.message),
    }))
}
