use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Simple in-memory database
type Database = Arc<RwLock<Vec<Post>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
    id: usize,
    title: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct CreatePost {
    title: String,
    content: String,
}

#[tokio::main]
async fn main() {
    // Load environment variables
    dotenv::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustforge_app=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Initialize database
    let db: Database = Arc::new(RwLock::new(vec![
        Post {
            id: 1,
            title: "Welcome to RustForge!".to_string(),
            content: "This is your first post. Start building amazing things!".to_string(),
        },
    ]));

    // Build application routes
    let app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        .route("/api/posts", get(list_posts).post(create_post))
        .route("/api/posts/:id", get(get_post))
        .with_state(db);

    // Get port from environment or use default
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("🚀 RustForge app starting on http://{}", addr);
    tracing::info!("📖 API endpoints:");
    tracing::info!("   GET  http://{}/", addr);
    tracing::info!("   GET  http://{}/health", addr);
    tracing::info!("   GET  http://{}/api/posts", addr);
    tracing::info!("   POST http://{}/api/posts", addr);
    tracing::info!("   GET  http://{}/api/posts/:id", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("Server error");
}

// Root handler
async fn root() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "message": "Welcome to RustForge!",
        "version": "1.0.0",
        "endpoints": {
            "health": "/health",
            "posts": "/api/posts"
        }
    }))
}

// Health check endpoint
async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339()
    }))
}

// List all posts
async fn list_posts(State(db): State<Database>) -> Json<Vec<Post>> {
    let posts = db.read().await;
    Json(posts.clone())
}

// Get a single post by ID
async fn get_post(
    Path(id): Path<usize>,
    State(db): State<Database>,
) -> Result<Json<Post>, StatusCode> {
    let posts = db.read().await;
    posts
        .iter()
        .find(|post| post.id == id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

// Create a new post
async fn create_post(
    State(db): State<Database>,
    Json(payload): Json<CreatePost>,
) -> Result<Json<Post>, StatusCode> {
    let mut posts = db.write().await;

    let new_id = posts.iter().map(|p| p.id).max().unwrap_or(0) + 1;

    let post = Post {
        id: new_id,
        title: payload.title,
        content: payload.content,
    };

    posts.push(post.clone());

    tracing::info!("Created new post with id: {}", new_id);

    Ok(Json(post))
}
