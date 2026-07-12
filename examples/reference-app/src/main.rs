//! # RustForge Reference Application — cycle 4
//!
//! A realistic blog-post API that exercises every stable-core surface end-to-end:
//!
//! | Surface          | How it is exercised                                    |
//! |------------------|--------------------------------------------------------|
//! | AUTH             | POST /auth/register, POST /auth/login, GET /me          |
//! | CRUD + ORM       | GET/POST/PUT/DELETE /posts (Model! + create!/find!/ … )|
//! | MIGRATIONS       | DB::statement() at startup creates all tables          |
//! | VALIDATION       | validate! DSL on create/update payloads (422 on fail)  |
//! | CACHE            | CacheFacade on GET /posts list (MemoryCache default)   |
//! | STORAGE          | StorageFacade on POST /upload (MemoryStorage default)  |
//! | QUEUE + JOB      | MemoryQueue + Worker + SendWelcomeJob dispatched at reg |
//! | MAIL             | MailFacade::send() on register (FileMailer default)    |
//! | HEALTH           | GET /health (rf-health, MemoryCheck)                   |
//! | METRICS          | GET /metrics (rf-metrics, Prometheus)                  |
//! | OBSERVABILITY    | rf-logging structured tracing throughout               |
//!
//! ## Environment switches (CI vs production)
//!
//! | Env var             | Absent / default           | Present                      |
//! |---------------------|----------------------------|------------------------------|
//! | `DATABASE_URL`      | in-memory SQLite (rusqlite)| SQLite file path (e.g. `./app.db`) |
//! | `JWT_SECRET`        | built-in dev secret        | custom secret (>= 32 chars)  |
//! | `SMTP_HOST`         | FileMailer → writes .eml   | real SMTP (+ PORT/USER/PASS) |
//! | `SMTP_PORT`         | 587                        | custom SMTP port             |
//! | `SMTP_USER`         | (none)                     | SMTP username                |
//! | `SMTP_PASS`         | (none)                     | SMTP password                |
//! | `MAIL_MAILBOX`      | /tmp/rustforge-mailbox     | custom .eml directory        |
//! | `PORT`              | 3000                       | custom listen port           |
//!
//! **Postgres note:** The `DB` facade (used by `Model!`, `create!`, etc.) uses
//! rusqlite — SQLite only. A `DATABASE_URL` starting with `postgres://` is
//! detected and logged as a warning; the app falls back to in-memory SQLite.
//! Production Postgres would require switching to rf-orm's SeaORM `DatabaseManager`
//! (a framework-level gap tracked in VISION_GAP.md).
//!
//! **Redis / S3 note:** The cache (CacheFacade) and storage (StorageFacade) use
//! in-process memory backends by default. Switching to Redis or S3 requires
//! configuring the global manager at startup — straightforward future work.
//!
//! ## Run it
//!
//! ```sh
//! cargo run -p reference-app          # in-memory SQLite, FileMailer, MemoryCache
//! DATABASE_URL=./blog.db cargo run -p reference-app  # persistent SQLite
//! ```

use std::sync::Arc;

use axum::{
    extract::{Extension, Multipart, Path, State},
    http::StatusCode,
    middleware,
    response::IntoResponse,
    routing::{get, post, put},
    Router,
};
use rf::prelude::*;
use rf_auth::{require_auth_with, Claims, JwtManager, PasswordHasher};
use rf_health::{checks::MemoryCheck, health_router, HealthChecker};
use rf_logging::{init_logging, LogConfig};
use rf_metrics::metrics_router;
use rf_queue::{Job as RfJob, Jobs, MemoryQueue, Queue, Worker};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

// ── ORM Models ───────────────────────────────────────────────────────────────

// User account — backed by the `users` SQLite table.
Model!(User {
    email: String,
    password_hash: String,
});

// Blog post — backed by the `posts` SQLite table.
Model!(Post {
    title: String,
    body: String,
    user_id: i64,
});

// ── Welcome Mail (defined at module level — trait impls must be at item level) ──

/// A welcome message sent to new registrants via MailFacade::send().
struct WelcomeMail {
    to: String,
}

impl rf_mail::Mailable for WelcomeMail {
    fn build(&self) -> rf_mail::MailBuilder {
        rf_mail::MailBuilder::new()
            .from(rf_mail::Address::new("noreply@rustforge.local"))
            .to(rf_mail::Address::new(self.to.as_str()))
            .subject("Welcome to RustForge Reference App!")
            .text(format!(
                "Hi {}!\n\nThanks for joining the RustForge reference app.\n\nEnjoy!",
                self.to
            ))
    }
}

