# RustForge — Laravel-Style DX Reference

RustForge's identity is the **Laravel-style DX layer**: terse handlers, `Model!` / `validate!`,
global facades, and request helpers that let you write a full application in far less code than the
explicit equivalent. Underneath that surface sits a fully **explicit Rust-native core** you can always
reach when you want compile-time strictness.

This page documents the DX layer — the default you write day to day.
For the full type-level reference (function signatures, return types, crate locations) see
[API-Documentation.md](API-Documentation.md).
For stability tiers see [docs/TIERS.md](../TIERS.md) and [docs/STABLE_CORE.md](../STABLE_CORE.md).

---

## The single import

```rust
use rf::prelude::*;
```

`rf::prelude` re-exports every stable DX item listed on this page. One line in your handler file is
sufficient — no direct crate dependencies beyond `rf`, `serde`, and `tokio`.

---

## Routing

Register routes with free functions; the framework accumulates them on a process-global router.

```rust
use rf::prelude::*;

get("/articles",         list_handler);
post("/articles",        store_handler);
get("/articles/{id}",   show_handler);
put("/articles/{id}",   update_handler);
delete("/articles/{id}", destroy_handler);
patch("/articles/{id}", patch_handler);
```

Build the axum app at startup, wiring the `capture_request` middleware so the request globals
(`input` / `has` / `file` / `all`) work in every handler:

```rust
fn build_app() -> axum::Router {
    // Register routes above, then:
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, build_app()).await.unwrap();
}
```

Source: `examples/blog-slice/src/main.rs`, `examples/rest-crud-resource/src/main.rs` (Stable).

---

## Request globals

Once `capture_request` is in the router layer, you can read the current request from any handler
with no function argument:

```rust
use rf::prelude::*;

async fn store() -> impl axum::response::IntoResponse {
    // Read a body or query field (coerces "2" -> 2 for numeric types).
    let title: Option<String> = input("title");
    let page:  Option<usize>  = input("page");   // "?page=2" -> Some(2)
    let id:    i64            = input("id").unwrap_or(0);  // path param

    // Check presence (does not consume the value).
    if has("draft") { /* field was present */ }

    // Access an uploaded file.
    let avatar = file("avatar"); // Option<UploadedFile>

    // All fields as a map.
    let fields = all(); // HashMap<String, serde_json::Value>

    json(serde_json::json!({ "ok": true }))
}
```

**Runtime caveat:** `input` / `has` / `file` / `all` read a **per-request task-local** that is
populated by the `capture_request` middleware. Called outside a `capture_request`-wrapped handler
they return `None` / `false` / empty map **silently** — the compiler cannot catch this. Always wire
`capture_request` and cover the risk with integration tests. See
[API_PHILOSOPHY.md](../API_PHILOSOPHY.md) for a full discussion.

---

## ORM — Model! macro and CRUD

### Declaring a model

```rust
use rf::prelude::*;

// Concise form — field types default to String.
Model!(Post: title, body);

// Explicit form — use when you need non-String types.
Model!(Post {
    title:   String,
    body:    String,
    user_id: i64,
});

// With relations.
Model!(Article {
    title:    String,
    body:     String,
    author_id: i64,
    belongsTo author: Author,
});

// With built-in scopes.
Model!(Task {
    title:   String,
    status:  String,
    scope open: where("status", "open"),
});
```

`Model!` generates the entity struct, a `create` DTO, a table mapping (plural snake-case table
name), and a `Model` trait implementation. Every field maps to a real database column.

### CRUD macros

```rust
// INSERT — returns Result<serde_json::Value, String>.
// The returned Value includes the auto-assigned `id` field.
let post = create!(Post, title = "Hello", body = "World")?;
let post_id = post["id"].as_i64().unwrap();

// SELECT by PK — returns Result<Option<serde_json::Value>, String>.
let post = find!(Post, post_id)?;  // None if not found

// UPDATE by PK — returns Result<usize, String> (affected rows).
let affected = update!(Post, post_id, title = "Updated")?;
if affected == 0 { /* row not found */ }

// DELETE by PK — returns Result<usize, String> (affected rows).
let deleted = delete!(Post, post_id)?;
if deleted == 0 { /* row not found */ }
```

The `create!` / `update!` / `delete!` macros generate parameterised SQL. The `update!` and `delete!`
macros return the number of affected rows so you can distinguish "not found" (0) from success (>0).

