# RustForge

**Laravel developer experience compiled to native Rust.**

RustForge is a full-stack web framework for Rust that hides the async machinery and
boilerplate developers usually have to write by hand, while keeping every promise the
compiler enforces. Its four pillars are:

1. **Write less** — `Model!` declares a struct, its DB table, CRUD macros, typed reads,
   relation loaders, query scopes, and validated DTOs in one line.
2. **Hide async** — argument-less handlers read the current request through ambient
   globals set by the `capture_request` middleware; `.await` is hidden inside the
   macros.
3. **Global facades** — `Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`,
   `Broadcast` work as static calls from anywhere in a request cycle, backed by real
   engines over a deadlock-safe `AsyncBridge` (no `block_on` inside an async runtime).
4. **Typed DSL** — `validate! { title: string.max(200), email: email }` and the
   `Model!` `@` field DSL (`@ email message("...")`, `@ min(3) max(20) alphanumeric`)
   build real `rf_validation` rules at compile time.

Built on **axum 0.8**, **SeaORM/rusqlite**, **lettre**, **redis**, and **Tokio**.

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)]()
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)]()

---

## Quickstart

The snippets below are taken verbatim from `examples/blog-slice` (the canonical CI-tested
entry point). Run it with:

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

No `Request` parameter. `input`, `validate!`, `create!`, and `json` come from
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
    post("/posts",      create_post);
    get("/posts",       list_posts);
    get("/posts/{id}",  show_post);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    DB::statement("CREATE TABLE IF NOT EXISTS posts \
                   (id INTEGER PRIMARY KEY, title TEXT, body TEXT)")
        .expect("create table");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, build_app()).await.unwrap();
}
```

That is a complete request -> validate -> model -> response slice backed by a real
SQLite database.

---

## More patterns

### Eager-loading relations (no N+1)

```rust
Model!(Author { name: String });
Model!(Article {
    title: String,
    body: String,
    author_id: i64,
    belongsTo author: Author,
});

// One fetch + one batched loader query. Returns typed Vec<Article>
// where every `article.author` field is populated.
let articles = Article::with(&["author"]).get().await?;

// Nested dot-path (one batched query per level):
let posts = Post::with(&["comments.author"]).get().await?;

// Constrained: only approved comments:
let posts = Post::with(&["comments"])
    .with_where("comments", "approved", 1)
    .get().await?;
```

All four relation kinds (`belongsTo`, `hasOne`, `hasMany`, `belongsToMany` incl.
pivot fields) hydrate as populated struct fields.

### Typed reads and pagination

```rust
let rows: Vec<Post>  = Post::all_typed().await?;        // concrete typed rows
let page             = Post::paginate(15, 2).await?;    // per_page=15, page=2
let _: &Vec<Post>    = &page.data;
let _: i64           = page.total;
```

### Validated DTOs (the `@` DSL + `ValidatedJson`)

```rust
Model!(User {
    validated,
    username: String @ min(3) max(20) alphanumeric,
    email:    String @ email message("A valid email address is required"),
    password: String @ min(8),
    zipcode:  String @ regex("^\\d{5}$"),
});

// The extractor deserializes AND validates in one step.
// An invalid body never reaches this handler — the extractor
// short-circuits with 422 Unprocessable Entity.
async fn signup(ValidatedJson(user): ValidatedJson<CreateUser>) -> impl IntoResponse {
    (StatusCode::CREATED, Json(serde_json::json!({ "username": user.username })))
}
```

The `@` DSL exposes: `email`, `url`, `uuid`, `ip`, `regex`, `alpha`, `alphanumeric`,
`starts_with`, `ends_with`, string `min`/`max`, numeric `range`, and per-field
`message`.

### Auth + protected routes

```rust
// Bearer-auth middleware: rejects unauthenticated requests with JSON 401
// BEFORE body extractors run (so auth fires before any 422).
let protected = axum::Router::new()
    .route("/admin", get(admin_handler))
    .route_layer(axum::middleware::from_fn(require_auth));

// Verify credentials and set per-request user state:
Auth::attempt(serde_json::json!({ "email": email, "password": password }))?;

// Read the current user (returns None outside an auth scope):
let user = Auth::user::<MyUser>();
```

For stateless token auth, `rf-sanctum` provides transient tokens (no DB required).

### Terse error propagation

```rust
use rf::prelude::*;
use rf::core::{AppError, AppResult, OrNotFound};