// ── Background Job: SendWelcomeJob ───────────────────────────────────────────

/// Dispatched on user registration. The worker drains it asynchronously and
/// sends the welcome email via the global MailFacade (FileMailer by default).
#[derive(Serialize, Deserialize)]
struct SendWelcomeJob {
    email: String,
}

#[async_trait::async_trait]
impl RfJob for SendWelcomeJob {
    async fn handle(&self) -> Result<(), rf_queue::QueueError> {
        info!(email = %self.email, "send-welcome job: delivering welcome email");

        match rf_mail::MailFacade::send(WelcomeMail {
            to: self.email.clone(),
        }) {
            Ok(()) => info!(email = %self.email, "welcome email delivered"),
            Err(e) => warn!(error = %e, "welcome email failed (non-fatal — job succeeds)"),
        }

        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "send_welcome"
    }
}

// ── Application State ─────────────────────────────────────────────────────────

#[derive(Clone)]
struct AppState {
    jwt: Arc<JwtManager>,
    hasher: Arc<PasswordHasher>,
}

// (jwt_auth removed — protected routes now use rf_auth::require_auth_with,
// which validates JWTs, populates Auth::user(), and injects Extension<Claims>
// all in one reusable framework middleware.)

// ── Migrations ────────────────────────────────────────────────────────────────

/// Run schema migrations using `DB::statement()` — the canonical rusqlite DDL
/// runner. `IF NOT EXISTS` makes this idempotent on every startup.
fn run_migrations() {
    DB::statement(
        "CREATE TABLE IF NOT EXISTS users (\
             id INTEGER PRIMARY KEY AUTOINCREMENT, \
             email TEXT NOT NULL UNIQUE, \
             password_hash TEXT NOT NULL)",
    )
    .expect("migration: create users table");

    DB::statement(
        "CREATE TABLE IF NOT EXISTS posts (\
             id INTEGER PRIMARY KEY AUTOINCREMENT, \
             title TEXT NOT NULL, \
             body TEXT NOT NULL, \
             user_id INTEGER NOT NULL)",
    )
    .expect("migration: create posts table");

    DB::statement(
        "CREATE TABLE IF NOT EXISTS files (\
             id INTEGER PRIMARY KEY AUTOINCREMENT, \
             path TEXT NOT NULL, \
             filename TEXT NOT NULL)",
    )
    .expect("migration: create files table");
}

// ── Auth Handlers ─────────────────────────────────────────────────────────────

