//! RustForge Starter Template
//!
//! A production-ready starter template for building REST APIs with:
//! - SeaORM database integration
//! - JWT authentication
//! - Request validation
//! - Middleware support
//! - Structured MVC architecture
//! - Comprehensive error handling

mod config;
mod controllers;
mod middleware;
mod models;

use axum::{
    middleware as axum_middleware,
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use sea_orm_migration::MigratorTrait;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use config::Settings;
use controllers::{AuthController, PostController, UserController};
use middleware::{auth::require_auth, logging::log_requests};

// Include migrations
mod migrations {
    include!("../database/migrations/mod.rs");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load configuration
    let settings = Settings::from_env()?;

    // Initialize tracing/logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustforge_app=debug,tower_http=debug,sea_orm=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("🚀 Starting {} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    tracing::info!("📝 Environment: {}", settings.app.env);
    tracing::info!("🔧 Debug mode: {}", settings.app.debug);

    // Connect to database
    tracing::info!("📦 Connecting to database...");
    let db = settings.database.connect().await?;
    tracing::info!("✅ Database connected");

    // Run migrations
    tracing::info!("🔄 Running database migrations...");
    migrations::Migrator::up(&db, None).await?;
    tracing::info!("✅ Migrations completed");

    // Build application with CORS
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Protected routes requiring authentication
    let protected_routes = Router::new()
        .route("/api/posts", post(PostController::create))
        .route("/api/posts/:id", put(PostController::update))
        .route("/api/posts/:id", delete(PostController::delete))
        .route("/api/profile", get(UserController::profile))
        .layer(axum_middleware::from_fn(require_auth));

    // Combine all routes
    let app = Router::new()
        // Public routes
        .route("/", get(root_handler))
        .route("/health", get(health_handler))

        // Auth routes (public)
        .route("/auth/register", post(AuthController::register))
        .route("/auth/login", post(AuthController::login))

        // Post routes (public)
        .route("/api/posts", get(PostController::list))
        .route("/api/posts/:id", get(PostController::get))

        // Merge protected routes
        .merge(protected_routes)

        // Global middleware
        .layer(axum_middleware::from_fn(log_requests))
        .layer(TraceLayer::new_for_http())
        .layer(cors)

        // Shared state
        .with_state(db);

    // Server information
    let addr = settings.server_address();
    tracing::info!("🌐 Server Configuration:");
    tracing::info!("   Address: http://{}", addr);
    tracing::info!("");
    tracing::info!("📋 Available Endpoints:");
    tracing::info!("   Public:");
    tracing::info!("     GET    /              - Root endpoint");
    tracing::info!("     GET    /health        - Health check");
    tracing::info!("     POST   /auth/register - Register new user");
    tracing::info!("     POST   /auth/login    - Login user");
    tracing::info!("     GET    /api/posts     - List all posts");
    tracing::info!("     GET    /api/posts/:id - Get post by ID");
    tracing::info!("");
    tracing::info!("   Protected (requires JWT token):");
    tracing::info!("     POST   /api/posts     - Create new post");
    tracing::info!("     PUT    /api/posts/:id - Update post");
    tracing::info!("     DELETE /api/posts/:id - Delete post");
    tracing::info!("     GET    /api/profile   - Get user profile");
    tracing::info!("");
    tracing::info!("📝 Quick Start:");
    tracing::info!("   1. Register: curl -X POST http://{}/auth/register -H 'Content-Type: application/json' -d '{{\"email\":\"user@example.com\",\"password\":\"password123\",\"name\":\"John Doe\"}}'", addr);
    tracing::info!("   2. Login: curl -X POST http://{}/auth/login -H 'Content-Type: application/json' -d '{{\"email\":\"user@example.com\",\"password\":\"password123\"}}'", addr);
    tracing::info!("   3. Use token: curl http://{}/api/profile -H 'Authorization: Bearer YOUR_TOKEN'", addr);
    tracing::info!("");

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("✅ Server started successfully on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

// ============================================================================
// Handler Functions
// ============================================================================

/// Root endpoint - API information
async fn root_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "description": "RustForge starter template with authentication, database, and more",
        "endpoints": {
            "health": "/health",
            "auth": {
                "register": "/auth/register",
                "login": "/auth/login"
            },
            "api": {
                "posts": "/api/posts",
                "profile": "/api/profile"
            }
        }
    }))
}

/// Health check endpoint
async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION")
    }))
}
