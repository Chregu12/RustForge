# Migration Guide: Laravel to RustForge

This guide is for Laravel PHP developers moving to RustForge. It maps Laravel
concepts to their RustForge equivalents concept by concept, shows you what the
DX layer hides and where Rust surfaces real differences, and links to the
authoritative sources so you can verify every claim.

**Authoritative references:**
- `docs/STABLE_CORE.md` — the v1 API contract and entry points per capability
- `docs/API_PHILOSOPHY.md` — the two-layer framing
- `docs/TIERS.md` — honest maturity tiers for every crate
- `examples/reference-app/` — the flagship real app exercising all stable surfaces

---

## Mental Model: Two Layers

RustForge has a two-layer architecture described in `docs/API_PHILOSOPHY.md`.
Understanding it upfront makes everything else click.

**Layer 1 — Laravel-style DX (the default you write).** `Model!`, `validate!`,
`input()`/`file()`/`has()` request globals, `Auth`/`Cache`/`Mail`/`Storage`/`Queue`
facades. This is the framework's identity. Write the terse, expressive code you
know from Laravel. Some correctness is checked at runtime rather than compile
time (documented caveats below), but in exchange you write far less code.

**Layer 2 — Explicit Rust-native core (the escape hatch).** Typed axum extractors
(`ValidatedJson<T>`, `axum::Json<T>`), `Result`-returning handlers, `AppError`/`AppResult`.
No ambient state, works outside a request scope. Drop to it for background jobs,
CLI tools, tests, or when you want compile-time proof.

Both layers live in the same router. You mix them per handler. Use the DX layer
by default; use the explicit core where you want Rust's full strictness.

---

## The Single Import

```rust
use rf::prelude::*;
```

One line brings in the entire stable surface: routing verbs, `input`/`has`/`file`,
`json`/`view`/`redirect`, `validate!`, `Model!`/`create!`/`find!`/`update!`/`delete!`,
`DB`, `Auth`, `Cache`, `Mail`, `Storage`, `Event`, `Hash`, `AppError`, `AppResult`,
`Collection`, and more. See `docs/STABLE_CORE.md` for the complete list.

You still need `serde` for `#[derive(Serialize, Deserialize)]` and `tokio` for
`#[tokio::main]`.

---

## Concept Map

### Artisan CLI → Forge CLI

| Laravel Artisan | RustForge Forge | Notes |
|----------------|-----------------|-------|
| `php artisan make:model User` | `forge make:model User` | Generates a compiling `Model!` file |
| `php artisan make:controller UserController` | `forge make:controller UserController` | Generates handler stubs |
| `php artisan make:migration create_users_table` | `forge make:migration create_users_table` | Migration file stub |
| `php artisan migrate` | `forge migrate` | Run migrations |
| `php artisan migrate:rollback` | `forge migrate:rollback` | Roll back |
| `php artisan queue:work` | `forge queue:work` | Start queue worker |
| `php artisan cache:clear` | `forge cache:clear` | Clear cache |
| `php artisan route:list` | `forge route:list` | List registered routes |

`forge-cli` is a stable-tier crate; `foundry-cli` (the old name) is kept for
backward compatibility only.

---

### Routing

**Laravel:**
```php
Route::get('/posts', [PostController::class, 'index']);
Route::post('/posts', [PostController::class, 'store']);
Route::middleware('auth')->group(function () {
    Route::put('/posts/{id}', [PostController::class, 'update']);
    Route::delete('/posts/{id}', [PostController::class, 'destroy']);
});
```

**RustForge:**
```rust
use rf::prelude::*;

get("/posts", index);
post("/posts", store);

// Resource shorthand registers index/show/store/update/destroy
resource("/articles", article_controller);

// Build the axum Router (must include capture_request for input() globals)
let app = rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

For protected routes, attach `require_auth` as a route-layer (see Auth section).

The `Route` facade is also available for a fluent style:

```rust
use rf::Route;
Route::get("/posts", index);
Route::post("/posts", store);
```

Route registration goes into source code (not a separate `routes/` directory).
There is no automatic scanning; call the registration functions before
`global_router().build_router()`.

---

### Request Helpers (input / has / file)

**Laravel:**
```php
$title = $request->input('title');
$page  = $request->input('page', 1);
$hasAvatar = $request->has('avatar');
$file  = $request->file('avatar');
```

**RustForge (DX layer):**
```rust
use rf::prelude::*;

