# RustForge

**A Laravel-style application framework core for Rust.**

RustForge brings Laravel-familiar ergonomics — `Model!`, `validate!`, global facades, and
argument-less handlers — to native Rust, compiled to a single binary with Tokio throughput.

[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange)]()
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)]()

---

## Four pillars

| Pillar | What it means |
|--------|---------------|
| **Write less** | `Model!(Post: title, body)` generates the struct, table mapping, CRUD macros, typed reads, relation loaders, query scopes, and validated DTOs in one line. |
| **Hide async** | Argument-less handlers read the current request through `input()`/`file()`/`has()` globals set by the `capture_request` middleware; `.await` is hidden inside the macros. |
| **Global facades** | `Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`, and `Broadcast` work as static calls from anywhere in a request cycle, backed by real engines over a deadlock-safe `AsyncBridge`. |
| **Typed DSL** | `validate! { title: string.max(200), email: email }` and the `Model!` `@` field DSL build real validation rules at compile time. |

Built on **axum 0.8**, **SeaORM / rusqlite**, **lettre**, **redis**, and **Tokio**.

---

## Quickstart

The snippets below are taken from `examples/blog-slice`, a CI-tested vertical slice. Run it:

```
cargo run -p blog-slice
```

### 1. One import

```rust
use rf::prelude::*;
```

### 2. Declare a model

```rust
// Generates the struct, INSERT/SELECT/UPDATE/DELETE macros, typed reads,
// eager-load builder, and the `posts` SQLite table backing.
Model!(Post: title, body);
```

### 3. Argument-less handlers

No `Request` parameter. `input`, `validate!`, `create!`, and `json` all come from
`use rf::prelude::*`.

```rust
async fn create_post() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(100), body: string }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }));
    }
    let title: String = input("title").unwrap_or_default();
    let body: String  = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(row) => json(row),
        Err(e)  => json(serde_json::json!({ "error": e.to_string() })),
    }
}

async fn list_posts() -> impl axum::response::IntoResponse {
    match Post::all().await {
        Ok(posts) => json(posts),
        Err(e)    => json(serde_json::json!({ "error": e.to_string() })),
    }
}

async fn show_post() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None     => return json(serde_json::json!({ "error": "invalid id" })),
    };
    match Post::find(id).await {
        Ok(Some(p)) => json(p),
        Ok(None)    => json(serde_json::json!({ "error": "not found" })),
        Err(e)      => json(serde_json::json!({ "error": e.to_string() })),
    }
}
```

### 4. Wire routes and serve

```rust
fn build_app() -> axum::Router {
    post("/posts",     create_post);
    get("/posts",      list_posts);
    get("/posts/{id}", show_post);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    DB::statement(
        "CREATE TABLE IF NOT EXISTS posts \
         (id INTEGER PRIMARY KEY, title TEXT, body TEXT)"
    ).expect("create table");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, build_app()).await.unwrap();
}
```

That is a complete request → validate → model → response slice backed by a real SQLite
database. No config file, no environment variable, no service running.

---

## Two-layer architecture

RustForge has two first-class layers that coexist in the same router.

**Layer 1 — Laravel-style DX (the default you write)**
Argument-less handlers, `Model!` / `validate!`, and the global facades. Fewest lines,
Laravel-familiar. Some correctness surfaces at runtime (documented — see caveats below),
not at compile time.

**Layer 2 — Explicit Rust-native core (the escape-hatch)**
Typed axum extractors (`ValidatedJson<T>`, `RequestExtractor`), an explicit `Request`
struct, and `AppResult`-returning handlers. Compile-time safe, no middleware dependency,
works outside a request scope (tests, CLI, background jobs).

Mix them per handler. The `capture_request` middleware buffers and re-inserts the body so
both layers can read the same request.

See [docs/API_PHILOSOPHY.md](docs/API_PHILOSOPHY.md) for the full trade-off table.

---

## Features

### Routing

