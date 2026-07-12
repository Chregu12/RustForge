# RustForge v1 Stable Core — API Contract

> **Cross-references:** [docs/TIERS.md](TIERS.md) — tier definitions and the full crate roster;
> [docs/RELEASING.md](RELEASING.md) — SemVer policy and the canonical stable surface list.

This document defines the **precise v1 API contract** for the RustForge stable core.
Every entry point listed here is grep-verified to exist in the source tree at the
time of writing. Adding items to this document (or to the `rf::prelude`) is an
additive, backward-compatible change. Removing or renaming items requires a MAJOR
version bump.

---

## Quick-start: the single import

```rust
use rf::prelude::*;
```

`rf::prelude` re-exports the entire stable surface listed below. One `use` statement
is sufficient for typical handler files. The `rf` crate (`crates/rf`) is the umbrella;
all items flow from the individual stable crates described in §Capabilities.

---

## Capabilities

### 1. Routing

**Crate:** `rf-routing` (**stable** tier — this is the routing facade the
`rf::prelude` exposes: `get/post/put/delete/patch`, `resource`, `Route`,
`global_router`)

**Prelude entry points** (grep-verified in `crates/rf-routing/src/`):

| Item | Location | Description |
|------|----------|-------------|
| `get(path, handler)` | `rf_routing::get` | Register GET handler on the global router |
| `post(path, handler)` | `rf_routing::post` | Register POST handler |
| `put(path, handler)` | `rf_routing::put` | Register PUT handler |
| `patch(path, handler)` | `rf_routing::patch` | Register PATCH handler |
| `delete(path, handler)` | `rf_routing::delete` | Register DELETE handler |
| `resource(path, controller)` | `rf_routing::resource` | RESTful resource routing sugar |
| `Route` (alias `RouteFacade`) | `rf_routing::RouteFacade` | Fluent route registration facade |
| `global_router()` | `rf_routing::global_router` | Returns the global `GlobalRouter` |
| `GlobalRouter` | `rf_routing::GlobalRouter` | Holds registered routes, builds `axum::Router` |

**Build the axum app:**
```rust
let app = rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

**Example:** `examples/blog-slice/` · `examples/rest-crud-resource/` · `examples/laravel-syntax-simple/`

---

### 2. Request / Response

**Crates:** `rf-request` (stable) · `rf-response` (stable)

#### Request globals (task-local, requires `capture_request` middleware)

Grep-verified in `crates/rf-request/src/context.rs`:

| Function | Signature | Description |
|----------|-----------|-------------|
| `input(key)` | `fn input<T: DeserializeOwned>(key: &str) -> Option<T>` | Read a body / query field |
| `has(key)` | `fn has(key: &str) -> bool` | Check field presence |
| `file(name)` | `fn file(name: &str) -> Option<UploadedFile>` | Access an uploaded file |
| `all()` | `fn all() -> HashMap<String, Value>` | All input fields as a map |
| `capture_request` | `async fn capture_request(req, next) -> Response` | axum middleware that buffers the body and populates the task-local; must be on the router layer for `input`/`has`/`file`/`all` to work |

#### Response helpers

Grep-verified in `crates/rf-response/src/`:

| Item | Description |
|------|-------------|
| `json(data)` | Build an `application/json` response from any `Serialize` value |
| `view(name, data)` | Render `resources/views/<name>.blade.html`, return `text/html` |
| `back()` | Redirect to the Referer / `/` |
| `download(path)` | Serve file bytes with `Content-Disposition` |
| `Response` | The `ResponseBuilder` type (`impl IntoResponse`) |

**Example:** `examples/blog-slice/` · `examples/validated-signup/`

---

### 3. Validation

**Crates:** `rf-validation` (stable) · `rf-validation-derive` (stable)

| Item | Crate | Description |
|------|-------|-------------|
| `validate! { field: rule.modifier, … }` | `rf_macros::validate` | Typed validation DSL; returns `Result<ValidatedData, ValidationErrors>` |
| `#[derive(Validate)]` | `rf_validation_derive::Validate` | Derive macro for struct-level validation |
| `ValidatedJson<T>` | `rf_validation::ValidatedJson` | axum extractor that validates JSON body against `T: Validate` |
| `ValidationErrors` | `rf_validation::ValidationErrors` | Structured per-field error map |
| `FieldError` | `rf_validation::FieldError` | Per-field error detail |
| `Validator` | `rf_validation::Validator` | Programmatic validator builder |