/// POST /auth/register
///
/// Validates email + password (≥ 8 chars) via `validate!`, hashes the password
/// with bcrypt, persists the user with `create!(User, …)`, and dispatches a
/// `SendWelcomeJob` onto the global MemoryQueue.
async fn register_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    // validate! reads from the capture_request task-local scope (body/query fields).
    if validate! { email: email, password: string.min(8) }.is_err() {
        return json(serde_json::json!({
            "error": "Validation failed",
            "details": "email is required; password must be at least 8 characters"
        }))
        .status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let email: String = input("email").unwrap_or_default();
    let password: String = input("password").unwrap_or_default();

    // Hash the password with bcrypt via rf-auth PasswordHasher.
    let hash = match state.hasher.hash(&password) {
        Ok(h) => h,
        Err(e) => {
            return json(serde_json::json!({"error": e.to_string()}))
                .status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    // Persist via the ORM macro — real INSERT into `users` table.
    let user = match create!(User, email = email.clone(), password_hash = hash) {
        Ok(u) => u,
        Err(e) if e.contains("UNIQUE") => {
            return json(serde_json::json!({"error": "Email already registered"}))
                .status(StatusCode::CONFLICT)
        }
        Err(e) => {
            return json(serde_json::json!({"error": e}))
                .status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    let user_id = user["id"].as_i64().unwrap_or(0);

    // Dispatch welcome job onto the process-global default queue.
    // dispatch_now() is sync and safe inside a Tokio runtime (uses AsyncBridge).
    if let Err(e) = (SendWelcomeJob { email: email.clone() }).dispatch_now() {
        warn!(error = %e, "Failed to enqueue welcome job (non-fatal)");
    }

    info!(user_id, email = %email, "User registered");
    json(serde_json::json!({"id": user_id, "email": email})).status(StatusCode::CREATED)
}

/// POST /auth/login
///
/// Validates credentials, looks up the user via raw `DB::select`, verifies the
/// bcrypt hash with `PasswordHasher::verify_timing_safe`, and issues a JWT via
/// `JwtManager::generate_token`.
async fn login_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    if validate! { email: email, password: string }.is_err() {
        return json(serde_json::json!({"error": "email and password are required"}))
            .status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let email: String = input("email").unwrap_or_default();
    let password: String = input("password").unwrap_or_default();

    // Look up user by email using the raw SQL facade.
    let rows = match DB::select(
        "SELECT id, email, password_hash FROM users WHERE email = ?",
        &[serde_json::Value::String(email.clone())],
    ) {
        Ok(r) => r,
        Err(e) => {
            return json(serde_json::json!({"error": e}))
                .status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    let user = match rows.into_iter().next() {
        Some(u) => u,
        None => {
            return json(serde_json::json!({"error": "Invalid credentials"}))
                .status(StatusCode::UNAUTHORIZED)
        }
    };

    let hash = user["password_hash"].as_str().unwrap_or_default();
    let user_id = user["id"].as_i64().unwrap_or(0);

    // Timing-safe password verification (bcrypt constant-time compare).
    if !state
        .hasher
        .verify_timing_safe(&password, hash)
        .unwrap_or(false)
    {
        return json(serde_json::json!({"error": "Invalid credentials"}))
            .status(StatusCode::UNAUTHORIZED);
    }

    // Issue JWT (24-hour expiry).
    let claims = Claims::new(user_id as i32, email.clone(), vec!["user".to_string()], 24);
    let token = match state.jwt.generate_token(&claims) {
        Ok(t) => t,
        Err(e) => {
            return json(serde_json::json!({"error": e.to_string()}))
                .status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    info!(user_id, email = %email, "User logged in");
    json(serde_json::json!({"token": token, "user_id": user_id, "email": email}))
}

/// GET /me — protected by `jwt_auth` route_layer; reads Claims from extensions.
async fn me_handler(Extension(claims): Extension<Claims>) -> impl IntoResponse {
    json(serde_json::json!({
        "user_id": claims.user_id,
        "email": claims.sub,
        "roles": claims.roles,
    }))
}

// ── Post CRUD Handlers ────────────────────────────────────────────────────────

/// GET /posts
///
/// Checks the `CacheFacade` first (MemoryCache by default; 60-second TTL).
/// On a cache miss, queries all posts via `Post::all()` and caches the result.
async fn list_posts_handler() -> impl IntoResponse {
    const CACHE_KEY: &str = "posts:list";

    // Cache::get uses the global CacheFacade (MemoryCache in CI/default).
    if let Ok(Some(cached)) = Cache::get::<serde_json::Value>(CACHE_KEY) {
        return json(cached);
    }

    let posts = match Post::all().await {
        Ok(p) => p,
        Err(e) => {
            return json(serde_json::json!({"error": e}))
                .status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    };

    // Serialize and cache for 60 seconds.
    let cached_val =
        serde_json::to_value(&posts).unwrap_or(serde_json::Value::Array(vec![]));
    let _ = Cache::put(CACHE_KEY, cached_val, 60u64);

    json(posts)
}

/// GET /posts/{id}
///
/// Uses axum's `Path<i64>` extractor and `Post::find(id)` for a real SELECT.
async fn show_post_handler(Path(id): Path<i64>) -> impl IntoResponse {
    match Post::find(id).await {
        Ok(Some(post)) => json(post),
        Ok(None) => {
            json(serde_json::json!({"error": "Post not found"})).status(StatusCode::NOT_FOUND)
        }
        Err(e) => json(serde_json::json!({"error": e})).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// POST /posts — protected; creates a post owned by the authenticated user.
///
/// Validates title (max 200) + body via `validate!`, persists with `create!`,
/// and busts the list cache so the next GET /posts reflects the new row.
async fn create_post_handler(Extension(claims): Extension<Claims>) -> impl IntoResponse {
    let user_id = claims.user_id as i64;

    if validate! { title: string.max(200), body: string }.is_err() {
        return json(serde_json::json!({
            "error": "Validation failed",
            "details": "title (max 200 chars) and body are required"
        }))
        .status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();

    // Bust the list cache so the next GET /posts reflects the new row.
    let _ = Cache::forget("posts:list");

    match create!(Post, title = title, body = body, user_id = user_id) {
        Ok(post) => {
            info!(user_id, "Post created");
            json(post).status(StatusCode::CREATED)
        }
        Err(e) => {
            json(serde_json::json!({"error": e})).status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// PUT /posts/{id} — protected; updates title and body of an existing post.
///
/// Returns 404 if no row was updated (`update!` reports 0 affected rows).
async fn update_post_handler(Path(id): Path<i64>) -> impl IntoResponse {
    if validate! { title: string.max(200), body: string }.is_err() {
        return json(serde_json::json!({
            "error": "Validation failed",
            "details": "title (max 200 chars) and body are required"
        }))
        .status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();

    let _ = Cache::forget("posts:list");

    match update!(Post, id, title = title, body = body) {
        Ok(0) => {
            json(serde_json::json!({"error": "Post not found"})).status(StatusCode::NOT_FOUND)
        }
        Ok(_) => match Post::find(id).await {
            Ok(Some(post)) => json(post),
            _ => {
                json(serde_json::json!({"error": "Post not found"})).status(StatusCode::NOT_FOUND)
            }
        },
        Err(e) => {
            json(serde_json::json!({"error": e})).status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// DELETE /posts/{id} — protected; deletes the row.
///
/// Returns 204 No Content on success, 404 if no row was affected.
async fn delete_post_handler(Path(id): Path<i64>) -> impl IntoResponse {
    let _ = Cache::forget("posts:list");

    match delete!(Post, id) {
        Ok(0) => {
            json(serde_json::json!({"error": "Post not found"})).status(StatusCode::NOT_FOUND)
        }
        Ok(_) => Response::no_content(),
        Err(e) => {
            json(serde_json::json!({"error": e})).status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

// ── Upload Handler ─────────────────────────────────────────────────────────────

/// POST /upload — protected; stores the first `file` multipart field.
///
/// Uses `StorageFacade::put` (MemoryStorage by default; S3 when S3_* envs are
/// set via the global StorageManager). Records the file path in the `files` table.
///
/// **Note:** this route is NOT wrapped in `capture_request` because rf-request's
/// multipart parsing drains the body, leaving an empty stream for axum's own
/// `Multipart` extractor. Each tool uses the body once.
async fn upload_handler(
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    let user_id = claims.user_id;

    while let Ok(Some(field)) = multipart.next_field().await {
        if field.name() != Some("file") {
            continue;
        }

        let filename = field.file_name().unwrap_or("upload.bin").to_string();
        let data = match field.bytes().await {
            Ok(b) => b.to_vec(),
            Err(e) => {
                return json(serde_json::json!({"error": e.to_string()}))
                    .status(StatusCode::BAD_REQUEST)
            }
        };

        // StorageFacade::put — MemoryStorage by default; S3 when configured.
        let path = format!("uploads/{user_id}/{filename}");
        if let Err(e) = Storage::put(&path, data) {
            return json(serde_json::json!({"error": e}))
                .status(StatusCode::INTERNAL_SERVER_ERROR);
        }

        // Record in DB (non-fatal if this fails).
        let _ = DB::insert(
            "INSERT INTO files (path, filename) VALUES (?, ?)",
            &[
                serde_json::Value::String(path.clone()),
                serde_json::Value::String(filename.clone()),
            ],
        );

        info!(user_id, path = %path, "File uploaded via StorageFacade");
        return json(serde_json::json!({
            "path": path,
            "filename": filename,
            "storage_url": format!("/storage/{path}"),
        }))
        .status(StatusCode::CREATED);
    }

    json(serde_json::json!({"error": "No 'file' field found in multipart body"}))
        .status(StatusCode::BAD_REQUEST)
}

// ── main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // ── Logging (rf-logging: structured tracing via tracing-subscriber) ──────
    init_logging(LogConfig::default()).map_err(|e| anyhow::anyhow!("{e}"))?;
    info!("RustForge Reference App — cycle 4 — starting up");

    // ── Database ─────────────────────────────────────────────────────────────
    // DATABASE_URL controls which SQLite database file is used (or in-memory).
    // The DB facade (rusqlite) supports SQLite only; see module-level doc for
    // the Postgres gap. The `connection()` call silently falls through on error.
    match std::env::var("DATABASE_URL") {
        Ok(url) if url.starts_with("postgres") => {
            warn!(
                "DATABASE_URL looks like a Postgres URL; the DB facade uses rusqlite \
                 (SQLite only) — falling back to in-memory SQLite. \
                 See examples/reference-app/README.md for the Postgres gap."
            );
        }
        Ok(url) => {
            let _ = DB::connection(&url);
            info!(url = %url, "SQLite database: file mode");
        }
        Err(_) => {
            info!("DATABASE_URL not set → in-memory SQLite (data lost on exit)");
        }
    }
    run_migrations();
    info!("Schema migrations applied (CREATE TABLE IF NOT EXISTS)");

    // ── Auth ──────────────────────────────────────────────────────────────────
    let jwt_secret = std::env::var("JWT_SECRET")
        .unwrap_or_else(|_| "rustforge-reference-app-jwt-secret-key-32ch!!".to_string());
    let jwt = Arc::new(JwtManager::new(&jwt_secret)?);
    let hasher = Arc::new(PasswordHasher::bcrypt(12)?);
    let state = Arc::new(AppState { jwt, hasher });

    // ── Mail ──────────────────────────────────────────────────────────────────
    // Default: FileMailer writes .eml files to $MAIL_MAILBOX (or /tmp/rustforge-mailbox).
    // SMTP: set SMTP_HOST (+ optional SMTP_PORT / SMTP_USER / SMTP_PASS).
    if let Ok(smtp_host) = std::env::var("SMTP_HOST") {
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(587);
        let smtp_cfg = rf_mail::SmtpConfig {
            host: smtp_host.clone(),
            port,
            username: std::env::var("SMTP_USER").unwrap_or_default(),
            password: std::env::var("SMTP_PASS").unwrap_or_default(),
            from_address: "noreply@rustforge.local".to_string(),
            from_name: Some("RustForge Reference App".to_string()),
        };
        match rf_mail::MailFacade::smtp(smtp_cfg) {
            Ok(()) => info!(host = %smtp_host, "SMTP mail transport configured"),
            Err(e) => warn!(error = %e, "SMTP setup failed — falling back to FileMailer"),
        }
    } else {
        info!("SMTP_HOST not set → FileMailer (writes .eml to /tmp/rustforge-mailbox)");
    }

    // ── Queue + Worker ────────────────────────────────────────────────────────
    // MemoryQueue: zero-config in-process driver with DLQ support.
    // Jobs::set_queue installs it as the process-global default so
    // dispatch_now() (called from handlers) routes here.
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
    Jobs::set_queue(queue.clone());

    // Register SendWelcomeJob and start draining in a background Tokio task.
    let worker = Worker::new(queue).register::<SendWelcomeJob>();
    tokio::spawn(async move {
        if let Err(e) = worker.start().await {
            warn!(error = %e, "Queue worker stopped unexpectedly");
        }
    });
    info!("Queue worker started (MemoryQueue, background task)");

    // ── Health ────────────────────────────────────────────────────────────────
    let health_checker = HealthChecker::new().add_check(MemoryCheck::default());

    // ── Router ────────────────────────────────────────────────────────────────

    // Protected routes: require_auth_with validates JWTs, sets up Auth:: scope,
    // and injects Extension<Claims> — replacing the old hand-rolled jwt_auth.
    let jwt_mw = require_auth_with(state.jwt.clone());
    let protected: Router<Arc<AppState>> = Router::new()
        .route("/me", get(me_handler))
        .route("/posts", post(create_post_handler))
        .route("/posts/{id}", put(update_post_handler).delete(delete_post_handler))
        .route_layer(middleware::from_fn(jwt_mw));

    // Main API router: capture_request buffers the body so input() / validate!
    // work in handlers (public + protected routes share the same middleware).
    let api: Router<Arc<AppState>> = Router::new()
        .route("/auth/register", post(register_handler))
        .route("/auth/login", post(login_handler))
        .route("/posts", get(list_posts_handler))
        .route("/posts/{id}", get(show_post_handler))
        .merge(protected)
        .layer(middleware::from_fn(rf::web::capture_request));

    // Upload sub-router WITHOUT capture_request: rf-request drains multipart
    // into the task-local scope and rebuilds with an empty body, so axum's own
    // Multipart extractor would see nothing. This route needs the raw body.
    let upload_router: Router<Arc<AppState>> = Router::new()
        .route("/upload", post(upload_handler))
        .route_layer(middleware::from_fn(require_auth_with(state.jwt.clone())));

    // Assemble: stateful routes first (with_state → Router<()>), then merge
    // the already-stateless health and metrics routers.
    let app: Router = Router::new()
        .merge(api)
        .merge(upload_router)
        .with_state(state)
        .merge(health_router(health_checker)) // GET /health, /health/live, /health/ready
        .merge(metrics_router()); // GET /metrics (Prometheus text format)

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!(addr = %addr, "RustForge Reference App listening");

    axum::serve(listener, app).await?;

    Ok(())
}
