# Quick Start

<!-- rf-status:begin -->
Workspace version: **1.0.0-rc.3** (development snapshot). Minimum supported Rust: **1.96.0**. RustForge is consumed from Git; no crates.io release is required.
<!-- rf-status:end -->

Create a binary project with `cargo new my-app`, enter it, and add:

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

Replace `src/main.rs` with this complete application:

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

Start it with `cargo run`. It listens on `127.0.0.1:3000` and creates an isolated,
in-memory SQLite database. It needs no external service or environment variable.

```sh
curl -i -X POST http://127.0.0.1:3000/posts \
  -H 'Content-Type: application/json' \
  -d '{"title":"Hello","body":"World"}'
curl -i http://127.0.0.1:3000/posts
curl -i http://127.0.0.1:3000/posts/1
```

| Request | Expected response |
|---|---|
| Valid POST | 201 and the created record |
| Missing/empty title or title longer than 200 characters | 422 with field errors |
| GET existing post | 200 |
| GET missing post | 404 |
| Invalid id | 400 |

For a persistent app configure `DatabaseManager` with a file-backed SQLite URL
or Postgres URL and apply versioned, backend-appropriate migrations. The demo
schema is deliberately SQLite-specific. Use `State<AppState>` or
`Extension<DatabaseManager>` for explicit services sharing the app's pool.

The example code is generated, not maintained separately. Run
`python3 scripts/sync-getting-started.py --check` to detect drift.