**Built-in rule types (48+):** `string`, `integer`, `float`, `boolean`, `email`,
`url`, `uuid`, `date`, `min`, `max`, `between`, `required`, `nullable`, `unique`,
`confirmed`, `in`, `not_in`, `regex`, `alpha`, `alpha_num`, `numeric`, `json`,
`image`, `mimes`, `max_file_size` (+ `kb(n)` / `mb(n)` helpers), and more.

**Example:** `examples/validated-signup/` · `examples/laravel-syntax-complete/`

---

### 4. ORM

**Crate:** `rf-orm` (stable), with Eloquent sugar in `rf-eloquent` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Model!(T: fields)` / `Model!(T { … })` | `rf_macros::Model` | Declare a model + table mapping + DTO types |
| `create!(Model, field = val, …)` | `rf_macros::create` | INSERT; returns new row as `serde_json::Value` |
| `find!(Model, id)` | `rf_macros::find` | SELECT by primary key; returns `Option` |
| `update!(Model, id, field = val, …)` | `rf_macros::update` | UPDATE by primary key |
| `delete!(Model, id)` | `rf_macros::delete` | DELETE by primary key |
| `DB` | `rf_orm::DB` | Facade: `DB::table("users").where_("id", 1).first().await` |
| `DatabaseManager` | `rf_orm::DatabaseManager` | Manages connection pool; used in `AppState` |
| `QueryBuilder` | `rf_orm::QueryBuilder` | Fluent query builder returned by `DB::table` |
| `Model` (trait) | `rf_orm::Model` | Trait all ORM models implement |
| `Transaction` / `TransactionExt` | `rf_orm::Transaction` | Async transaction blocks |
| `SoftDelete` | `rf_orm::SoftDelete` | Soft-delete mixin |
| `migration! { … }` | `rf_macros::migration` | Schema migration DSL |

**Example:** `examples/blog-slice/` · `examples/phase12-blog/` · `examples/rest-crud-resource/`

---

### 5. Authentication

**Crate:** `rf-auth` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Auth` | `rf_auth::Auth` | Facade: `Auth::user()`, `Auth::check()`, `Auth::login(user)`, `Auth::logout()` |
| `require_auth` | `rf_auth::require_auth` | axum middleware; rejects unauthenticated requests with JSON 401 before body extraction |
| `JwtManager` | `rf_auth::JwtManager` | Issue and verify JWT tokens |
| `Claims` | `rf_auth::Claims` | JWT claims struct |
| `PasswordHasher` | `rf_auth::PasswordHasher` | bcrypt/argon2 password hashing (wraps `rf-global-helpers::Hash`) |
| `Guard` | `rf_auth::Guard` | Auth guard trait for custom auth strategies |
| `AuthError` / `AuthResult` | `rf_auth::AuthError` | Auth-specific error/result types |

**Usage:**
```rust
// Protect a route — add as a route layer so auth runs before body extraction
Router::new()
    .route("/api/me", get(me_handler))
    .route_layer(axum::middleware::from_fn(require_auth))
```

**Example:** `examples/auth-paginated-search/` · `examples/auth-demo/`

---

### 6. Configuration

**Crate:** `rf-config` (**stable** tier — `Config` / `AppConfig::from_env` are
re-exported in the prelude and covered by the 1.0 stable surface)

| Item | Location | Description |
|------|----------|-------------|
| `Config` | `rf_config::Config` | Facade: `Config::get("app.name")`, `Config::set(key, val)` |
| `AppConfig` | `rf_config::AppConfig` | Typed config struct loaded from `config/*.toml` + env vars |

---

### 7. Cache

