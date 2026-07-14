# Features

This page lists RustForge's capabilities grouped by area, each tagged with its
maturity tier per [docs/TIERS.md](../../TIERS.md):

| Tag | Meaning |
|-----|---------|
| **[stable]** | v1 API contract. No breaking changes without a major-version bump. CI-tested or probe-verified. |
| **[beta]** | Real implementation; documented gaps or not exhaustively integration-tested. API may shift in minor versions. |
| **[experimental]** | Excluded from `default-members` and the 1.0 supported surface. No SemVer guarantee. |

The **34 stable crates** form the v1 surface. See [docs/STABLE_CORE.md](../../STABLE_CORE.md)
for the complete entry-point inventory and [docs/TIERS.md](../../TIERS.md) for the full
crate-by-crate roster.

---

## Table of contents

- [Single import and prelude](#single-import-and-prelude)
- [HTTP and routing](#http-and-routing)
- [Request globals and response helpers](#request-globals-and-response-helpers)
- [Validation](#validation)
- [ORM and database](#orm-and-database)
- [Authentication](#authentication)
- [Cache](#cache)
- [Queue and background jobs](#queue-and-background-jobs)
- [Mail](#mail)
- [File storage](#file-storage)
- [Events](#events)
- [WebSocket broadcast](#websocket-broadcast)
- [Rate limiting and security middleware](#rate-limiting-and-security-middleware)
- [Internationalization](#internationalization)
- [Health and observability](#health-and-observability)
- [Notifications](#notifications)
- [Error handling](#error-handling)
- [Configuration](#configuration)
- [Helpers and collections](#helpers-and-collections)
- [CLI (forge)](#cli-forge)
- [Templates and views](#templates-and-views)
- [GraphQL](#graphql)
- [OAuth and social login](#oauth-and-social-login)
- [AI and vector search](#ai-and-vector-search)
- [Testing utilities](#testing-utilities)
- [Task scheduling](#task-scheduling)
- [Browser testing](#browser-testing)
- [Service container](#service-container)
- [Experimental crates](#experimental-crates)

---

## Single import and prelude

**[stable]** — crates: `rf`, `rustforge`, `rf-facades`

```rust
use rf::prelude::*;
```

One `use` statement brings all stable facades, macros, and helpers into scope.
No additional direct crate dependencies beyond `rf`, `serde`, and `tokio` are needed
for typical handler files.

Available module groupings:
```rust
use rf::facades::*;     // Auth, Cache, Mail, Storage, Event, DB, Route, ...
use rf::web::*;         // HTTP types, middleware, CSRF
use rf::data::*;        // ORM, Cache, Validation
use rf::helpers::*;     // Hash, redirect, csrf_token
```

---

## HTTP and routing

**[stable]** — crates: `rf-routing` (routing facade), `rf-web` (axum stack)

Register routes on the global router using free functions:

```rust
use rf::prelude::*;

get("/articles",        index);
post("/articles",       store);
put("/articles/{id}",   update);
delete("/articles/{id}", destroy);

// RESTful resource shorthand (generates all five CRUD routes).
resource("/posts", PostController);

// Build the axum Router and add the body-buffering middleware.
let app = rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

`rf-web` provides:
- CORS middleware (`cors_layer(CorsConfig)`)
- Security-header middleware (`security_headers_layer(SecurityHeadersConfig)`)
- Per-request session isolation (`session_scope` middleware)
- CSRF validation — covers `Authorization` header, `x-www-form-urlencoded`, and `multipart/form-data` bodies
- JSON 404 / 405 handlers

Named routes, signed URLs, and route groups are part of the `rf-routing` stable surface.

---

## Request globals and response helpers

**[stable]** — crates: `rf-request`, `rf-response`

Request globals (require `capture_request` middleware on the router):

```rust
let title: Option<String> = input("title");   // coerces JSON / form / query strings
let exists: bool          = has("avatar");
let upload: Option<UploadedFile> = file("photo");
let all_fields            = all();             // HashMap<String, Value>
```

Response helpers:

```rust
json(value)               // application/json — any Serialize type
view("posts.index", data) // text/html — renders resources/views/posts/index.blade.html
back()                    // redirect to Referer or /
download(path)            // Content-Disposition: attachment
Response::no_content()    // 204
```

**Honest caveats:** Request globals return `None` / `false` / empty map when called
outside a `capture_request`-wrapped handler. `SessionFacade` requires `session_scope`;
without it, sessions bleed between concurrent clients. See [docs/API_PHILOSOPHY.md](../../API_PHILOSOPHY.md).

---

## Validation

**[stable]** — crates: `rf-validation`, `rf-validation-derive`

```rust
// Inline DSL — short-circuits with 422 on failure.
if validate! {
    title: string.max(200),
    body:  string,
    email: email,
    age:   integer.min(18),
    photo: image.max(mb(5)).optional,
}.is_err() {
    return json(serde_json::json!({ "error": "validation failed" }))
        .status(StatusCode::UNPROCESSABLE_ENTITY);
}

// Per-field @ DSL on a model struct with ValidatedJson extractor.
Model!(User {
    validated,
    username: String @ min(3) max(20) alphanumeric,
    email:    String @ email message("Please enter a valid email address"),
    password: String @ min(8),
});

async fn signup(ValidatedJson(body): ValidatedJson<CreateUser>) -> impl IntoResponse {
    // Validation happened before this body ran; invalid bodies never reach here.
    json(serde_json::json!({ "username": body.username }))
}
```

48+ built-in rule types including: `string`, `integer`, `float`, `boolean`, `email`,
`url`, `uuid`, `date`, `min`, `max`, `between`, `required`, `nullable`, `confirmed`,
`in`, `not_in`, `regex`, `alpha`, `alpha_num`, `image`, `mimes`, `max_file_size`,
`kb(n)` / `mb(n)` file-size helpers.

Sources: `examples/validated-signup/`, `examples/taskflow/`

---

## ORM and database

**[stable]** — crates: `rf-orm`, `rf-eloquent`

Database backend is selected by `DATABASE_URL`:
- Absent or file path → **SQLite** (rusqlite, the default including in-memory)
- `postgres://...` or `postgresql://...` → **Postgres** (sqlx PgPool)

Both paths are CI-tested. The primary key column must be named `id` on the Postgres path.
`NUMERIC`/`DECIMAL` columns on Postgres must be cast to `TEXT` or `FLOAT8` in raw queries.

```rust
// Declare a model.
Model!(Post {
    title: String,
    body:  String,
    belongsTo author: Author,
});

// Macro-based CRUD.
let post    = create!(Post, title = "Hello", body = "World").unwrap();
let post    = find!(Post, 1).unwrap();       // returns Option
let updated = update!(Post, 1, title = "Updated title").unwrap();
let deleted = delete!(Post, 1).unwrap();     // returns rows affected

// Fluent query builder.
let rows = DB::table("posts")
    .r#where("published", true)
    .order_by("created_at", "desc")
    .limit(10)
    .get()
    .await?;

// Eager loading.
let posts = Post::with(&["author"]).get().await?;

// Pagination.
let page = Post::paginate(15, 1).await?;

// Transactions (ACID-atomic on both SQLite and Postgres).
DB::begin().await?;
create!(Post, title = "First").unwrap();
create!(Post, title = "Second").unwrap();
DB::commit().await?;
```

Related stable features: `SoftDelete`, scopes, `migration!` DSL.

Sources: `examples/rest-crud-resource/`, `examples/taskflow/`, `examples/phase12-blog/`

---

## Authentication

**[stable]** — crates: `rf-auth`, `rf-sanctum`

```rust
// Protect a route — require_auth validates the `Authorization: Bearer <jwt>` header.
// It rejects with JSON 401 before any body extractor runs.
Router::new()
    .route("/api/me", get(me_handler))
    .route_layer(axum::middleware::from_fn(require_auth))

// Inside a protected handler.
async fn me_handler() -> impl IntoResponse {
    if let Some(user_id) = Auth::user_id() {
        json(serde_json::json!({ "id": user_id }))
    } else {
        json(serde_json::json!({ "error": "unauthenticated" }))
    }
}

// Issue a JWT.
let manager = JwtManager::from_env()?;
let token   = manager.create_token(user_id, claims)?;

// Sanctum personal-access tokens (DB-free, SHA-256 hashed).
let repo  = TokenRepository::new(&conn);
let tok   = repo.create("User", 42, "mobile-app", vec!["posts:read".into()], None).await?;
let found = repo.find_by_token(&PersonalAccessToken::hash_token(&tok.access_token)).await?;
assert!(found.unwrap().can("posts:read"));
```

`Auth::user()`, `Auth::check()`, `Auth::login(user)`, `Auth::logout()` — available inside
a `require_auth` or `with_auth_scope` scope.

Sources: `examples/auth-demo/`, `examples/auth-paginated-search/`, sandbox probe `require_auth_guard`

---

## Cache

**[stable]** — crate: `rf-cache`

```rust
use rf::prelude::*;

// Memory driver (zero-config default).
Cache::put("key", "value", Duration::from_secs(3600));
let val: Option<String> = Cache::get("key");
Cache::forget("key");

// Cache-or-compute.
let posts = Cache::remember("posts:all", Duration::from_secs(300), || async {
    Post::all().await
}).await?;

// Redis driver — requires `redis` feature and a live Redis instance.
// Configure at startup; the facade API is identical.
```

Drivers: `MemoryCache` (default, zero-config), `RedisCache` (optional; requires `redis` feature).

Sources: `examples/facades-demo/`, sandbox probe `cache`

---

## Queue and background jobs

**[stable]** — crates: `rf-queue` (in-process), `rf-jobs` (Redis-backed workers)

```rust
use rf_queue::{Job, Jobs, MemoryQueue, Queue, QueueError, Worker};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SendWelcome { email: String }

#[async_trait::async_trait]
impl Job for SendWelcome {
    async fn handle(&self) -> Result<(), QueueError> {
        // send email ...
        Ok(())
    }
    fn job_type(&self) -> &'static str { "send_welcome" }
}

// Zero-config in-process queue (no Redis).
let queue = std::sync::Arc::new(MemoryQueue::new());
Jobs::set_queue(std::sync::Arc::clone(&queue));
Jobs::dispatch(SendWelcome { email: "user@example.com".into() })?;

// Drain the queue in-process.
let worker = Worker::new(queue).register::<SendWelcome>();
while worker.work_once().await? {}
```

`rf-jobs` adds a Redis-backed `WorkerPool` for production deployments
(requires a live Redis instance; see `examples/jobs-demo/`).

`MemoryQueue` features: priority FIFO, configurable retry, dead-letter queue (`Worker::failed()`),
panic-isolated worker tasks.

Sources: `examples/jobs-offline/`, `examples/jobs-demo/`

---

## Mail

**[stable]** — crate: `rf-mail`

```rust
use rf::prelude::*;
use rf_mail::{Mailable, MailBuilder};

struct WelcomeEmail;
impl Mailable for WelcomeEmail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .subject("Welcome!")
            .text("Welcome to RustForge!")
    }
}

// Facade (synchronous, writes a .eml via FileMailer by default).
Mail::to("alice@example.com").send(WelcomeEmail)?;
```

Transports: `SmtpMailer` (real lettre SMTP), `FileMailer` (writes RFC 5322 `.eml` files — default when
no SMTP is configured). Template-based mail via `TemplateEngine` (Handlebars).

For testing: `fake()` guard + `get_fake()` recorder intercepts sends without touching the network.

Sources: `examples/mail-demo/`, sandbox probe `mail`

---

## File storage

**[stable]** — crate: `rf-storage`

```rust
use rf::prelude::*;

// Facade (synchronous, MemoryStorage default in tests / no-config).
Storage::put("uploads/photo.jpg", bytes)?;
let data = Storage::get("uploads/photo.jpg")?;
Storage::delete("uploads/photo.jpg")?;

// Explicit async LocalStorage (path-traversal-safe).
let store = LocalStorage::new(&root, "http://localhost:3000").await?;
store.put("uploads/report.pdf", bytes).await?;
```

Drivers: `MemoryStorage` (default, zero-config), `LocalStorage` (real filesystem, path-traversal-safe,
413 on oversize upload), `S3Storage` (AWS S3 / S3-compatible; requires `s3` feature).

Sources: `examples/reference-app/`, sandbox probe `storage`

---

## Events

**[stable]** — crate: `rf-events`

```rust
use rf::prelude::*;

// Register a listener.
Event::listen("user.registered", |data: serde_json::Value| {
    println!("User registered: {}", data["email"]);
});

// Dispatch an event.
Event::dispatch("user.registered", serde_json::json!({ "email": "alice@example.com" }));
```

`EventDispatcher` is type-keyed; listener panics are isolated (one failing listener does not
abort subsequent listeners). `EventManager` is the sync global bus backing the `Event` facade.

---

## WebSocket broadcast

**[stable]** — crate: `rf-broadcast`

```rust
use rf_broadcast::{websocket_router, Broadcaster, Channel, MemoryBroadcaster, SimpleEvent};

let broadcaster = std::sync::Arc::new(MemoryBroadcaster::new());
let app = axum::Router::new().merge(websocket_router(std::sync::Arc::clone(&broadcaster)));

// Broadcast from any code path.
let room  = Channel::public("room-1");
let event = SimpleEvent::new(
    "message.posted",
    serde_json::json!({ "text": "hello" }),
    vec![room.clone()],
);
broadcaster.broadcast(&room, &event).await?;
```

Client subscribes with `{"type":"subscribe","channel":"room-1"}`.
Server acks with `{"type":"subscribed"}` and delivers `{"type":"event","channel":"room-1",...}` frames.
Lagged clients skip missed events and continue (no disconnect).

Source: `examples/realtime-chat/`

---

## Rate limiting and security middleware

**[stable]** — crates: `rf-ratelimit`, `rf-web`

```rust
use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimiter};

let limiter = MemoryRateLimiter::new(RateLimitConfig::per_minute(60));
let result  = limiter.check("user:42").await?;
if !result.allowed {
    // return 429
}
// Non-destructive peek (does not consume a slot).
let info = limiter.info("user:42").await?;
```

Security-header middleware:
```rust
use rf_web::{security_headers_layer, SecurityHeadersConfig};

let app = Router::new()
    .layer(security_headers_layer(SecurityHeadersConfig::default()));
// Adds: X-Content-Type-Options: nosniff, X-Frame-Options: DENY, Referrer-Policy: no-referrer.
// Optional: HSTS, Content-Security-Policy.
```

Sources: sandbox probes `rate_limiting`, `security_headers`

---

## Internationalization

**[stable]** — crate: `rf-i18n`

```rust
use rf_i18n::{AcceptLanguage, I18n, TranslationCatalog};

let i18n = std::sync::Arc::new(
    I18n::new("en").fallback("en")
        .add_catalog(TranslationCatalog::new("en").add("greeting", serde_json::json!("Welcome!")))
        .add_catalog(TranslationCatalog::new("de").add("greeting", serde_json::json!("Willkommen!"))),
);

async fn greet(AcceptLanguage(locale): AcceptLanguage, Extension(i18n): Extension<Arc<I18n>>) -> ... {
    let msg = i18n.for_locale(&locale).t("greeting", None)?;
    // ...
}
```

Locale resolution order: `?locale=` query param > `Accept-Language` header primary tag > fallback.
CLDR plural rules (Slavic, Arabic, etc.) via `t_plural(key, count)`.

Source: `examples/i18n-localized-api/`

---

## Health and observability

**[stable]** — crates: `rf-health`, `rf-logging`, `rf-metrics`

```rust
use rf_health::{HealthChecker, HealthCheck, CheckResult};
use rf_health::checks::{DiskCheck, MemoryCheck};

let checker = HealthChecker::new()
    .add_check(MemoryCheck::new(0.8, 0.95))  // warn 80%, crit 95%
    .add_check(DiskCheck::new("/", 0.8, 0.95));

let result = checker.check_all().await;
// result.status: Healthy | Degraded | Unhealthy
// result.http_status(): 200 for Healthy/Degraded, 503 for Unhealthy
```

Prometheus metrics:
```rust
use rf::services::metrics::{HTTP_REQUEST_COUNT, HTTP_REQUEST_DURATION, Counter};

HTTP_REQUEST_COUNT.with_label_values(&["GET", "/posts", "200"]).inc();
let orders = Counter::new("orders_total", "Total orders")?;
orders.inc();
```

`rf-logging` provides structured `tracing` spans with per-request trace and span IDs.

Source: sandbox probes `health_checks`, `metrics_prometheus`

---

## Notifications

**[stable]** — crate: `rf-notifications`

Multi-channel delivery (Mail + Database + custom channels) with aggregated failure reporting
(no abort on first failure). Includes `mark_as_read` / `get_unread_notifications`.

Source: sandbox probe `notifications`

---

## Error handling

**[stable]** — crate: `rf-core`

```rust
// Use AppResult<T> as handler return type for ?-ergonomics.
async fn show_post(axum::extract::Path(id): axum::extract::Path<i64>) -> AppResult<impl IntoResponse> {
    let post = find!(Post, id).or_404()?;   // Option -> 404 on None
    Ok(json(post))
}
```

`AppError` implements `IntoResponse` and renders RFC 7807 JSON.
`OrNotFound` trait adds `.or_404()` to `Option<T>` and `Result<Option<T>, E>`.

---

## Configuration

**[stable]** — crate: `rf-config`

```rust
use rf::prelude::*;

let name = Config::get("app.name");
Config::set("app.debug", "true");

// Typed config from config/*.toml + env vars.
let cfg = AppConfig::from_env()?;
```

dotenvy-backed `.env` loading. `Config` and `AppConfig::from_env` are part of the prelude.

---

## Helpers and collections

**[stable]** — crates: `rf-global-helpers`, `rf-collections`

```rust
use rf::prelude::*;

// Password hashing (bcrypt/argon2).
let hash    = Hash::make("secret");
let matches = Hash::check("secret", &hash);

// CSRF.
let token = csrf_token();
let html  = csrf_field();   // <input type="hidden" name="_token" value="...">

// Redirects.
let r = redirect("/dashboard");
let r = back();

// Laravel-style collections.
let coll = collect(vec![1, 2, 3, 4, 5]);
let evens = coll.filter(|n| n % 2 == 0);
let page  = coll.paginate(2, 1);  // per_page=2, page=1
```

---

## CLI (forge)

**[stable]** — crate: `forge-cli`

```bash
# Scaffolding.
forge new my-app
forge make model Post
forge make model Comment --migration
forge make controller PostController
forge make migration create_posts_table

# Database.
forge migrate
forge migrate --rollback

# Dev server.
forge serve
forge serve --port 3001

# Deploy artifacts (Dockerfile + docker-compose.yml + optional K8s manifests).
forge deploy generate my-app --port 3001 --with-postgres 15
forge deploy generate my-app --port 3001 --kubernetes --image my-app:v1.0.0
```

Generated model code produces `Model!(Post { name: String })` with the table name `posts`
(pluralised from the struct name). Generated code compiles warning-clean and is verified in CI.

`foundry-cli` (the legacy command name) is retained for backward compatibility but `forge` is
the canonical CLI.

---

## Templates and views

**[beta]** — crates: `rf-blade`, `rf-views`, `rf-view`

Tera-based template rendering with Blade-like directives (`@if`, `@foreach`, `@auth`, `@csrf`).
`view("template", data)` from the stable `rf-response` crate renders templates in
`resources/views/`. The template engine crates themselves are beta.

---

## GraphQL

**[beta]** — crate: `rf-graphql`

async-graphql 7.0 integration; per-request auth context injected. API may shift in minor versions.
Not load-tested against large schemas.

---

## OAuth and social login

**[beta]** — crates: `rf-socialite` (preferred for social login), `rf-passport` (OAuth2 server)

- `rf-socialite` — GitHub, Google, Facebook, Twitter OAuth2 client flows.
- `rf-passport` — Laravel Passport-style complete OAuth2 authorization server (PKCE, client credentials,
  personal access tokens, scope management). Requires a live DB.
- `rf-oauth` — lighter OAuth2 client (overlaps `rf-socialite`; `rf-socialite` is preferred).

Do not start new code in `rf-oauth-server` or `rf-oauth2-server` — these are deprecated beta crates
to be removed in a future cleanup.

---

## AI and vector search

**[beta]** — crates: `rf-ai`, `rf-vector`

`rf-ai` provides provider-agnostic `ChatProvider` / `EmbeddingProvider` traits with an Anthropic
Messages API provider (`AnthropicProvider`) and a deterministic `MockChatProvider` for offline tests.

`rf-vector` provides dense embedding vectors (`Vector`), similarity metrics (`Cosine`, `Euclidean`,
`DotProduct`), an `InMemoryVectorStore` for k-nearest-neighbour search, and `pgvector` SQL helpers.

Beta tier: API may shift in minor versions; not load-tested.

---

## Testing utilities

**[beta]** — crate: `rf-testing`

`HttpTester` wraps an axum `Router` for HTTP-level integration tests. `TestDatabase` spins up a
test database. Real but minimal coverage; API may evolve.

---

## Task scheduling

**[beta]** — crates: `rf-scheduling`, `rf-scheduler`

Cron-expression scheduler with fluent builder. Two crates exist with overlapping scope and are
planned to be unified. API may shift.

---

## Browser testing

**[beta]** — crate: `rf-dusk`

WebDriver-based browser testing via `fantoccini`. Inspired by Laravel Dusk.
Chrome, Firefox, Safari supported. API may evolve.

---

## Service container

**[beta]** — crates: `rf-service-container`, `rf-container`

DI container with singleton, scoped, and transient registrations. Two crates with overlapping scope;
prefer `rf-service-container` for new code. API may evolve.

---

## Experimental crates

The following 8 crates are **excluded from `default-members`** and from the 1.0 supported surface.
They are compiled by `cargo check --workspace` to prevent bitrot, but carry **no SemVer guarantee**
and may be removed or renamed without a major-version bump.

| Crate | State |
|-------|-------|
| `rf-nova` | Nova admin panel — multi-resource type-erased dispatch unfinished |
| `rf-nova-macros` | `#[derive(Resource)]` generates broken stubs |
| `rf-swagger` | OpenAPI/utoipa — route-annotation-only, no auto-scan |
| `rf-telescope` | Debugging dashboard — stub implementation |
| `rf-cms` | CMS — media processing/versioning unfinished |
| `rf-breeze` | Auth scaffolding — depends on `rf-blade`; not integration-tested |
| `rf-vite` | Vite asset pipeline — dev-tool only; not verified against axum 0.8 |
| `rf-livereload` | Live reload/HMR — WebSocket watcher not integration-tested |

Do not depend on these crates in production code.

---

## Full crate roster

See [docs/TIERS.md](../../TIERS.md) for the complete list of all 127 crates
(34 stable / 76 beta / 8 experimental / 9 stub) with per-crate justifications.