async fn store() -> impl axum::response::IntoResponse {
    let title: Option<String> = input("title");
    let page:  Option<usize>  = input("page");   // coerces "2" -> 2 automatically
    let has_avatar = has("avatar");
    let avatar = file("avatar");                  // Option<UploadedFile>
    // ...
}
```

**Requirement:** The `capture_request` middleware must be on the router. Without
it, `input`/`has`/`file`/`all` return `None`/`false`/empty silently — this is a
runtime condition, not a compile error. The reference app and the `global_router()`
scaffolding wire it for you.

For a compile-time guarantee that a field is present, use the explicit layer:

```rust
use rf_request::extractors::RequestExtractor;

async fn store(RequestExtractor(req): RequestExtractor)
    -> impl axum::response::IntoResponse
{
    let title: String = req.require("title")?;  // typed, ?-propagated, no middleware dep
    // ...
}
```

---

### Models and Database (Eloquent → Model! + DB)

#### Declaring a model

**Laravel:**
```php
class Post extends Model
{
    protected $fillable = ['title', 'body', 'author_id'];

    public function author(): BelongsTo
    {
        return $this->belongsTo(Author::class);
    }
}
```

**RustForge:**
```rust
use rf::prelude::*;

Model!(Author { name: String });

Model!(Post {
    title: String,
    body: String,
    author_id: i64,
    belongsTo author: Author,   // relation declared inline
});
```

The `Model!` macro generates the struct, the table mapping, DTO types, and
relation hydration. `hasMany`, `hasOne`, `belongsTo`, and `belongsToMany` are
all supported. See `examples/taskflow/` for a bidirectional relation example with
a foreign-key override.

#### CRUD macros (synchronous via AsyncBridge)

The `create!`/`find!`/`update!`/`delete!` macros run synchronously over an
internal `AsyncBridge` — no `.await` needed, and they are safe inside a Tokio
runtime.

| Laravel | RustForge | Returns |
|---------|-----------|---------|
| `Post::create(['title' => $t])` | `create!(Post, title = t, body = b)` | `Result<Value, _>` |
| `Post::find($id)` | `find!(Post, id)` | `Option<Value>` |
| `Post::findOrFail($id)` | `find!(Post, id).or_404()?` | row or 404 |
| `$post->update(['title' => $t])` | `update!(Post, id, title = t)` | `Result<Value, _>` |
| `Post::destroy($id)` | `delete!(Post, id)` | `Result<u64, _>` |

```rust
// In a handler (no .await on the macros)
async fn store() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(200), body: string }.is_err() {
        return json(serde_json::json!({"error": "validation failed"}))
            .status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
    let title: String = input("title").unwrap_or_default();
    let body:  String = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(created) => json(created).status(axum::http::StatusCode::CREATED),
        Err(e)      => json(serde_json::json!({"error": e.to_string()}))
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn show() -> impl axum::response::IntoResponse {
    let id: i64 = input("id").expect("path param");
    match find!(Post, id) {
        Some(post) => json(post),
        None       => json(serde_json::json!({"error": "not found"}))
            .status(axum::http::StatusCode::NOT_FOUND),
    }
}
```

Source: `examples/blog-slice/src/main.rs`, `examples/rest-crud-resource/src/main.rs`.

#### Fluent QueryBuilder (DB facade, async)

For queries beyond simple CRUD, drop to the `DB` facade. Unlike the ORM macros,
`QueryBuilder` calls are async:

```rust
use rf::prelude::*;

// Equivalent to: Post::where('user_id', $uid)->where('title', 'LIKE', "%$q%")->paginate(10)
let results = DB::table("posts")
    .where_eq("user_id", user_id)
    .where_like("title", format!("%{q}%"))
    .order_by("id", "asc")
    .paginate(10, page)
    .await?;
```

**Database support:** SQLite by default (zero-config, in-memory). Set
`DATABASE_URL=postgres://user:pass@host/db` and the `DB` facade plus all ORM
macros route to Postgres automatically. The full CRUD cycle is CI-verified
against real Postgres 16. Caveat: the primary key column must be named `id`;
`NUMERIC`/`DECIMAL` columns must be cast to `TEXT` or `FLOAT8` in queries.

---

### Validation (FormRequest → validate!)

**Laravel FormRequest:**
```php
class CreatePostRequest extends FormRequest
{
    public function rules(): array
    {
        return [
            'title'   => 'required|string|max:200',
            'body'    => 'required|string',
            'email'   => 'required|email',
        ];
    }
}
```

**RustForge DX layer (`validate!` macro):**

```rust
use rf::prelude::*;

async fn store() -> impl axum::response::IntoResponse {
    // validate! reads from the current request (requires capture_request middleware)
    // Returns Err(ValidationErrors) on failure; the framework automatically
    // responds 422 with per-field structured errors.
    if validate! { title: string.max(200), body: string, email: email }.is_err() {
        return json(serde_json::json!({"error": "validation failed"}))
            .status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
    // ...
}
```

**RustForge explicit layer (`ValidatedJson<T>` extractor):**

```rust
use rf::prelude::*;

// The derive generates a full Validate impl; validation runs inside the extractor.
// A missing or invalid field is rejected with 422 before the handler body runs.
Model!(Post {
    title: String,
    body: String,
    @ title: min(1), max(200)
    @ body: min(1)
    validated
});

async fn store(ValidatedJson(body): ValidatedJson<CreatePost>)
    -> impl axum::response::IntoResponse
{
    // body.title is already validated — no manual validate!() call needed
    match create!(Post, title = body.title, body = body.body) {
        Ok(row) => json(row).status(axum::http::StatusCode::CREATED),
        Err(e)  => json(serde_json::json!({"error": e.to_string()}))
            .status(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

Source: `examples/validated-signup/src/main.rs`.

**Built-in rules (48+):** `string`, `integer`, `float`, `boolean`, `email`,
`url`, `uuid`, `date`, `min`, `max`, `between`, `required`, `nullable`,
`confirmed`, `in`, `not_in`, `regex`, `alpha`, `alpha_num`, `numeric`, `json`,
`image`, `mimes`, `max_file_size`, `kb(n)`, `mb(n)`, and more. See
`docs/STABLE_CORE.md §3 Validation`.

---

### Authentication (Auth guard → require_auth + JWT)

**Laravel:**
```php
// routes/api.php
Route::middleware('auth:sanctum')->group(function () {
    Route::get('/me', [UserController::class, 'me']);
});

// In controller
$user = Auth::user();
```

**RustForge:**

Protect a route by adding `require_auth` (or `require_auth_with`) as a
route-layer. It reads `Authorization: Bearer <token>`, validates the JWT, sets
up the per-request `Auth` scope, and rejects with JSON 401 on missing/invalid/
expired tokens.

```rust
use rf::prelude::*;
use rf_auth::{require_auth_with, Claims, JwtManager, PasswordHasher};
use axum::{middleware, routing::get, Router, extract::Extension};

// At startup: create a JwtManager with your secret
let jwt = std::sync::Arc::new(JwtManager::new(&jwt_secret)?);

// Protect specific routes
let protected = Router::new()
    .route("/me", get(me_handler))
    .route_layer(middleware::from_fn(require_auth_with(jwt.clone())));

// Inside a protected handler
async fn me_handler(Extension(claims): Extension<Claims>)
    -> impl axum::response::IntoResponse
{
    // Auth::user() returns Some(...) inside a require_auth scope
    json(serde_json::json!({ "user_id": claims.sub }))
}
```

**Registration / login flow:**

```rust
use rf::prelude::*;          // brings in Hash
use rf_auth::JwtManager;

// Hash a password — Hash::make returns a String (not Result), no .await
let hash: String = Hash::make(&plaintext_password);

// Verify — Hash::check returns a bool
let ok: bool = Hash::check(&plaintext_password, &hash);

// Issue a JWT (requires a JwtManager instance created at startup)
let jwt = JwtManager::new(&jwt_secret)?;
let token = jwt.generate_token(user_id, &email)?;
```

For configurable cost / algorithm, use the `PasswordHasher` instance from
`rf-auth` directly:

```rust
use rf_auth::PasswordHasher;

let hasher = PasswordHasher::bcrypt(12)?;   // bcrypt with cost 12
let hash = hasher.hash(&plaintext_password)?;
let ok   = hasher.verify(&plaintext_password, &hash)?;
```

Source: `examples/auth-demo/src/main.rs`,
`examples/reference-app/src/main.rs`.

**CSRF:** `csrf_token()` and `csrf_field()` generate tokens. The framework
validates CSRF on `Authorization` header, `application/x-www-form-urlencoded`,
and `multipart/form-data` requests. For pure JSON APIs, the bearer-token
pattern replaces CSRF.

---

### Facades (Cache / Mail / Storage / Event)

All facades are synchronous (they use the internal `AsyncBridge` to bridge
async drivers). You call them from anywhere — handlers, jobs, CLI — with no
`.await`.

#### Cache

**Laravel:**
```php
Cache::put('key', $value, 3600);
$value = Cache::get('key');
Cache::forget('key');
$users = Cache::remember('users', 300, fn() => User::all());
```

**RustForge:**
```rust
use rf::prelude::*;

// Defaults to MemoryCache (in-process). Set REDIS_URL env var to switch to Redis.
let _ = Cache::put("posts:list", &data, 60u64);  // ttl in seconds
let cached = Cache::get::<serde_json::Value>("posts:list");
Cache::forget("posts:list");

// Cache-aside remember
let posts = Cache::remember("posts:published", 300u64, || {
    // closure returns the value to cache
    serde_json::to_value(&all_posts).unwrap()
});
```

Source: `examples/facades-demo/src/main.rs`.

#### Mail

**Laravel:**
```php
Mail::to($user)->send(new WelcomeEmail($user));
```

**RustForge:**
```rust
use rf::prelude::*;
use rf_mail::{Address, MailBuilder, Mailable};

struct WelcomeEmail { user: User }

impl Mailable for WelcomeEmail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .from(Address::new("hello@example.com"))
            .to(Address::new(&self.user.email))
            .subject("Welcome!")
            .html(format!("<p>Hi {}!</p>", self.user.email))
    }
}