`get`/`post`/`put`/`patch`/`delete` + `resource` register on the global router.
`build_router()` returns an `axum::Router`. Default JSON 404/405 on unmatched routes.

```rust
get("/posts",      list_posts);
post("/posts",     create_post);
get("/posts/{id}", show_post);
let router = rf::global_router().build_router();
```

### Request / Response

`input(key)`, `has(key)`, `file(name)`, and `all()` read the buffered request set by the
`capture_request` middleware. `json(data)`, `view(name, data)`, `back()`, and `download(path)`
build responses.

### Validation

```rust
// Inline DSL — returns ValidationErrors on failure
validate! { title: string.max(200), email: email, age: int.min(18) }?;

// Model @ DSL — generates a CreateUser DTO + ValidatedJson<CreateUser> extractor
Model!(User {
    validated,
    username: String @ min(3) max(20) alphanumeric,
    email:    String @ email message("A valid e-mail address is required"),
    password: String @ min(8),
});

async fn signup(ValidatedJson(user): ValidatedJson<CreateUser>) -> impl IntoResponse {
    // body never reaches here if validation fails — 422 with per-field errors
    (StatusCode::CREATED, json(serde_json::json!({ "username": user.username })))
}
```

48+ built-in rule types: `string`, `int`, `float`, `email`, `url`, `uuid`, `regex`,
`alpha`, `alphanumeric`, `min`, `max`, `between`, `required`, `nullable`, `confirmed`,
`in`, `not_in`, `image`, `mimes`, `max_file_size`, and more.

### ORM + Database

The `DB` facade and ORM macros run on **SQLite** (rusqlite, the default — in-memory,
zero-config) **or Postgres** (sqlx `PgPool` bridged via `AsyncBridge`). The backend is
selected by `DATABASE_URL`:

- Absent / non-URL path → in-memory SQLite
- SQLite file path → persistent SQLite
- `postgres://...` or `postgresql://...` → Postgres

```rust
// Struct + table mapping in one line
Model!(Post {
    title: String,
    body: String,
    user_id: i64,
});

// CRUD macros — real INSERT/SELECT/UPDATE/DELETE
let post   = create!(Post, title = "Hello", body = "World", user_id = 1)?;
let row    = find!(Post, 42)?;
let _      = update!(Post, 42, title = "Edited")?;
let _      = delete!(Post, 42)?;

// Fluent query builder
let recent = DB::table("posts")
    .where_("user_id", 1)
    .order_by("id", "DESC")
    .limit(10)
    .get()
    .await?;
```

**Eager-loading — no N+1:**

```rust
Model!(Article {
    title: String,
    author_id: i64,
    belongsTo author: Author,
});

// One fetch + one batched loader query
let articles = Article::with(&["author"]).get().await?;

// Nested dot-path (one query per level)
let posts = Post::with(&["comments.author"]).get().await?;
```

All four relation kinds — `belongsTo`, `hasOne`, `hasMany`, `belongsToMany` (including
pivot fields) — hydrate as populated struct fields.

**Postgres notes:** Primary key column must be named `id`. `NUMERIC`/`DECIMAL` columns
need a `::TEXT` cast for JSON decode. Transactions are ACID-atomic (dedicated pool
connection per transaction — `rollback` genuinely undoes inserts, verified against real
Postgres 16 in CI).

### Authentication

`require_auth` / `require_auth_with` are axum middleware that validate the
`Authorization: Bearer <jwt>` token via `JwtManager`, set the per-request `Auth` scope,
and inject `Extension<Claims>`. Rejection is JSON 401 before any body extractor runs.

```rust
let jwt = Arc::new(JwtManager::new(&secret)?);

let protected = Router::new()
    .route("/me", get(me_handler))
    .route("/posts", post(create_post_handler))
    .route_layer(middleware::from_fn(require_auth_with(jwt.clone())));
```

`rf-sanctum` provides transient API token auth (DB-free).

### Cache