**Crate:** `rf-cache` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Cache` (alias `CacheFacade`) | `rf_cache::CacheFacade` | Facade: `Cache::get(key)`, `Cache::put(key, val, ttl)`, `Cache::forget(key)`, `Cache::remember(key, ttl, \|\| val)` |
| `Cache` (trait) | `rf_cache::Cache` | The driver trait; implement for custom backends |
| `MemoryCache` | `rf_cache::MemoryCache` | In-process memory driver (default, zero-config) |
| `RedisCache` | `rf_cache::RedisCache` | Redis driver (requires `redis` feature) |
| `CacheManager` | `rf_cache::CacheManager` | Manages multiple named cache stores |

**Example:** `examples/facades-demo/`

---

### 8. Queue / Background Jobs

**Crate:** `rf-queue` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Queue` (trait) | `rf_queue::Queue` | Driver trait: `push(job)`, `pop()`, `failed()` |
| `Job` (alias `QueueJob` in prelude) | `rf_queue::Job` | Trait to implement on job structs; `handle(&self) -> Result<()>` |
| `MemoryQueue` | `rf_queue::MemoryQueue` | Zero-config in-process driver; panic-isolated, DLQ-capable |
| `Worker` | `rf_queue::Worker` | Drains a `Queue`, spawning async tasks with retry and dead-letter |
| `dispatch(job)` | `rf_queue::dispatch` | Dispatch a job onto the default queue |
| `Jobs` | `rf_queue::Jobs` | Fluent job dispatch facade |

**Note:** The `rf_macros::job` macro and `rf-jobs` crate provide Redis-backed workers
for production. `rf-queue` (`MemoryQueue` + `Worker`) is the stable in-process driver.

**Example:** `examples/jobs-demo/`

---

### 9. Mail

**Crate:** `rf-mail` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Mail` (alias `MailFacade`) | `rf_mail::MailFacade` | Facade: `Mail::to(addr).subject(s).body(b).send()` |
| `Mailable` | `rf_mail::Mailable` | Trait for typed mailable objects |
| `MailableAsync` | `rf_mail::MailableAsync` | Async variant of `Mailable` |
| `MessageBuilder` | `rf_mail::MessageBuilder` | Fluent message construction |
| `MailBuilder` | `rf_mail::MailBuilder` | Alternative builder API |
| `SmtpMailer` / `FileMailer` | `rf_mail::backends` | SMTP (lettre) and file-log transports |

---

### 10. Storage

**Crate:** `rf-storage` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Storage` (alias `StorageFacade`) | `rf_storage::StorageFacade` | Facade: `Storage::put(path, bytes)`, `Storage::get(path)`, `Storage::delete(path)`, `Storage::url(path)` |
| `Storage` (trait) | `rf_storage::Storage` | Driver trait for custom backends |
| `LocalStorage` | `rf_storage::LocalStorage` | Local-filesystem driver; path-traversal-safe |
| `MemoryStorage` | `rf_storage::MemoryStorage` | In-process driver (tests / defaults) |
| `S3Storage` / `S3Config` | `rf_storage::S3Storage` | AWS S3 driver (requires `s3` feature) |

---

### 11. Events