// Synchronous; no .await
Mail::send(WelcomeEmail { user: user.clone() })?;
// or
Mail::to(&user.email).subject("Welcome!").body("Hi!").send()?;
```

Defaults to `FileMailer` (writes `.eml` files to disk). Set `SMTP_HOST` to use
real SMTP via `lettre`.

Source: `examples/mail-demo/src/main.rs`, `examples/reference-app/src/main.rs`.

#### Storage

**Laravel:**
```php
Storage::put('uploads/' . $filename, $data);
$url = Storage::url('uploads/' . $filename);
Storage::delete('uploads/' . $filename);
```

**RustForge:**
```rust
use rf::prelude::*;

// Defaults to MemoryStorage. Configure LocalStorage or S3Storage at startup.
Storage::put(&path, bytes_vec)?;           // Vec<u8>
let bytes = Storage::get(&path)?;          // Vec<u8>
Storage::delete(&path)?;
let exists = Storage::exists(&path);       // bool
let url = Storage::url(&path);             // String
```

`LocalStorage` is path-traversal-safe. `S3Storage` requires the `s3` feature and
`AWS_*` environment variables.

#### Events

```rust
use rf::prelude::*;

Event::listen("user.registered", |data| {
    println!("User registered: {:?}", data);
});
Event::dispatch("user.registered", serde_json::json!({"email": "x@example.com"}));
```

---

### Blade Templates → rf-blade (beta)

**Status: beta.** The `rf-blade` crate provides a Blade-like template engine
built on Tera. It supports `{{ var }}`, `@if`/`@else`/`@foreach` directives,
`@extends`/`@section` template inheritance, and `@csrf`. It does not implement
every Blade feature; check `docs/TIERS.md` for the maturity detail.

Templates live in `resources/views/<name>.blade.html`. Use the `view()` helper
to render:

```rust
use rf::prelude::*;