```rust
// MemoryCache is the zero-config default; switch to Redis via RedisCache
if let Ok(Some(cached)) = Cache::get::<serde_json::Value>("posts:list") {
    return json(cached);
}
let _ = Cache::put("posts:list", value, 60u64);  // 60-second TTL
let _ = Cache::forget("posts:list");
```

No `block_on` panic inside an async runtime — the Redis driver uses `AsyncBridge`.

### Queue / Background Jobs

```rust
// MemoryQueue: zero-config in-process driver with DLQ, retry, panic-isolation
let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
Jobs::set_queue(queue.clone());

let worker = Worker::new(queue).register::<SendWelcomeJob>();
tokio::spawn(async move { worker.start().await.unwrap(); });

// Dispatch from a handler
SendWelcomeJob { email }.dispatch_now()?;
```

`rf-jobs` provides a Redis-backed `WorkerPool` for production workloads.

### Mail

```rust
// FileMailer (default, zero-config) or SMTP via env vars
rf_mail::MailFacade::send(WelcomeMail { to: email })?;

// Or fluent builder
Mail::to("user@example.com")
    .subject("Welcome!")
    .body("Thanks for joining.")
    .send()?;
```

Real SMTP transport via `lettre`; live round-trip tested in CI against MailHog.

### Storage

```rust
// MemoryStorage by default; S3 when S3_* env vars are set
Storage::put("uploads/avatar.png", bytes)?;
let data = Storage::get("uploads/avatar.png")?;
Storage::delete("uploads/avatar.png")?;
```

`LocalStorage` is path-traversal-safe. `S3Storage` uses `AsyncBridge` — no `block_on`
panic in async context. 413 on oversize upload.

### Security

- **CSRF** — validates the `_token` field in `x-www-form-urlencoded`, `multipart/form-data`,
  and the `X-CSRF-Token` header (constant-time comparison, multipart body buffered and
  re-inserted so uploads still reach the handler).
- **Security headers** — `default_security_headers_layer()` sets HSTS, CSP, X-Frame-Options,
  and Referrer-Policy in one call.
- **Rate limiting** — `RateLimitLayer` per client IP; JSON 429 response.
- **CORS** — `rf_web::cors` wires standard tower-http CORS.

### Observability

```rust
// Structured tracing with real trace/span IDs
init_logging(LogConfig::default())?;

// Prometheus registry (HTTP timings included)
app.merge(metrics_router())   // GET /metrics

// Health checks (fail-closed by default; Degraded returns 503)
app.merge(health_router(HealthChecker::new().add_check(MemoryCheck::default())))
```

---

## More patterns

### Terse error propagation

```rust
async fn update_post(Path(id): Path<i64>) -> AppResult<impl IntoResponse> {
    let _ = validate! { title: string.max(200), body: string }?;
    let title: String = input("title").unwrap_or_default();
    update!(Post, id, title = title)?;
    Ok(json(serde_json::json!({ "ok": true })))
}
```

`AppError` renders 400 / 401 / 403 / 404 / 422 / 500 as RFC 7807 JSON automatically.
`OrNotFound` adds `.or_404()` on `Option<T>`.

### Query scopes and pagination

```rust
let rows: Vec<Post>  = Post::all_typed().await?;
let page             = Post::paginate(15, 2).await?;  // per_page=15, page=2
let _: i64           = page.total;
let _: &Vec<Post>    = &page.data;
```

---

## Maturity

> **Canonical tier source:** [`docs/TIERS.md`](docs/TIERS.md) lists every workspace crate
> (127 total) with its tier and one-line justification. This matrix is a user-facing summary;
> `docs/TIERS.md` is authoritative.

**Stable tier (34 crates)** — real engine, CI-tested or probe-verified, used in a shipped
example. v1 compatibility promise. Includes: routing, request/response, validation, ORM,
auth, sanctum, cache, queue, jobs, mail, storage, events, broadcast, rate-limiting, CORS,
security headers, observability (tracing + Prometheus + health), i18n, scaffolding.