async fn update_post() -> AppResult<impl IntoResponse> {
    let id: i64 = input("id").ok_or(AppError::NotFound("post".into()))?;
    let _ = validate! { title: string.max(200), body: string }?;
    update!(Post, id, title = input::<String>("title").unwrap_or_default())?;
    Ok(json(serde_json::json!({ "ok": true })))
}
```

`AppError` renders 400/401/403/404/422/500 with a JSON envelope automatically.

---

## Feature-maturity matrix

> **Canonical tier source:** [`docs/TIERS.md`](docs/TIERS.md) lists **every**
> workspace crate (113 total) with its tier and one-line justification. This
> matrix is a user-facing summary of the highlighted surfaces; if the two files
> ever conflict, fix `docs/TIERS.md` first, then propagate here.

Graded against the verified state after 8 production-loop rounds (documented in
`VISION_GAP.md`). **Stable** = real engine, CI-tested or probe-verified, used in a
shipped example (maps to `stable` tier in `docs/TIERS.md`). **Usable** = real core,
documented minor gaps (maps to `beta` tier). **Experimental** = exists in the repo
but NOT part of the 1.0 supported surface — SemVer guarantees do NOT apply; API may
change or be removed without a bump (maps to `experimental` tier). **Deferred** =
intentionally not built, with a reason (not a crate; a language/design ceiling).

> **SemVer scope:** Only the Stable and Usable surfaces listed below are covered by
> SemVer guarantees starting at 1.0. Experimental crates are excluded from the
> workspace `default-members` so a plain `cargo check` skips them; they are kept in
> `members` so `cargo check --workspace` still compiles them (no bitrot). Do not
> depend on them in production code.

### Supported 1.0 surface

| Surface | Grade | Note |
|---|---|---|
| **Routing** — `get`/`post`/`put`/`patch`/`delete` + `build_router` | Stable | Real axum 0.8. Default JSON 404/405. |
| **Request globals** — `input`/`file`/`has` + `capture_request` | Stable | Task-local; body buffered and re-inserted so globals + body extractors coexist on one router. |
| **ORM / CRUD macros** — `Model!` + `create!`/`find!`/`update!`/`delete!` | Stable | Real SQLite/Postgres/MySQL via SeaORM. `create!` no-fields inserts DEFAULT VALUES. |
| **Relations (eager)** — all four kinds as populated fields | Stable | `belongsTo`, `hasOne`, `hasMany`, `belongsToMany` (incl. pivot). Bidirectional pairs compile (boxed future, no E0391). |
| **`with().get()` + nested + constrained** | Stable | Typed fetch-time builder; one-level nested dot-paths (`comments.author`); `with_where` constrains child rows. Combining nested + constrained applies constraint to the first path segment only (documented). |
| **Query scopes** — `scope name: chain` in `Model!` | Stable | Generates `Model::name() -> QueryBuilder`; chainable with further filter/order/limit. |
| **Typed reads** — `all_typed()` / `paginate(per_page, page)` | Stable | `Vec<Self>` and a generated `<Name>Page { data, total, per_page, current_page, last_page }`. |
| **Mass-assignment guard** — `guarded` marker | Stable | Opt-in; `create!`/`Model::create` reject non-fillable fields, naming them with no insert. Non-guarded models unchanged. |
| **Validation** — `validate!` DSL | Stable | Typed (`string.max`, `int.min`, `email`, `regex`, ...); 422 with structured per-field errors. |
| **Validated DTOs** — `@` DSL + `ValidatedJson` | Stable | `@ email`/`url`/`uuid`/`ip`/`regex`/`alpha`/`alphanumeric`/`starts_with`/`ends_with`/`min`/`max`/`range` + `message`. `ValidatedJson<T>` is an axum 0.8 extractor. |
| **Auth** — `Auth` facade + `require_auth` middleware | Stable | Per-request state (no cross-request bleed); bearer guard fires before body extractors (401 before 422). |
| **API token auth** — `rf-sanctum` | Stable | Transient token auth (DB-free); real JSON error envelope on failure. |
| **Sessions** — per-client + flash | Stable | `session_scope` middleware + `SessionFacade`; cookie session-id; flash is a true one-request flush; no cross-client bleed. |
| **Mail** — `Mail` facade (lettre / FileMailer) | Stable | Real SMTP/FileMailer; `Mail::to(addr).subject(s).body(b).send()`; queued mail via `MailQueue` actually enqueues and drains to the configured transport. |
| **Notifications** — multi-channel | Stable | `Notifier` attempts all channels, returns aggregate report; no abort on first failure. |
| **Background jobs** — `rf-queue` in-process | Stable | `MemoryQueue` + `Worker` with priority FIFO, retry, dead-letter (`Queue::failed()`), panic-isolation (`catch_unwind`). No Redis needed. Run: `examples/jobs-offline`. |
| **Background jobs** — `rf-jobs` Redis | Stable | Redis-backed `WorkerPool`; requires live Redis. |
| **Events** — `Event` facade | Stable | Type-keyed sync bus; listener panics isolated (`catch_unwind`); no deadlock on re-entrant dispatch. |
| **WebSocket / Broadcast** — `Broadcast` facade | Stable | Room isolation; `Subscribed` ack; `Lagged` skip-and-continue (no silent socket drop); process-global facade callable from background jobs. |
| **Cache** — `Cache` facade | Stable | Memory + optional Redis via `AsyncBridge` (no `block_on` panic in an async runtime). |
| **Storage** — `Storage` facade | Stable | Local + S3 via `AsyncBridge`; path-traversal-safe; `file("field")` request helper; 413 on oversize upload. |
| **Rate limiting** — `RateLimitLayer` | Stable | Per-client IP default key; JSON 429; non-destructive `info()` peek. |
| **CORS** | Stable | `rf_web::cors` middleware; standard tower-http CORS layer wired. |
| **Security headers** — `SecurityHeadersLayer` | Stable | `default_security_headers_layer()` for zero-config; configurable HSTS/CSP/X-Frame-Options/Referrer-Policy. |
| **Observability** — tracing + metrics + health | Stable | Real trace/span IDs in log correlation; unified Prometheus registry includes HTTP timings; `HealthChecker` fail-closed by default; `Degraded` can return 503. |
| **Scaffolding** — `foundry-cli` / `forge-cli` | Stable | `make:model/controller/request/migration` generates compiling, running code (plural table name, real `Model!` DSL, axum 0.8 routes, validated request API). `forge deploy generate` wires rf-deploy from the CLI. Generated code is warning-clean. |
| **i18n** — `rf-i18n` | Stable | `AcceptLanguage` axum extractor (header + `?locale=`); CLDR plural rules (Slavic/Arabic included); Handlebars rendering (no raw template leakage); `Arc`-shareable. |
| **SSR / Inertia** — `rf-inertia` | Usable | Full protocol: X-Inertia-Location on 409, browser→HTML/XHR→JSON finalization, SharedProps wired. Not exhaustively load-tested. |
| **Deploy / config** — `rf-deploy`, `rf-config`, `rf-env` | Usable | DockerCompose serialization correct; `AppConfig::from_env` errors on unparseable values; dotenvy-backed; `forge deploy generate` CLI. |
| **GraphQL** — `rf-graphql` | Usable | Per-request auth context injected so `AuthGuard`/`RoleGuard` are reachable. |
| **Multi-tenancy** — `rf-tenancy` | Usable | Real axum 0.8 Layer + `Tenant::current()` + isolation helpers; `spawn_with_tenant()` for spawned tasks; missing-tenant header returns 400 (not 500). |
| **Resource transform** — `rf-api-resources` | Usable | `WrappedResource`/`WrappedCollection`; `axum::Json(wrapped)` and `to_json()` are identical (manual Serialize, no silent wrapper drop). |
| **`Result`/`Option` hiding** | Deferred | Rust language ceiling: `.await` is hidden, but `?`/`Result`/`Option` stay visible. Hiding them needs one uniform error type; `AppError` + `AppResult` make the idiomatic `?` path short (~8 vs 34 lines for equivalent behavior). |
| **Lazy relation property access** (`post.user`) | Deferred | Impossible in Rust for lazy loading: field reads cannot be async. Bare `post.user` only works for eager-hydrated `Option<User>` fields (which is what `with().get()` populates). |
| **`load_session` / `readiness_check` / `init_telemetry`** | Deferred | Each requires external infra or a design decision: full Session entity, service-probe design, real OTel SDK respectively. Honestly stubbed, not faked. |

### Experimental surfaces (NOT covered by SemVer)

These crates exist in the repo and compile, but are **not part of the 1.0 supported
surface**. They are excluded from `default-members` so plain `cargo check` skips them.
Do not use them in production without accepting that their APIs may change at any time.

| Crate | What it is | Why experimental |
|---|---|---|
| `rf-nova` + `rf-nova-macros` | Laravel Nova-inspired admin panel | Multi-resource type-erased dispatch unfinished; `#[derive(Resource)]` generates broken stubs |
| `rf-swagger` | Thin utoipa/OpenAPI integration | Route-annotation-only (no auto-scan); not load-tested against real route trees |
| `rf-telescope` | Debugging dashboard (request/query monitor) | Stub implementation; not stress-tested against the framework |
| `rf-cms` | Content Management System features | Stub; media processing / versioning unfinished |
| `rf-breeze` | Auth scaffolding generator (Laravel Breeze equiv.) | Depends on rf-blade template engine; not integration-tested |
| `rf-vite` | Vite asset pipeline integration | Dev-tool only; not verified against current axum 0.8 handler model |
| `rf-livereload` | Live reload / HMR for development | Dev-tool only; WebSocket watcher not integration-tested |