async fn home() -> impl axum::response::IntoResponse {
    view("home", serde_json::json!({ "user_name": "Alice" }))
}
```

The response helper `view(name, data)` renders
`resources/views/<name>.blade.html` and returns `text/html`. For a raw Tera
render (without the response wrapper), use `rf_blade::render(name, data)`.

Source: `examples/phase12-blog/` (note: also uses experimental crates
`rf-vite`/`rf-livereload`/`rf-cms` — see `docs/TIERS.md`).

---

### Queues and Jobs (Queue/Job → rf-queue / rf-jobs)

RustForge has two queue back-ends:

| | `rf-queue` (MemoryQueue + Worker) | `rf-jobs` (Redis-backed) |
|-|-------------------------------------|--------------------------|
| External service | None — in-process | Redis required |
| Tier | stable | stable |
| Use for | Dev, tests, simple apps | Production multi-worker setups |

#### In-process queue (rf-queue)

**Laravel:**
```php
dispatch(new ProcessUpload($file));
```

**RustForge:**
```rust
use rf_queue::{Job, Jobs, MemoryQueue, Worker};
use async_trait::async_trait;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct ProcessUpload { file: String }

#[async_trait]
impl Job for ProcessUpload {
    async fn handle(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Processing {}", self.file);
        Ok(())
    }
}