### Eloquent-style methods

In addition to the macros, generated models expose Eloquent-style async methods:

```rust
// SELECT all rows.
let posts = Post::all().await?;  // Vec<Post>

// SELECT by PK.
let post = Post::find(42).await?;  // Option<Post>

// Eager-load relations (N+1-free batched load).
let articles = Article::with(&["author"]).get().await?;
let article  = Article::with(&["author"]).r#where("id", id).get().await?;

// Paginate.
let page = Task::paginate(10, 1).await?;  // PaginatedResult { data, total, per_page, ... }

// Named scope.
let open_tasks = Task::open().get().await?;
```

### Database backends

The ORM macros and `DB` facade run on **SQLite** (default, in-memory or file path) or **Postgres**.
The backend is selected by `DATABASE_URL`:

| `DATABASE_URL` value | Backend |
|---|---|
| Absent / empty | In-memory SQLite (rusqlite) — data lost on exit |
| A file path | Persistent SQLite file |
| `postgres://...` or `postgresql://...` | Postgres via sqlx PgPool |

```sh
cargo run -p my-app                                    # in-memory SQLite
DATABASE_URL=./app.db cargo run -p my-app              # SQLite file
DATABASE_URL=postgres://user:pass@localhost/db cargo run -p my-app  # Postgres
```

**Postgres caveats:**
- The primary key column must be named `id` (the framework convention; `RETURNING id` is appended
  on INSERT).
- `NUMERIC`/`DECIMAL` columns are not decoded to JSON yet — cast to `TEXT` or `FLOAT8` in the
  query.

---

## DB query builder

For queries that go beyond the CRUD macros, use the `DB` facade directly:

```rust
use rf::prelude::*;

// Raw DDL / DML (idempotent migrations).
DB::statement("CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT)")
    .expect("migration");

// Fluent builder — returns Vec<serde_json::Value>.
let rows = DB::table("posts")
    .r#where("user_id", user_id)
    .order_by("created_at", "desc")
    .limit(20)
    .get()
    .await?;

// Search with LIKE (safe: escapes %, _, \ in the user term).
let results = DB::table("posts")
    .where_like_escaped("title", &search_term)
    .get()
    .await?;

// First row or None.
let post = DB::table("posts").r#where("id", id).first().await?;

// Raw SELECT with positional parameters.
let rows = DB::select(
    "SELECT id, email FROM users WHERE email = ?",
    &[serde_json::Value::String(email.clone())],
)?;

// Raw INSERT.
DB::insert(
    "INSERT INTO files (path, filename) VALUES (?, ?)",
    &[serde_json::Value::String(path), serde_json::Value::String(name)],
)?;

// Check which backend is active.
let is_pg = DB::connection_name().starts_with("postgres");

// Switch to a different URL at runtime.
DB::connection("postgres://user:pass@host/db")?;
```

Source: `examples/reference-app/src/main.rs`, `examples/taskflow/src/main.rs` (Stable).

---

## Validation

### Inline `validate!` DSL

Use `validate!` inside a handler to short-circuit with a 422 on bad input:

```rust
use rf::prelude::*;
use axum::http::StatusCode;

async fn store() -> impl axum::response::IntoResponse {
    if validate! {
        title:     string.max(200),
        body:      string,
        author_id: int.min(1),
        age:       int.between(18, 120),
        email:     email,
        website:   url.optional,
        avatar:    image.max(mb(5)).min(kb(100)),  // upload constraints
    }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY);
    }

    let title: String = input("title").unwrap_or_default();
    // ... proceed with valid data
    json(serde_json::json!({ "ok": true })).status(StatusCode::CREATED)
}
```

The type prefix (`string`, `int`, `email`, `image`, `file`, ...) resolves ambiguity between
string-length and numeric-value modifiers. The macro reads from the `capture_request` task-local.

**Built-in rules (selection):** `string`, `int`, `float`, `boolean`, `email`, `url`, `uuid`,
`date`, `min(N)`, `max(N)`, `between(lo, hi)`, `required`, `optional`, `nullable`, `confirmed`,
`in`, `not_in`, `regex`, `alpha`, `alpha_num`, `numeric`, `json`, `image`, `file`,
`mimes`, `max_file_size`, `kb(N)`, `mb(N)`.

### Per-field `@` rules in `Model!`