**Crate:** `rf-events` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Event` (alias `EventFacade`) | `rf_events::EventFacade` | Facade: `Event::dispatch(name, data)`, `Event::listen(name, \|data\| …)` |
| `EventDispatcher` | `rf_events::EventDispatcher` | Type-keyed async dispatcher; listener panics isolated |
| `EventManager` | `rf_events::EventManager` | Sync global bus backing the `Event` facade |
| `EventListener` | `rf_events::EventListener` | Async listener trait |
| `EventListenerFor<E>` | `rf_events::EventListenerFor` | Typed listener trait for event `E` |

---

### 12. Facades (consolidated)

**Crate:** `rf-facades` (stable)

`use rf::facades::*` brings all facades into scope at once:
`Route`, `Auth`, `DB`, `Cache`, `Event`, `Storage`, `Log`, `Mail`, `Session`,
`Config`, `View`.

Alternatively each facade is available through `use rf::prelude::*` (preferred).

---

### 13. Error Handling

**Crate:** `rf-core` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `AppError` | `rf_core::AppError` | Framework error; implements `axum::IntoResponse` (renders RFC 7807 JSON) |
| `AppResult<T>` | `rf_core::AppResult` | `Result<T, AppError>` — use as handler return type for `?`-ergonomics |
| `OrNotFound` (trait) | `rf_core::OrNotFound` | `.or_404()` on `Option<T>` and `Result<Option<T>, E>` |
| `RfResult` | prelude alias | `rf_errors::Result<T>` — one-parameter alias; named `RfResult` in prelude to avoid shadowing `std::result::Result` |
| `RustForgeError` | `rf_errors::RustForgeError` | Rich multi-variant error type with structured context |

**`?`-based handler pattern:**
```rust
async fn show_post(Path(id): Path<i64>) -> AppResult<impl IntoResponse> {
    let post = find!(Post, id).or_404()?;
    Ok(json(post))
}
```

---

### 14. Helpers

**Crates:** `rf-global-helpers` (stable) · `rf-helpers` (stable)

| Item | Location | Description |
|------|----------|-------------|
| `Hash::make(pw)` | `rf_global_helpers::Hash` | BCrypt/Argon2 password hashing |
| `Hash::check(pw, hash)` | `rf_global_helpers::Hash` | Verify a password against a hash |
| `redirect(path)` | `rf_global_helpers::redirect` | Build a `RedirectResponse` with flash-message chaining |
| `back()` | `rf_global_helpers::back` | Redirect to previous page |
| `csrf_token()` | `rf_global_helpers::csrf_token` | Generate a CSRF token string |
| `csrf_field()` | `rf_global_helpers::csrf_field` | HTML hidden-input for CSRF |
| `Collection<T>` | `rf_collections::Collection` | Laravel-style collection with `map`, `filter`, `pluck`, `first`, `paginate`, … |
| `collect(iter)` | `rf_collections::collect` | Convert any iterator into a `Collection` |

---

## What the prelude does NOT include

The following items are intentionally absent from `rf::prelude` (or not in the v1
stable contract) as of 1.0.0-rc.1:

| Item | Reason |
|------|--------|
| `Pest` / `Cashier` / `Mcp` / `Nightwatch` | Beta-tier crates retained in prelude for backward compatibility only; will be moved behind an opt-in feature flag in a future minor release |
| `rf-nova` / `rf-swagger` / `rf-telescope` | Experimental tier — no 1.x API guarantee |
| `rf-breeze` / `rf-vite` / `rf-livereload` | Experimental tier — dev/scaffold tooling |
| `rf-socialite` / `rf-ai` / `rf-vector` | Alpha — subject to removal or rename |
| `rf_errors::Result` (bare name) | Deliberately renamed to `RfResult` in prelude to prevent shadowing `std::result::Result` |

---

## Prelude completeness check

The following stable-tier items are re-exported through `use rf::prelude::*`:

- **Routing:** `get`, `post`, `put`, `patch`, `delete`, `resource`, `Route`, `global_router`
- **Request:** `input`, `has`, `file`, `capture_request` (via `rf::web::capture_request`)
- **Response:** `json`, `view`, `back`, `download`, `Response`
- **Validation:** `validate!`, `#[derive(Validate)]`, `ValidatedJson`, `ValidationErrors`
- **ORM:** `Model!`, `create!`, `find!`, `update!`, `delete!`, `DB`, `QueryBuilder`
- **Auth:** `Auth`, `require_auth`
- **Cache:** `Cache` (facade)
- **Queue:** `Queue`, `Job` (as `QueueJob`), `MemoryQueue`, `Worker`
- **Mail:** `Mail`
- **Storage:** `Storage`
- **Events:** `Event`
- **Config:** `Config`
- **Helpers:** `Hash`, `redirect`, `csrf_token`, `csrf_field`, `Collection`, `collect`
- **Errors:** `AppError`, `AppResult`, `OrNotFound`, `RfResult`, `RustForgeError`
- **Session / View:** `Session`, `View`

All items above resolve from a single `use rf::prelude::*` statement with no
additional direct crate dependencies beyond `rf` itself (plus `serde` for
`#[derive(Serialize)]` and `tokio` for `#[tokio::main]`).

---

## NOT in contract: internal / infra crates

The following crates are used internally by the stable surface but are not
themselves part of the v1 API contract and may change without a MAJOR bump:

`rf-async-bridge`, `rf-db-facade`, `rf-auth-facade`, `rf-cache-facade`,
`rf-event-facade`, `rf-mail-facade`, `rf-route-facade`, `rf-storage-facade`,
`rf-model-macro`, `rf-macros` (macro expansion details only), `rf-global-helpers`
(exposed via the `rf::helpers` module but not a first-class stable surface),
`rf-collections` (exposed via `rf::data` and prelude; stable items: `Collection`,
`collect`, `LazyCollection`).