---

## Known limitations

Read these before you build.

- **`Result` and `Option` are visible.** RustForge hides `.await` inside macros and
  provides `AppError`/`AppResult`/`OrNotFound` to keep `?`-propagation short, but you
  still write `?`, match on `Result`, and handle `Option`. This is a Rust language
  boundary, not a bug.

- **Ambient globals are request-scoped.** `input()`/`file()`/`has()` read a task-local
  set by `capture_request`. Calling them outside a request handler returns empty/`None`
  — a runtime condition, not a compile error.

- **Eager-load caveats.** Combining a nested path (`"comments.author"`) with
  `with_where(...)` applies the constraint to the **first** path segment (`comments`)
  only. `with_where` stores one equality constraint per relation (repeating it replaces
  the prior one). These are documented follow-ups, not silent bugs.

- **`::` not `.`** Rust requires `Post::all()`, `Mail::to(...)`, `Cache::put(...)` —
  static calls on a type use `::`. Dot-on-type syntax from Laravel (`Post.all()`) is not
  valid Rust.

- **Live-backend tests skip by default.** Redis/SMTP/S3 round-trips are real and wired,
  but their tests skip gracefully unless you start the services:
  `docker compose -f docker-compose.test.yml up`. Offline `cargo test` is green because
  those paths skip, not because they are mocked.