Add `validated` to a `Model!` declaration and attach `@` rules per field. The generated DTO then
implements `rf_validation::Validate`, making it usable with the `ValidatedJson` extractor:

```rust
use rf::prelude::*;
use rf_validation::ValidatedJson;

Model!(User {
    validated,
    username: String @ min(3) max(20) alphanumeric,
    email:    String @ email message("Please enter a valid email address"),
    password: String @ min(8),
    zipcode:  String @ regex("^\\d{5}$"),
});

// ValidatedJson validates before the handler body runs.
// Invalid body -> 422 with per-field errors map; the handler body is never entered.
async fn signup(ValidatedJson(dto): ValidatedJson<CreateUser>) -> impl axum::response::IntoResponse {
    (axum::http::StatusCode::CREATED, axum::Json(serde_json::json!({
        "username": dto.username,
        "email":    dto.email,
    })))
}
```

Available `@` rules: `min(N)`, `max(N)`, `email`, `url`, `uuid`, `ip`, `regex("…")`,
`alpha`, `alphanumeric`, `starts_with("…")`, `ends_with("…")`, `range(lo, hi)`,
`message("…")` (custom error text override).

### 422 error shape

Both paths produce the same structured JSON on failure:

```json
{
  "errors": {
    "email":    [{ "code": "email",   "message": "Please enter a valid email address" }],
    "password": [{ "code": "min",     "message": "must be at least 8 characters" }]
  }
}
```

Source: `examples/validated-signup/src/main.rs`, `examples/rest-crud-resource/src/main.rs` (Stable).

---

## Auth facade and JWT middleware

### Protecting routes

The preferred pattern is `require_auth_with` (when `JwtManager` lives in app state) or
`require_auth` (when injected as an `Extension`):

```rust
use rf_auth::{require_auth_with, Claims, JwtManager, PasswordHasher};
use std::sync::Arc;

let jwt = Arc::new(JwtManager::new("your-secret-min-32-characters")?);

// Protected routes — route_layer ensures auth runs before body extraction.
// A missing/invalid/expired JWT returns JSON 401 before the handler body runs.
let protected = axum::Router::new()
    .route("/me",           axum::routing::get(me_handler))
    .route("/posts",        axum::routing::post(create_handler))
    .route("/posts/{id}",   axum::routing::delete(delete_handler))
    .route_layer(axum::middleware::from_fn(require_auth_with(jwt.clone())));
```

`require_auth` / `require_auth_with` both:
1. Read the `Authorization: Bearer <jwt>` header (headers only — never the body).
2. Validate the token signature and expiry via `JwtManager`.
3. On success, open a fresh per-request auth scope and inject `Extension<Claims>`.
4. On failure, short-circuit with a JSON 401 before any body extractor runs.

### Reading the authenticated user

```rust
use rf_auth::Claims;
use axum::extract::Extension;

// Option 1: Extension<Claims> extractor (explicit, compile-time safe).
async fn me_handler(Extension(claims): Extension<Claims>) -> impl axum::response::IntoResponse {
    json(serde_json::json!({
        "user_id": claims.user_id,
        "email":   claims.sub,
        "roles":   claims.roles,
    }))
}

// Option 2: Auth facade (DX, task-local — requires require_auth middleware in stack).
use rf::prelude::*;

async fn profile() -> impl axum::response::IntoResponse {
    if Auth::check() {
        let user = Auth::user();   // Option<serde_json::Value>
        let id   = Auth::id();     // Option<i64>
        json(user.unwrap_or_default())
    } else {
        json(serde_json::json!({ "error": "unauthenticated" }))
            .status(axum::http::StatusCode::UNAUTHORIZED)
    }
}
```

`Auth::user()` returns `None` outside a `require_auth` / `require_auth_with` scope. The `Extension<Claims>` form is compile-time safe and is the explicit-core alternative.

### Issuing JWT tokens

```rust
use rf_auth::{Claims, JwtManager};

let jwt = JwtManager::new("your-secret-min-32-characters")?;

// 24-hour token with roles.
let claims = Claims::new(user_id as i32, email.clone(), vec!["user".to_string()], 24);
let token  = jwt.generate_token(&claims)?;
```

### Password hashing

