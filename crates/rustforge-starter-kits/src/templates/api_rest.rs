use serde::{Deserialize, Serialize};

pub const API_REST_TEMPLATE: &str = r#"
# API REST Template Structure

/project-root
├── Cargo.toml
├── .env
├── .env.example
├── rustforge.toml
├── migrations/
│   └── 001_initial_schema.sql
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── lib.rs
│   ├── api/
│   │   ├── mod.rs
│   │   ├── v1/
│   │   │   ├── mod.rs
│   │   │   ├── users.rs
│   │   │   ├── auth.rs
│   │   │   └── posts.rs
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── post.rs
│   │   └── token.rs
│   ├── services/
│   │   ├── mod.rs
│   │   ├── auth_service.rs
│   │   ├── user_service.rs
│   │   └── post_service.rs
│   ├── middleware/
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   ├── cors.rs
│   │   ├── rate_limit.rs
│   │   └── logging.rs
│   ├── errors/
│   │   ├── mod.rs
│   │   └── api_error.rs
│   └── utils/
│       ├── mod.rs
│       ├── validators.rs
│       └── pagination.rs
├── tests/
│   ├── integration/
│   │   ├── auth_test.rs
│   │   └── users_test.rs
│   └── fixtures/
│       └── test_data.sql
└── docs/
    ├── api.md
    └── openapi.yaml
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiRestConfig {
    pub name: String,
    pub version: String,
    pub features: ApiFeatures,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiFeatures {
    pub openapi: bool,
    pub rate_limiting: bool,
    pub cors: bool,
    pub compression: bool,
    pub metrics: bool,
    pub health_check: bool,
    pub versioning: bool,
}

pub fn generate_api_rest_main() -> String {
    r#"use rustforge::prelude::*;
use axum::{
    Router,
    routing::{get, post, put, delete},
    middleware,
    extract::Extension,
};
use std::net::SocketAddr;
use tower_http::{
    cors::CorsLayer,
    compression::CompressionLayer,
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod api;
mod config;
mod errors;
mod middleware as app_middleware;
mod models;
mod services;
mod utils;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    // Load configuration
    dotenvy::dotenv().ok();
    let config = config::load()?;

    // Initialize database
    let db = rustforge::database::connect(&config.database).await?;

    // Run migrations
    rustforge::database::migrate(&db).await?;

    // Initialize services
    let auth_service = services::AuthService::new(db.clone());
    let user_service = services::UserService::new(db.clone());
    let post_service = services::PostService::new(db.clone());

    // Build API router
    let api_v1 = Router::new()
        // Public routes
        .route("/health", get(api::v1::health))
        .route("/auth/login", post(api::v1::auth::login))
        .route("/auth/register", post(api::v1::auth::register))
        .route("/auth/refresh", post(api::v1::auth::refresh))

        // Protected routes
        .route("/users", get(api::v1::users::list))
        .route("/users/:id", get(api::v1::users::get))
        .route("/users", post(api::v1::users::create))
        .route("/users/:id", put(api::v1::users::update))
        .route("/users/:id", delete(api::v1::users::delete))

        .route("/posts", get(api::v1::posts::list))
        .route("/posts/:id", get(api::v1::posts::get))
        .route("/posts", post(api::v1::posts::create))
        .route("/posts/:id", put(api::v1::posts::update))
        .route("/posts/:id", delete(api::v1::posts::delete))

        // Apply authentication middleware to protected routes
        .layer(middleware::from_fn(app_middleware::auth::require_auth));

    // Main application router
    let app = Router::new()
        .nest("/api/v1", api_v1)
        .route("/", get(root))
        .route("/docs", get(api_docs))

        // Global middleware
        .layer(Extension(auth_service))
        .layer(Extension(user_service))
        .layer(Extension(post_service))
        .layer(CorsLayer::permissive())
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(app_middleware::rate_limit::limit));

    // Start server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    tracing::info!("🚀 API Server running on http://{addr}");
    tracing::info!("📚 API Documentation available at http://{addr}/docs");

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;

    Ok(())
}

async fn root() -> &'static str {
    "RustForge REST API v1.0"
}

async fn api_docs() -> impl axum::response::IntoResponse {
    axum::response::Html(include_str!("../static/swagger.html"))
}
"#.to_string()
}

pub fn generate_api_rest_cargo_toml() -> String {
    r#"[package]
name = "rustforge-api"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge Core
rustforge = "0.1"

# Web Framework
axum = { version = "0.7", features = ["macros", "multipart", "ws"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "compression", "trace", "fs"] }
hyper = { version = "1.1", features = ["full"] }

# Database
sea-orm = { version = "0.12", features = ["runtime-tokio-rustls", "sqlx-postgres"] }
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono"] }

# Authentication
jsonwebtoken = "9.2"
argon2 = "0.5"
uuid = { version = "1.7", features = ["v4", "serde"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"

# Validation
validator = { version = "0.16", features = ["derive"] }

# Async Runtime
tokio = { version = "1.35", features = ["full"] }

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# Environment
dotenvy = "0.15"

# Date/Time
chrono = { version = "0.4", features = ["serde"] }

# OpenAPI
utoipa = { version = "4.2", features = ["axum_extras"] }
utoipa-swagger-ui = { version = "6.0", features = ["axum"] }

[dev-dependencies]
# Testing
tokio-test = "0.4"
tower = { version = "0.4", features = ["util"] }
hyper = { version = "1.1", features = ["full"] }
reqwest = { version = "0.11", features = ["json"] }
fake = { version = "2.9", features = ["derive"] }
quickcheck = "1.0"
proptest = "1.4"

# Benchmarking
criterion = { version = "0.5", features = ["async_tokio"] }

[[bench]]
name = "api_benchmark"
harness = false
"#.to_string()
}