**Beta tier (76 crates)** — real implementations with documented minor gaps or not yet
exhaustively integration-tested. API may shift in minor versions. Includes: Inertia.js,
GraphQL, multi-tenancy, API resources, OAuth/Passport, scheduling, Blade templates, admin,
DDD application/domain/infra layers, and more.

**Experimental tier (8 crates)** — excluded from `default-members`; NOT part of the 1.0
supported surface; no SemVer guarantees; API may change or be removed without a bump.
Plain `cargo check` skips them; `cargo check --workspace` still compiles them (no bitrot).

| Experimental crate | What it is | Why excluded |
|--------------------|------------|--------------|
| `rf-nova` + `rf-nova-macros` | Admin panel | Type-erased dispatch unfinished; `#[derive(Resource)]` generates broken stubs |
| `rf-swagger` | OpenAPI / utoipa | Route-annotation-only, no auto-scan; not load-tested |
| `rf-telescope` | Debug dashboard | Stub implementation; not stress-tested |
| `rf-cms` | CMS features | Media processing / versioning unfinished |
| `rf-breeze` | Auth scaffolding | Depends on rf-blade; not integration-tested |
| `rf-vite` | Vite asset pipeline | Dev-tool only; not verified against axum 0.8 |
| `rf-livereload` | Live reload / HMR | WebSocket watcher not integration-tested |

See [`docs/STABLE_CORE.md`](docs/STABLE_CORE.md) for the full v1 API contract and every
stable entry point.

---

## Known limitations

Read these before you build.

- **`Result` and `Option` are visible.** RustForge hides `.await` inside macros and
  provides `AppError` / `AppResult` / `OrNotFound` to keep `?`-propagation short, but you
  still write `?`, match on `Result`, and handle `Option`. This is a Rust language boundary,
  not a bug.

- **Request globals are request-scoped.** `input()` / `file()` / `has()` read a task-local
  set by `capture_request`. Called outside a `capture_request`-wrapped handler they return
  `None` / `false` / empty — a runtime condition the compiler cannot catch. Wire
  `capture_request` and cover the gap with integration tests.

- **`SessionFacade` outside `session_scope`.** Without the `session_scope` middleware the
  session falls back to a single process-local store shared by all callers — concurrent
  clients can bleed into each other. Always add `session_scope` when serving concurrent
  HTTP traffic.

- **Eager-load caveats.** Combining a nested path (`"comments.author"`) with `with_where`
  applies the constraint to the first path segment only. `with_where` stores one equality
  constraint per relation (repeating it replaces the prior one). Documented follow-ups,
  not silent bugs.

- **DB facade: Postgres caveats.** Primary key column must be named `id`. `NUMERIC` /
  `DECIMAL` columns need a `::TEXT` cast for JSON decode.

- **Live-backend tests skip by default.** Redis / SMTP / S3 round-trips are real and wired
  but skip gracefully if the services are not running. Start them with:
  `docker compose -f docker-compose.test.yml up`. Offline `cargo test` is green because
  those paths skip, not because they are mocked.

- **`::` not `.`** Static calls on types use `::` in Rust: `Post::all()`, `Cache::get(...)`,
  `Mail::to(...)`. Laravel's dot syntax is not valid Rust.

---

## Reference application

`examples/reference-app/` is the flagship CI-tested app. It exercises every stable-core
surface in one binary:

| Surface | How it is exercised |
|---------|---------------------|
| Auth | `POST /auth/register`, `POST /auth/login`, `GET /me` |
| CRUD + ORM | `GET/POST/PUT/DELETE /posts` via `Model!` + `create!` / `find!` / `update!` / `delete!` |
| Migrations | `DB::statement()` at startup with `CREATE TABLE IF NOT EXISTS` |
| Validation | `validate!` DSL — 422 with per-field errors on create / update |
| Cache | `CacheFacade` on `GET /posts` list (60-second TTL, `Cache::forget` on write) |
| Storage | `StorageFacade::put` on `POST /upload` (MemoryStorage default, S3 via env) |
| Queue + Job | `MemoryQueue` + `Worker` + `SendWelcomeJob` dispatched on registration |
| Mail | `MailFacade::send()` on register (`FileMailer` default, SMTP via `SMTP_HOST`) |
| Health | `GET /health` via `rf-health`, `MemoryCheck` |
| Metrics | `GET /metrics` via `rf-metrics`, Prometheus text format |
| Observability | `rf-logging` structured tracing throughout |