// At startup: install the global queue
let queue = std::sync::Arc::new(MemoryQueue::new());
Jobs::set_queue(queue.clone());

// Drain in a background task
tokio::spawn(async move {
    Worker::new(queue).run().await;
});

// Dispatch from a handler (synchronous)
ProcessUpload { file: "photo.jpg".into() }.dispatch_now()?;
```

Source: `examples/jobs-offline/src/main.rs`.

#### Redis-backed queue (rf-jobs)

```rust
use rf_jobs::{dispatch, Job, JobContext, JobResult, QueueManager};

#[async_trait::async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // ... send email
        Ok(())
    }
    fn queue(&self) -> &str { "emails" }
    fn max_attempts(&self) -> u32 { 3 }
}

dispatch(&queue_manager, SendEmailJob { user_id: 42 })?;
```

Source: `examples/jobs-demo/src/main.rs`.

---

### Error Handling (exceptions → AppError / Result / ?)

**Laravel:**
```php
public function show($id)
{
    $post = Post::findOrFail($id);   // throws ModelNotFoundException → 404
    return response()->json($post);
}
```

**RustForge DX (`.or_404()`):**
```rust
use rf::prelude::*;

async fn show(axum::extract::Path(id): axum::extract::Path<i64>)
    -> AppResult<impl axum::response::IntoResponse>
{
    let post = find!(Post, id).or_404()?;  // None → 404 JSON, Some → unwrap
    Ok(json(post))
}
```

`AppResult<T>` is `Result<T, AppError>`. `AppError` implements `IntoResponse`
(renders RFC 7807 JSON) and covers `400`, `401`, `403`, `404`, `422`, `429`,
`500`. Use `?` freely in `AppResult`-returning handlers.

```rust
// Explicit error
return Err(AppError::not_found("Post not found"));
return Err(AppError::unauthorized("Token expired"));
return Err(AppError::validation(errors));
```

---

## What Is Genuinely Different in Rust

These differences are not paper-over-able; they are Rust. The DX layer reduces
the friction but does not eliminate the underlying reality.

### Types are compile-time

`input("field")` returns `Option<T>` — you must handle `None`. There is no
dynamic typing, no `mixed`, no late-binding. Untyped `$request->input('field')`
becomes typed `input::<String>("field")`.

### `Result` instead of `try/catch`

Rust propagates errors with `?`. There are no thrown exceptions. Handlers return
`AppResult<impl IntoResponse>` and use `?` to surface errors as JSON responses.
You get the same end-user behavior; you write `?` instead of wrapping in
`try/catch`.

### Async is explicit (but DX macros hide it)

The `create!`/`find!`/`update!`/`delete!` macros are synchronous (they use an
internal `AsyncBridge`). The `DB::table(...)` `QueryBuilder` is async and
requires `.await`. The `Cache`/`Mail`/`Storage` facades are synchronous. When a
call requires `.await`, the compiler tells you.

### Middleware must be wired explicitly

The `capture_request` middleware is required for `input()`/`has()`/`file()`/`all()`.
The `session_scope` middleware is required for per-request `Session` isolation.
`require_auth`/`require_auth_with` is required for `Auth::user()` to return
`Some(...)`. Forgetting a middleware is a runtime gap, not a compile error;
cover it with integration tests.

### Session scope matters

Without `session_scope`, `SessionFacade` falls back to a single process-local
session shared by all concurrent callers — a correctness hazard for production.
Always add `session_scope` to routes that use `Session`.

### `Auth::user()` returns `Option`

Outside a `require_auth` scope, `Auth::user()` returns `None`. Protect routes
with `require_auth`/`require_auth_with` before calling `Auth::user()`.

---

## From RustForge 0.x (foundry_* → rf_*)

If you are migrating from RustForge 0.x which used `foundry_*` crate names:

```rust
// Old (0.x) — these names are dead; do not use them
use foundry_orm::prelude::*;
use foundry_request::Request;
use foundry_auth::JwtManager;
use foundry_queue::Queue;