```rust
use rf::Hash;  // re-export of rf_global_helpers::Hash (bcrypt, simple API)

let hash  = Hash::make("password123");     // String
let valid = Hash::check("password123", &hash); // bool

// Fine-grained bcrypt with cost control (from rf-auth):
use rf_auth::PasswordHasher;

let hasher = PasswordHasher::bcrypt(12)?;
let hash   = hasher.hash("password123")?;
let ok     = hasher.verify_timing_safe("password123", &hash)?;
```

Source: `examples/reference-app/src/main.rs` (Stable).

---

## Cache facade

The default backend is `MemoryCache` (zero-config, in-process). Switching to Redis requires
configuring the global manager at startup (see API-Documentation.md).

```rust
use rf::prelude::*;

// Store with TTL (accepts u64 seconds or Duration).
Cache::put("posts:list", posts_value, 60u64)?;

// Retrieve (turbofish the expected type).
let cached: Option<serde_json::Value> = Cache::get::<serde_json::Value>("posts:list")?;

// Cache-aside pattern.
let users = Cache::remember("users:all", 300u64, || async {
    Ok(User::all().await?)
})?;

// Store forever.
Cache::forever("settings", settings_value)?;

// Atomic check-and-set (returns true if the key was absent).
let added = Cache::add("lock:export", "1", 30u64)?;

// Delete one key.
Cache::forget("posts:list")?;

// Flush everything.
Cache::flush()?;

// Check presence.
if Cache::has("posts:list")? { /* ... */ }

// Pull (get + delete atomically).
let value: Option<String> = Cache::pull::<String>("temp_key")?;

// Counters.
Cache::increment("request_count", 1)?;
Cache::decrement("request_count", 1)?;
```

Source: `examples/reference-app/src/main.rs`, `examples/facades-demo/src/main.rs` (Stable).

---

## Mail facade

The default transport is `FileMailer` (writes `.eml` files to `$MAIL_MAILBOX` or
`/tmp/rustforge-mailbox`). Set `SMTP_HOST` to use real SMTP.

```rust
use rf::prelude::*;
use rf_mail::{Address, MailBuilder, Mailable};

// Implement Mailable to describe a message.
struct WelcomeMail { to: String, name: String }

impl Mailable for WelcomeMail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new(&self.to))
            .subject("Welcome!")
            .text(format!("Welcome, {}!", self.name))
    }
}

// Send via the synchronous facade (no .await).
Mail::send(WelcomeMail { to: "user@example.com".into(), name: "Alice".into() })?;

// Recipient-first builder form.
Mail::to("user@example.com").send(WelcomeMail {
    to: "user@example.com".into(),
    name: "Bob".into(),
})?;
```

Source: `examples/reference-app/src/main.rs`, `docs/COOKBOOK.md` recipe 10 (Stable).

---

## Storage facade

The default backend is `MemoryStorage` (in-process). Local-filesystem and S3 are also available.

```rust
use rf::prelude::*;

// Write bytes.
Storage::put("uploads/avatar.png", bytes)?;

// Read bytes.
let data: Vec<u8> = Storage::get("uploads/avatar.png")?;

// Delete.
Storage::delete("uploads/avatar.png")?;

// Check existence.
if Storage::exists("uploads/avatar.png") { /* ... */ }

// Public URL for the stored path.
let url = Storage::url("uploads/avatar.png")?;
```

Source: `examples/reference-app/src/main.rs` (Stable).

---

## Queue and background jobs

The in-process path (`rf-queue` / `MemoryQueue`) requires no external services. For
Redis-backed production workers use `rf-jobs` (also stable, requires live Redis).

```rust
use async_trait::async_trait;
use rf_queue::{Job, Jobs, MemoryQueue, Queue, QueueError, Worker};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// 1. Define a job.
#[derive(Serialize, Deserialize, Clone)]
struct SendWelcomeJob { email: String }

#[async_trait]
impl Job for SendWelcomeJob {
    async fn handle(&self) -> Result<(), QueueError> {
        println!("sending welcome to {}", self.email);
        Ok(())
    }
    fn job_type(&self) -> &'static str { "send_welcome" }
}

// 2. At startup: install the global queue and start a background worker.
let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
Jobs::set_queue(queue.clone());

let worker = Worker::new(queue).register::<SendWelcomeJob>();
tokio::spawn(async move { worker.start().await.ok(); });

// 3. Dispatch from anywhere (no queue handle needed at the call site).
Jobs::dispatch(SendWelcomeJob { email: "user@example.com".into() })?;

// Or directly on the job (also sync, no .await):
SendWelcomeJob { email: "user@example.com".into() }.dispatch_now()?;
```

