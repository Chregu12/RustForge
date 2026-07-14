# Quick Start

Build a small blog-post API end-to-end using only the stable DX surface.
After completing this guide you will have a running HTTP server that persists
posts in SQLite, validates input, and returns JSON — written in a handful of
lines that mirror the `examples/blog-slice` reference in the repository
(that example ships an integration test and runs in CI).

Estimated time: ~15 minutes.

---

## Prerequisites

- Rust >= 1.79.0 (see [Installation](Installation))
- The `rf` crate added to your project (see [Installation → Adding RustForge](Installation#adding-rustforge-to-your-project))

---

## Step 1 — Create a new Rust project

```sh
cargo new my-blog-api
cd my-blog-api
```

---

## Step 2 — Configure `Cargo.toml`

Replace the generated `[dependencies]` section:

```toml
[package]
name = "my-blog-api"
version = "0.1.0"
edition = "2021"

[dependencies]
# RustForge umbrella — all stable DX via `use rf::prelude::*`
rf = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.1" }

# Async runtime
tokio = { version = "1", features = ["full"] }

# Serialization (required for Model! derive)
serde     = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP layer — must match rf-routing's axum version
axum = "0.8"
```

---

## Step 3 — Write `src/main.rs`

The snippet below is derived directly from `examples/blog-slice/src/main.rs`
(a CI-tested example). Every name here is grep-verified in `crates/`.

```rust
use rf::prelude::*;

// Declare a model backed by SQLite.
// Model! generates the struct, table mapping, and async CRUD methods.
Model!(Post: title, body);

// POST /posts — validate input, persist a row, return it as JSON.
async fn create_post() -> impl axum::response::IntoResponse {
    // validate! reads from the request task-local populated by capture_request.
    // Returns Err(ValidationErrors) on failure; the DX layer hides the .await.
    if validate! { title: string.max(100), body: string }.is_err() {
        return json(serde_json::json!({"error": "validation failed"}));
    }
    let title: String = input("title").unwrap_or_default();
    let body: String  = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(created) => json(created),
        Err(e)      => json(serde_json::json!({"error": e.to_string()})),
    }
}

// GET /posts — list every post.
async fn list_posts() -> impl axum::response::IntoResponse {
    match Post::all().await {
        Ok(posts) => json(posts),
        Err(e)    => json(serde_json::json!({"error": e.to_string()})),
    }
}

// GET /posts/{id} — show one post.
// The `:id` path parameter is available to `input()` without an explicit
// Path extractor because capture_request merges path params into the
// task-local request scope.
async fn show_post() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None     => return json(serde_json::json!({"error": "invalid id"})),
    };
    match Post::find(id).await {
        Ok(Some(post)) => json(post),
        Ok(None)       => json(serde_json::json!({"error": "not found"})),
        Err(e)         => json(serde_json::json!({"error": e.to_string()})),
    }
}

// Register routes on the global router and attach the capture_request
// middleware that populates input() / validate! / has() / file().
fn build_app() -> axum::Router {
    post("/posts",      create_post);
    get("/posts",       list_posts);
    get("/posts/{id}",  show_post);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    // Create the table once at startup (idempotent).
    DB::statement(
        "CREATE TABLE IF NOT EXISTS posts \
         (id INTEGER PRIMARY KEY, title TEXT, body TEXT)",
    )
    .expect("create table");

    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("bind");
    println!("listening on http://127.0.0.1:3000");
    axum::serve(listener, app).await.expect("serve");
}
```

### What each piece does

| Piece | What it does |
|---|---|
| `use rf::prelude::*` | Single import for the entire stable DX surface |
| `Model!(Post: title, body)` | Declares the `Post` struct, `posts` table mapping, and async `Post::all()` / `Post::find(id)` |
| `validate! { title: string.max(100), body: string }` | Validates fields from the current request; returns `Err(ValidationErrors)` on failure, `422` with per-field errors when wired to a proper error handler |
| `input("title")` | Reads the `title` field from the buffered request (body, query string, or path params) |
| `create!(Post, title = title, body = body)` | Executes `INSERT INTO posts …` and returns the created row as `serde_json::Value` |
| `json(value)` | Builds an `application/json` response |
| `rf::web::capture_request` | Middleware that buffers the body and populates the task-local so `input()` / `validate!` work |

---

## Step 4 — Run it

```sh
cargo run
# Output: listening on http://127.0.0.1:3000
```

---

## Step 5 — Hit the routes

Open a second terminal:

```sh
# Create a post
curl -X POST http://127.0.0.1:3000/posts \
  -H "Content-Type: application/json" \
  -d '{"title":"Hello","body":"World"}'
# -> {"id":1,"title":"Hello","body":"World"}

# List all posts
curl http://127.0.0.1:3000/posts
# -> [{"id":1,"title":"Hello","body":"World"}]

# Show one post
curl http://127.0.0.1:3000/posts/1
# -> {"id":1,"title":"Hello","body":"World"}

# Trigger validation (title too long)
curl -X POST http://127.0.0.1:3000/posts \
  -H "Content-Type: application/json" \
  -d "{\"title\":\"$(python3 -c 'print("x"*200)')\",\"body\":\"b\"}"
# -> {"error":"validation failed"}
```

---

## Using Postgres instead of SQLite

No code changes are needed. Set `DATABASE_URL` before running:

```sh
DATABASE_URL=postgres://user:pass@localhost/mydb cargo run
```

The `DB` facade and ORM macros (`create!`, `find!`, …) detect the `postgres://`
prefix at startup and switch backends automatically. See
[Installation → Database support](Installation#database-support) for caveats.

---

## Next: add validation structs, auth, cache, and more

For a fuller app that exercises **auth + CRUD + cache + file upload + queue +
mail + health + metrics**, read `examples/reference-app/src/main.rs` — it is the
flagship reference and is CI-tested end-to-end. Key patterns shown there:

| Feature | API |
|---|---|
| JWT auth middleware | `require_auth_with(jwt_manager)` as a `route_layer` |
| Password hashing | `rf_auth::PasswordHasher::bcrypt(cost)` / `hasher.hash(pw)` / `hasher.verify_timing_safe(pw, hash)` |
| Cache | `Cache::get::<T>(key)` / `Cache::put(key, val, ttl_secs)` / `Cache::forget(key)` |
| File upload | `axum::extract::Multipart` + `Storage::put(path, bytes)` |
| Background job | `impl rf_queue::Job for MyJob { async fn handle(&self) }` + `Jobs::set_queue(Arc::new(MemoryQueue::new()))` + `Worker::new(queue).register::<MyJob>().start()` |
| Mail | `impl rf_mail::Mailable for MyMail { fn build(&self) -> MailBuilder }` + `rf_mail::MailFacade::send(mail)` |
| Structured validation in extractor | `Model!(User { validated, username: String @ min(3) max(20) alphanumeric, … })` then `ValidatedJson<CreateUser>` in the handler (see `examples/validated-signup/`) |

---

## Stable DX at a glance

Everything below is available from `use rf::prelude::*`:

```rust
// Routing (register on the global router)
get("/path", handler_fn);
post("/path", handler_fn);
put("/path", handler_fn);
patch("/path", handler_fn);
delete("/path", handler_fn);

// Build the axum::Router with the request middleware
rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request))

// ORM
Model!(Thing: field_a, field_b);          // simple form
Model!(Thing { field_a: String, … });     // typed form

create!(Thing, field_a = val, …)          // INSERT -> serde_json::Value
find!(Thing, id)                          // SELECT by PK -> Option<Value>
update!(Thing, id, field_a = val, …)      // UPDATE
delete!(Thing, id)                        // DELETE -> affected rows

Thing::all().await                        // SELECT *
Thing::find(id).await                     // SELECT by PK

// Raw SQL
DB::statement("CREATE TABLE …")
DB::select("SELECT … WHERE id = ?", &[json!(1)])
DB::insert("INSERT INTO … VALUES (?, ?)", &[…])

// Request globals (require capture_request middleware)
input::<T>("field")   // reads body / query / path params
has("field")          // field presence check
file("name")          // uploaded file

// Response
json(value)           // application/json response

// Validation DSL
validate! { field: rule.modifier, … }

// Auth facade
Auth::user()          // Option<serde_json::Value> for the current request
Auth::check()         // bool

// Cache facade
Cache::get::<T>("key")
Cache::put("key", value, ttl_secs)
Cache::forget("key")

// Error types
AppError, AppResult<T>, OrNotFound
```

---

## Where to go next

| Resource | Contents |
|---|---|
| [docs/STABLE_CORE.md](../STABLE_CORE.md) | Full v1 API contract — every entry point, grep-verified |
| [docs/TIERS.md](../TIERS.md) | Maturity tier for every crate |
| [docs/COOKBOOK.md](../COOKBOOK.md) | Task-oriented recipes with CI-tested snippets |
| [docs/API_PHILOSOPHY.md](../API_PHILOSOPHY.md) | Two-layer architecture: DX vs explicit core |
| `examples/reference-app/` | Full app: auth, CRUD, cache, storage, queue, mail, health, metrics |
| `examples/validated-signup/` | Auto-validating extractor with `ValidatedJson<T>` |
| `examples/auth-demo/` | JWT auth end-to-end |
| `examples/jobs-demo/` | Redis-backed job queue |