// Current (1.0) — all crate names use the rf_ prefix
use rf::prelude::*;          // umbrella — preferred for application code
use rf_orm::DB;              // explicit crate import when needed
use rf_request::input;       // explicit
use rf_auth::JwtManager;
use rf_queue::MemoryQueue;
```

A mechanical rename suffices for most code:

```bash
find . -name "*.rs" -exec sed -i 's/foundry_/rf_/g' {} +
```

The nine old `rf-auth-facade`, `rf-cache-facade`, `rf-db-facade`, etc. crates
in `crates/` are non-workspace stubs (superseded in Phase 20); do not reference
them in new code.

---

## Laravel Feature → RustForge Status Quick Reference

| Laravel feature | RustForge equivalent | Tier |
|----------------|---------------------|------|
| Eloquent ORM (basic CRUD) | `Model!` + `create!`/`find!`/`update!`/`delete!` | stable |
| Eloquent relations | `hasMany`, `hasOne`, `belongsTo`, `belongsToMany` in `Model!` | stable |
| Query Builder (`DB::table`) | `DB::table(...).where_eq(...).paginate()` | stable |
| FormRequest validation | `validate!` DSL / `ValidatedJson<T>` | stable |
| Auth facades | `Auth::user()`, `Auth::check()`, `Auth::login()` | stable |
| JWT tokens | `JwtManager::generate_token()` + `require_auth` middleware | stable |
| Cache facade | `Cache::get/put/forget/remember` | stable |
| Mail facade + Mailables | `Mail::send(mailable)`, `impl Mailable` | stable |
| Storage facade | `Storage::put/get/delete/url` | stable |
| Queue + Jobs | `rf-queue` (MemoryQueue) / `rf-jobs` (Redis) | stable |
| Event system | `Event::dispatch/listen` | stable |
| CSRF protection | `csrf_token()` / `csrf_field()` | stable |
| Pagination | `QueryBuilder::paginate(per_page, page)` | stable |
| Health checks | `rf-health` (`HealthChecker`, `health_router`) | stable |
| Blade templates | `rf-blade` (Tera-based, not full Blade parity) | **beta** |
| Socialite (OAuth social login) | `rf-socialite` | **beta** |
| GraphQL | `rf-graphql` (async-graphql 7) | **beta** |
| Nova admin panel | `rf-nova` | **experimental** |
| Vite asset pipeline | `rf-vite` | **experimental** |
| Telescope debug dashboard | `rf-telescope` | **experimental** |
| Breeze auth scaffolding | `rf-breeze` | **experimental** |

See `docs/TIERS.md` for the complete 127-crate roster with justifications.

---

## Next Steps

- **[Quick Start](Quick-Start)** — build your first RustForge app in minutes
- **[Examples](Examples)** — tour of the real example apps
- **[API Documentation](API-Documentation)** — detailed API reference
- **[Installation](Installation)** — add RustForge to Cargo.toml
- `docs/STABLE_CORE.md` — the definitive v1 API contract
- `docs/COOKBOOK.md` — task-oriented recipes with CI-verified snippets

*Questions? Open an issue on [GitHub](https://github.com/Chregu12/RustForge/issues).*