```bash
# In-memory SQLite + FileMailer (zero-config)
cargo run -p reference-app

# Persistent SQLite
DATABASE_URL=./blog.db cargo run -p reference-app

# Postgres
DATABASE_URL=postgres://user:pass@localhost/db cargo run -p reference-app
```

---

## Examples

| Example | What it shows |
|---------|---------------|
| `examples/blog-slice` | Minimal vertical slice: route → validate → model → response |
| `examples/rest-crud-resource` | Full five-verb REST + eager `belongsTo` relation |
| `examples/validated-signup` | `Model!` `@` DSL + `ValidatedJson<T>` extractor end-to-end |
| `examples/taskflow` | Bidirectional relations, FK override, nested eager, `require_auth` |
| `examples/auth-paginated-search` | Auth facade + `where_like` search + `paginate` |
| `examples/jobs-offline` | `rf-queue` `MemoryQueue` + `Worker` (no Redis) |
| `examples/phase12-blog` | Larger blog: sessions, flash, blade views, mail |
| `examples/reference-app` | Full reference app: auth + CRUD + cache + storage + queue + mail + health/metrics |

---

## Documentation

| Doc | What it covers |
|-----|----------------|
| [docs/API_PHILOSOPHY.md](docs/API_PHILOSOPHY.md) | Two-layer design, trade-offs, when to use each layer |
| [docs/STABLE_CORE.md](docs/STABLE_CORE.md) | Full v1 API contract — every stable entry point, grep-verified |
| [docs/TIERS.md](docs/TIERS.md) | Every workspace crate (127) with its tier and justification |
| [docs/GETTING_STARTED.md](docs/GETTING_STARTED.md) | 5-minute quickstart and first REST resource |
| [docs/COOKBOOK.md](docs/COOKBOOK.md) | Copy-paste patterns for common tasks |
| [docs/LARAVEL_SYNTAX.md](docs/LARAVEL_SYNTAX.md) | Side-by-side Laravel PHP ↔ RustForge Rust |
| [docs/wiki/](docs/wiki/) | Extended guides and how-tos |

---

## Install / Build

```bash
git clone https://github.com/Chregu12/RustForge.git
cd RustForge

# Compile the stable workspace (0 warnings required)
cargo check --workspace

# Run offline tests (live-backend tests skip gracefully)
cargo test --workspace

# Format + lint
cargo fmt && cargo clippy

# Start live backends (Postgres, Redis, MailHog, MinIO) for full integration tests
docker compose -f docker-compose.test.yml up
cargo test --workspace   # now runs the live-backend paths too
```

Minimum Rust version: **1.96.0** (see `rust-toolchain.toml`).

---

## CI / Quality gates

| Gate | What it checks |
|------|----------------|
| `workspace-gate` | `RUSTFLAGS="-Dwarnings" cargo check --workspace` exits 0 with 0 warnings |
| Clippy | `cargo clippy --workspace` — no denies |
| Tier coverage | `scripts/check-tiers.sh` — every `crates/*` must carry a valid tier; fails the build if any is missing (127/127, 100%) |
| Probe sweep | 9 integration probes: rate-limiting, security headers, session isolation, tenancy, scaffold, validation DSL, CRUD macros, eager relations |
| Live backends | Full round-trips against real Postgres 16, Redis, MailHog (SMTP), and MinIO (S3) |

---

## License

MIT OR Apache-2.0
