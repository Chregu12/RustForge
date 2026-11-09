# RustForge

**A Laravel-inspired application framework for Rust, built on Axum and Tokio.**

<!-- rf-status:begin -->
Workspace version: **1.0.0-rc.3** (development snapshot). Minimum supported Rust: **1.96.0**. RustForge is consumed from Git; no crates.io release is required.
<!-- rf-status:end -->

RustForge combines compact models, declarative validation and familiar facades
with an explicit, injectable Rust API. `App` owns its routes and supplies the
same database pool to model calls and typed services. The core umbrella is `rf`;
additional integrations are opt-in through `rf-full`.

This is a release candidate under development, not a production-proven platform.
The example below targets the pinned implementation snapshot. It does not claim
to work against earlier RC tags.

## Install

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

## A complete application

Save this as `src/main.rs`, then run `cargo run`. The example creates its demo
schema and uses an independent in-memory SQLite database. Data lasts only for
the lifetime of the process. Use a persistent database and versioned migrations
for deployed applications.

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

Try `POST /posts` with `{"title":"Hello","body":"World"}`, then `GET /posts`.
Successful creation returns 201; invalid input returns structured 422 errors;
missing posts return 404. Unexpected errors use the central error response.

The code is generated from `examples/blog-slice/src/main.rs`. CI checks that the
snippets match and tests the application's HTTP behavior and app isolation.

## Async and typed APIs

- `Model!` queries and `validate!` use async database execution. CRUD macros
  expand to `.await`; they do not turn I/O into a synchronous operation.
- In handlers use `Cache::get_async`, `Cache::put_async`, `Mail::send_async`,
  `Mail::raw_async`, and `Storage::get_async` / `put_async`. Existing synchronous
  facade methods and older helper macros remain **blocking compatibility APIs**.
- `validate!` returns only fields with rules. `bool` requires a JSON boolean.
  Use `ValidatedData::into_typed::<Dto>()` or `ValidatedJson<Dto>` for typed input.
- Typed model reads fail on row-mapping errors, including paginated/eager reads.
- `DatabaseManager::scope` shares an injected pool with model/facade calls.
  `DB::transaction(async { ... })` commits on success and rolls back on error.

## Start a Laravel-style project

Install the `forge` CLI from this checkout and generate a standalone app.

```bash
cargo install --path crates/forge-cli --locked
forge new blog
cd blog
forge migrate
forge serve
```

The starter includes a posts API, a persistent SQLite database and reversible
migrations. See the [Forge CLI guide](crates/forge-cli/README.md) for the project
layout and commands.

## Documentation and checks

See [the Wiki](https://github.com/Chregu12/RustForge/wiki),
[API philosophy](docs/API_PHILOSOPHY.md), [core contract](docs/STABLE_CORE.md),
[review-fix migration notes](docs/CORE_REVIEW_FIXES.md), and [tiers](docs/TIERS.md).

```sh
python3 scripts/sync-getting-started.py --check
cargo +1.96.0 check --locked -p rf -p rustforge
cargo +1.96.0 test --locked -p blog-slice
cargo +1.96.0 test --locked -p probe-sweep
```

## License

MIT OR Apache-2.0.