Source: `examples/jobs-offline/src/main.rs`, `examples/reference-app/src/main.rs` (Stable).

---

## Event facade

```rust
use rf::prelude::*;  // Event re-exported as rf_events::EventFacade

// Register a listener (process-global, sync).
Event::listen("user.created", |data: &serde_json::Value| {
    println!("user created: {:?}", data);
});

// Dispatch (sync).
Event::dispatch("user.created", serde_json::json!({ "id": 42, "email": "u@example.com" }))?;

// Check and clean up.
if Event::has_listeners("user.created") { /* ... */ }
Event::forget("user.created");
```

---

## Response helpers

```rust
use rf::prelude::*;
use axum::http::StatusCode;

// JSON response (any Serialize value).
json(serde_json::json!({ "id": 1 }))

// Set status code.
json(data).status(StatusCode::CREATED)

// No-body responses.
Response::no_content()  // 204

// HTML view (renders resources/views/<name>.blade.html).
view("posts.index", context)

// Redirect.
redirect("/dashboard")

// Redirect to previous page.
back()
```

---

## Error handling

```rust
use rf::prelude::*;

// AppResult<T> = Result<T, AppError>. Use as a handler return type for ?-propagation.
async fn show_post(axum::extract::Path(id): axum::extract::Path<i64>)
    -> rf_core::AppResult<impl axum::response::IntoResponse>
{
    let post = find!(Post, id)?
        .or_404()?;   // Option<T>::or_404() converts None to AppError::NotFound -> 404
    Ok(json(post))
}

// AppError variants render RFC 7807 JSON automatically:
// AppError::NotFound { resource }  -> 404
// AppError::Unauthorized           -> 401
// AppError::Forbidden { reason }   -> 403
// AppError::BadRequest { message } -> 400
// AppError::Validation(_)          -> 422
// AppError::Conflict { message }   -> 409
// AppError::Internal(_)            -> 500
```

---

## Helpers

```rust
use rf::prelude::*;

// Password hashing (bcrypt, simple API).
let hash  = Hash::make("password");
let valid = Hash::check("password", &hash);

// CSRF.
let token = csrf_token();   // UUID v4 string
let field = csrf_field();   // HTML hidden-input string

// Laravel-style Collection.
let names: Vec<String> = collect(users.iter())
    .map(|u| u.name.clone())
    .filter(|n| !n.is_empty())
    .to_vec();
```

---

## Explicit-core alternative (when to drop down)

The DX layer's globals are silent at runtime when middleware is absent. For library code,
background tasks, CLI tools, or handlers where you want a compile-time guarantee of field
presence, drop to the explicit core:

```rust
use rf_request::extractors::RequestExtractor;

// Compiler knows exactly what this handler needs — no ambient state.
async fn create(RequestExtractor(req): RequestExtractor)
    -> axum::response::Result<impl axum::response::IntoResponse>
{
    let title: String = req.require("title")?;  // compile-time typed, ?-propagated
    let body:  String = req.require("body")?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}
```

Or mix: use `ValidatedJson<T>` for compile-time body validation while keeping `input("page")` for
optional query params. The `capture_request` middleware re-inserts the body so both can read the
same request. See [API_PHILOSOPHY.md](../API_PHILOSOPHY.md) for the full two-layer framing.

---

## See also

- [API-Documentation.md](API-Documentation.md) — reference-style page (types, signatures, crate locations)
- [docs/STABLE_CORE.md](../STABLE_CORE.md) — the v1 API contract and full entry-point table
- [docs/API_PHILOSOPHY.md](../API_PHILOSOPHY.md) — two-layer framing and honest trade-offs
- [docs/TIERS.md](../TIERS.md) — maturity tiers for every crate
- [docs/COOKBOOK.md](../COOKBOOK.md) — task-oriented recipes with CI-verified snippets
- `examples/reference-app/` — the flagship app (auth + CRUD + cache + storage + queue + mail + health)
- `examples/blog-slice/` — minimal vertical slice
- `examples/rest-crud-resource/` — five-verb CRUD with relations
- `examples/validated-signup/` — `ValidatedJson` + `@` DSL end-to-end
