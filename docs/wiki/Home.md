# RustForge

A Laravel-inspired application framework for Rust on Axum and Tokio.

<!-- rf-status:begin -->
Workspace version: **1.0.0-rc.3** (development snapshot). Minimum supported Rust: **1.96.0**. RustForge is consumed from Git; no crates.io release is required.
<!-- rf-status:end -->

Use `Model!`, `validate!`, and concise handlers, or explicit extractors and
injected services. Both paths can share an application-owned database pool.
`App::new()` owns its routing registry and installs the request context.

## Start here

<!-- rf-dependencies:begin -->
```toml
[dependencies]
rf = { git = "https://github.com/Chregu12/RustForge", rev = "07b84ba3a61669ec64ebc9444d94166fc8eea13d" }
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```
<!-- rf-dependencies:end -->

<!-- rf-example:begin -->
```rust
use axum::{http::StatusCode, response::IntoResponse};
use rf::prelude::*;

Model!(Post: title, body);

async fn store() -> AppResult<impl IntoResponse> {
    let data = validate! { title: string.max(200), body: string }?;
    let post = Post::create(data).await?;
    Ok(json(post).status(StatusCode::CREATED))
}

async fn index() -> AppResult<impl IntoResponse> {
    Ok(json(Post::all_typed().await?))
}

async fn show() -> AppResult<impl IntoResponse> {
    let id = input::<i64>("id").ok_or_else(|| AppError::BadRequest {
        message: "Invalid post id".into(),
    })?;
    Ok(json(Post::find(id).await?.or_404()?))
}

async fn build_app() -> AppResult<axum::Router> {
    let db = DatabaseManager::connect(DatabaseConfig {
        url: "sqlite::memory:".into(),
        max_connections: 1,
        min_connections: 1,
        ..Default::default()
    })
    .await
    .map_err(|e| e.to_string())?;
    // Demo schema. Production applications should apply versioned migrations.
    db.scope(DB::statement_async(
        "CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT NOT NULL, body TEXT NOT NULL)"
    )).await?;
    Ok(App::new()
        .database(db)
        .get("/posts", index)
        .get("/posts/{id}", show)
        .post("/posts", store)
        .build())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = build_app().await?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}
```
<!-- rf-example:end -->

This complete example uses an in-memory SQLite database and creates its schema.
Run it with `cargo run`. The database is recreated on each process start.
The snippets come from the tested `blog-slice` application.

## Documentation

- [Installation](Installation): dependencies and supported compiler.
- [Quick Start](Quick-Start): executable example and HTTP responses.
- [Laravel Syntax](Laravel-Syntax): macros and facade APIs.
- [Core review fixes](https://github.com/Chregu12/RustForge/blob/fix/framework-core-review/docs/CORE_REVIEW_FIXES.md): migration and async guarantees.
- [Feature tiers](https://github.com/Chregu12/RustForge/blob/main/docs/TIERS.md): stable, beta, and experimental surfaces.

Request globals are task-local and panic outside their required scope. Synchronous
I/O facades are blocking compatibility methods; HTTP handlers use `_async`
methods. A `ValidatedData` value is a filtered dynamic map, not a compile-time DTO.