- **Three migration backends.** The three code generators (`foundry-cli`,
  `rf-scaffold`, `rf-cli-gen`) each use a different migration format
  (sea-orm-migration / sqlx-raw-SQL / plain-SQL). Unification is a planned follow-up.

---

## Documentation

- **[Getting Started](docs/GETTING_STARTED.md)** — 5-minute quickstart, full REST
  resource example, validated-DTO pattern, and a detailed Done/Partial/Deferred
  maturity matrix with per-surface notes.

---

## Examples

| Example | What it shows |
|---|---|
| `examples/blog-slice` | Minimal vertical slice: route → validate → model → response |
| `examples/rest-crud-resource` | Full five-verb REST (GET list/show, POST, PUT, DELETE) + eager relation |
| `examples/validated-signup` | `Model!` `@` DSL + `ValidatedJson<T>` extractor end-to-end |
| `examples/taskflow` | Bidirectional relations, FK override, body+globals coexist, nested eager, require_auth |
| `examples/auth-paginated-search` | Auth facade + `where_like` search + `paginate` |
| `examples/jobs-offline` | `rf-queue` `MemoryQueue` + `Worker` (no Redis) |
| `examples/phase12-blog` | Larger blog: sessions, flash, blade views, mail |

---

## Contributing

```bash
git clone https://github.com/Chregu12/RustForge.git
cd RustForge
cargo check --workspace   # must exit 0 with 0 warnings
cargo test --workspace    # green offline (live-backend tests skip)
cargo fmt && cargo clippy
```

Open a PR against `main`. A stub-hunt (`grep` for functions returning hardcoded/empty
data) runs on every PR.

---

## License

MIT OR Apache-2.0
