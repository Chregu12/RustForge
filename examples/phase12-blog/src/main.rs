// # Phase 12 Blog Example
//
// A full-stack blog application demonstrating integration of all Phase 12 crates:
// - rf-blade: Template engine
// - rf-vite: Asset pipeline
// - rf-livereload: Development hot reload
// - rf-cms: Media management

use axum::{
    routing::{get, post},
    Router, extract::{Path, State, Multipart},
    response::{Html, IntoResponse},
    http::StatusCode,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing_subscriber;
use uuid::Uuid;
use chrono::{DateTime, Utc};

// Import Phase 12 crates
use rf_blade::BladeEngine;
use rf_vite::ViteConfig;
use rf_livereload::LiveReload;
use rf_cms::MediaLibrary;

/// Application state
#[derive(Clone)]
struct AppState {
    blade: Arc<BladeEngine>,
    media: Arc<MediaLibrary>,
    posts: Arc<DashMap<Uuid, Post>>,
    vite_dev: bool,
}

/// Blog post model
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
    id: Uuid,
    title: String,
    content: String,
    featured_image: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    tracing::info!("Starting Phase 12 Blog Example...");

    // Initialize rf-blade template engine
    let blade = Arc::new(BladeEngine::new("templates/")?);
    blade.add_component_path("templates/components/")?;

    // Initialize rf-cms media library
    let media = Arc::new(MediaLibrary::new("storage/media"));

    // Initialize in-memory post storage
    let posts = Arc::new(DashMap::new());

    // Seed with example post
    seed_posts(&posts, &media).await?;

    // Determine if running in dev mode
    let vite_dev = std::env::var("VITE_DEV").unwrap_or_else(|_| "true".to_string()) == "true";

    // Initialize rf-livereload (dev mode only)
    if vite_dev {
        tracing::info!("Starting live reload server...");
        let live_reload = LiveReload::new()
            .watch("templates")
            .watch("resources/css")
            .watch("resources/js")
            .debounce_ms(300);

        tokio::spawn(async move {
            if let Ok(_server) = live_reload.start().await {
                tracing::info!("Live reload server started on port 35729");
            }
        });
    }

    // Initialize rf-vite (dev mode only)
    if vite_dev {
        tracing::info!("Starting Vite dev server...");
        let vite_config = ViteConfig::new(".")
            .entry("resources/js/app.js")
            .entry("resources/css/app.css");

        tokio::spawn(async move {
            if let Ok(_dev_server) = vite_config.dev_server().await {
                tracing::info!("Vite dev server started");
            }
        });
    }

    // Create application state
    let state = AppState {
        blade,
        media,
        posts,
        vite_dev,
    };

    // Build router
    let app = Router::new()
        .route("/", get(home))
        .route("/posts/:id", get(show_post))
        .route("/posts/create", get(create_post_form).post(create_post))
        .route("/media/upload", post(upload_media))
        .nest_service("/storage", ServeDir::new("storage"))
        .with_state(state);

    // Start server
    let addr = "127.0.0.1:3000";
    tracing::info!("Blog server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Home page - list all posts
async fn home(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let posts: Vec<Post> = state.posts.iter().map(|p| p.value().clone()).collect();
    let mut posts_sorted = posts;
    posts_sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));

    let html = state.blade.render("posts.index", serde_json::json!({
        "posts": posts_sorted,
        "vite_dev": state.vite_dev,
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

/// Show single post
async fn show_post(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let uuid = Uuid::parse_str(&id).map_err(|_| StatusCode::BAD_REQUEST)?;

    let post = state.posts.get(&uuid)
        .ok_or(StatusCode::NOT_FOUND)?
        .clone();

    let html = state.blade.render("posts.show", serde_json::json!({
        "post": post,
        "vite_dev": state.vite_dev,
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

/// Show create post form
async fn create_post_form(State(state): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = state.blade.render("posts.create", serde_json::json!({
        "vite_dev": state.vite_dev,
    })).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

/// Create new post
async fn create_post(
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    // In a real app, parse multipart form data
    // For this example, we'll just redirect
    Ok((StatusCode::SEE_OTHER, [("Location", "/")]))
}

/// Upload media file
async fn upload_media(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, StatusCode> {
    while let Some(field) = multipart.next_field().await.map_err(|_| StatusCode::BAD_REQUEST)? {
        if let Some(filename) = field.file_name() {
            let data = field.bytes().await.map_err(|_| StatusCode::BAD_REQUEST)?;

            let file = state.media.upload(filename, data.to_vec())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return Ok(serde_json::json!({
                "success": true,
                "file_id": file.id,
                "url": format!("/storage/media/{}", file.filename)
            }).to_string());
        }
    }

    Err(StatusCode::BAD_REQUEST)
}

/// Seed example posts
async fn seed_posts(
    posts: &DashMap<Uuid, Post>,
    _media: &MediaLibrary,
) -> Result<(), Box<dyn std::error::Error>> {
    let post1 = Post {
        id: Uuid::new_v4(),
        title: "Welcome to RustForge Blog".to_string(),
        content: r#"<p>This is a <strong>full-stack blog</strong> built with RustForge Phase 12 features:</p>
<ul>
<li><strong>rf-blade</strong>: Laravel Blade-like templating</li>
<li><strong>rf-vite</strong>: Modern asset pipeline with HMR</li>
<li><strong>rf-livereload</strong>: Live browser reload</li>
<li><strong>rf-cms</strong>: Media library and content management</li>
</ul>
<p>This demonstrates how all Phase 12 crates work together seamlessly!</p>"#.to_string(),
        featured_image: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    let post2 = Post {
        id: Uuid::new_v4(),
        title: "Getting Started with RustForge".to_string(),
        content: r#"<p>RustForge now supports full-stack development with features like:</p>
<h3>Template Engine</h3>
<p>Use Blade syntax for your views with template inheritance and components.</p>
<h3>Asset Pipeline</h3>
<p>Integrate Vite for lightning-fast frontend builds with Hot Module Replacement.</p>
<h3>Media Management</h3>
<p>Upload images, generate thumbnails, and manage media files easily.</p>"#.to_string(),
        featured_image: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    };

    posts.insert(post1.id, post1);
    posts.insert(post2.id, post2);

    Ok(())
}
