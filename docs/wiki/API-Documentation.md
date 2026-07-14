# API Documentation — RustForge v1 Reference

This page is the **reference-style companion** to [Laravel-Syntax.md](Laravel-Syntax.md).
It lists the concrete types, function signatures, return types, and crate locations for every
stable capability. All items below are grep-verified against the source tree.

**Authoritative sources:**
- [docs/STABLE_CORE.md](../STABLE_CORE.md) — the v1 API contract (every entry point with its source location)
- [docs/TIERS.md](../TIERS.md) — maturity tiers for all 127 crates
- [docs/API_PHILOSOPHY.md](../API_PHILOSOPHY.md) — the two-layer framing and honest trade-offs

---

## Prelude

```rust
use rf::prelude::*;
```

One import makes every stable item on this page available in handler files.
It is a re-export of `rf::prelude`, defined in `crates/rf/src/lib.rs`.
No direct dependency on individual crates is required beyond `rf`, `serde`, and `tokio`.

Items visible after this import:

| Group | Items |
|-------|-------|
| Routing | `get`, `post`, `put`, `patch`, `delete`, `resource`, `Route`, `global_router` |
| Request | `input`, `has`, `file`, `all`, `capture_request` (via `rf::web::capture_request`) |
| Response | `json`, `view`, `back`, `download`, `Response` |
| ORM macros | `Model!`, `create!`, `find!`, `update!`, `delete!`, `DB` |
| Validation | `validate!`, `ValidatedJson` (via `rf_validation`), `ValidationErrors` |
| Auth | `Auth`, `require_auth` |
| Cache | `Cache` (= `rf_cache::CacheFacade`) |
| Mail | `Mail` (= `rf_mail::MailFacade`) |
| Storage | `Storage` (= `rf_storage::StorageFacade`) |
| Events | `Event` (= `rf_events::EventFacade`) |
| Queue | `MemoryQueue`, `Worker`, `Jobs` |
| Config | `Config` |
| Helpers | `Hash`, `redirect`, `csrf_token`, `csrf_field`, `Collection`, `collect` |
| Errors | `AppError`, `AppResult`, `OrNotFound`, `RfResult`, `RustForgeError` |
| Session/View | `Session`, `View` |

---

## 1. Routing

**Crate:** `rf-routing` (stable) — `crates/rf-routing/src/`

### Free functions (register on the global router)

| Signature | Description |
|-----------|-------------|
| `get(path: &str, handler: H)` | Register a GET route |
| `post(path: &str, handler: H)` | Register a POST route |
| `put(path: &str, handler: H)` | Register a PUT route |
| `patch(path: &str, handler: H)` | Register a PATCH route |
| `delete(path: &str, handler: H)` | Register a DELETE route |
| `resource(path: &str, controller: C)` | Expand RESTful resource routes (index, store, show, update, destroy) |

Path parameters use axum `{id}` syntax (e.g. `/posts/{id}`).

### Building the axum app

```rust
let app: axum::Router = rf::global_router()
    .build_router()
    .layer(axum::middleware::from_fn(rf::web::capture_request));
```

`global_router()` returns `&'static GlobalRouter`. `build_router()` returns `axum::Router`.
`capture_request` must be present for `input`/`has`/`file`/`all` to work in handlers.

### Types

| Type | Location | Description |
|------|----------|-------------|
| `GlobalRouter` | `rf_routing::GlobalRouter` | Accumulates registered routes; builds the axum router |
| `Route` / `RouteFacade` | `rf_routing::RouteFacade` | Fluent route registration facade |

---

## 2. Request globals

**Crate:** `rf-request` (stable) — `crates/rf-request/src/context.rs`

All four functions read from a **per-request task-local** populated by `capture_request`. They
return `None` / `false` / empty silently when called outside that middleware. See
[API_PHILOSOPHY.md](../API_PHILOSOPHY.md) for the full caveat.

| Function | Signature | Returns |
|----------|-----------|---------|
| `input(key)` | `fn input<T: DeserializeOwned>(key: &str) -> Option<T>` | Any deserializable value from body or query; coerces strings to numeric types |
| `has(key)` | `fn has(key: &str) -> bool` | True if the field is present in body or query |
| `file(name)` | `fn file(name: &str) -> Option<UploadedFile>` | An uploaded multipart file |
| `all()` | `fn all() -> HashMap<String, serde_json::Value>` | All input fields |

### Middleware

| Item | Signature | Description |
|------|-----------|-------------|
| `capture_request` | `async fn capture_request(req: Request, next: Next) -> Response` | Buffers the body and populates the task-local. Body re-inserted so downstream axum extractors (`Json<T>`, `ValidatedJson<T>`) can also read it. |

---

## 3. Response helpers

**Crate:** `rf-response` (stable) — `crates/rf-response/src/`

| Item | Signature / Usage | Description |
|------|-----------|-------------|
| `json(data)` | `fn json<T: Serialize>(data: T) -> ResponseBuilder` | Build `application/json` response |
| `json(data).status(code)` | `ResponseBuilder::status(StatusCode) -> ResponseBuilder` | Set HTTP status |
| `view(name, data)` | `fn view(name: &str, data: impl Serialize) -> ResponseBuilder` | Render `resources/views/<name>.blade.html` |
| `back()` | `fn back() -> RedirectResponse` | Redirect to Referer or `/` |
| `redirect(path)` | `fn redirect(path: &str) -> RedirectResponse` | Build a redirect response |
| `download(path)` | `fn download(path: &str) -> ResponseBuilder` | Serve file with `Content-Disposition: attachment` |
| `Response::no_content()` | `fn no_content() -> ResponseBuilder` | 204 No Content |

---

## 4. ORM — Models and CRUD

**Crates:** `rf-orm` (stable), `rf-eloquent` (stable), `rf-macros` (stable)

### `Model!` macro

Declares a model, generates an entity struct, a `Create<Name>` DTO, and a `Model` trait
implementation. The table name is the plural snake-case of the struct name.

```
Model!(Name: field1, field2)           // concise — field types inferred as String
Model!(Name { field: Type, ... })      // explicit types
Model!(Name { validated, field: Type @ rule, ... })   // with @ validation rules
Model!(Name { field: Type, belongsTo rel: Other, })   // with relation
Model!(Name { field: Type, hasMany rels: Child, })    // hasMany relation
Model!(Name { field: Type, scope name: where("col", val), })  // named scope
```

The `validated` keyword makes the generated DTO implement `rf_validation::Validate`,
enabling `ValidatedJson<Create<Name>>`.

### CRUD macros

All macros are synchronous (AsyncBridge) and return `Result<_, String>`.

| Macro | Return type | Notes |
|-------|-------------|-------|
| `create!(Model, field = val, ...)` | `Result<serde_json::Value, String>` | INSERT; returned value includes `id` |
| `find!(Model, id)` | `Result<Option<serde_json::Value>, String>` | SELECT by PK |
| `update!(Model, id, field = val, ...)` | `Result<usize, String>` | UPDATE by PK; 0 = not found |
| `delete!(Model, id)` | `Result<usize, String>` | DELETE by PK; 0 = not found |

**PK convention:** the primary key column must be named `id`. On Postgres, `RETURNING id` is
appended automatically.

### Eloquent methods (on generated model structs)

| Method | Return type | Notes |
|--------|-------------|-------|
| `Model::all().await` | `Result<Vec<Model>, _>` | SELECT all rows |
| `Model::find(id).await` | `Result<Option<Model>, _>` | SELECT by PK |
| `Model::paginate(per_page, page).await` | `Result<PaginatedResult<Model>, _>` | Paginated SELECT |
| `Model::with(&["rel"]).get().await` | `Result<Vec<Model>, _>` | Eager-load relation |
| `Model::with(&["rel"]).r#where("col", val).get().await` | `Result<Vec<Model>, _>` | Filtered + eager-loaded |
| `Model::with(&["rel.nested"]).get().await` | `Result<Vec<Model>, _>` | Nested eager-load |
| `Model::<scope_name>().get().await` | `Result<Vec<Model>, _>` | Named scope |

### DB facade

**Crate:** `rf-orm` (stable) — `crates/rf-orm/src/`

| Method | Return type | Notes |
|--------|-------------|-------|
| `DB::table(name)` | `QueryBuilder` | Start a fluent builder |
| `DB::statement(sql)` | `Result<(), String>` | Raw DDL / DML (sync) |
| `DB::select(sql, params)` | `Result<Vec<serde_json::Value>, String>` | Raw parameterised SELECT (sync) |
| `DB::insert(sql, params)` | `Result<i64, String>` | Raw INSERT; returns last_insert_id (sync) |
| `DB::connection(url)` | `Result<(), String>` | Switch backend URL at runtime |
| `DB::connection_name()` | `&'static str` | `"postgres"` or `"sqlite"` |

#### QueryBuilder methods (all async unless noted)

| Method | Notes |
|--------|-------|
| `.r#where(col, val)` | Add WHERE col = val |
| `.where_like(col, pattern)` | LIKE with raw pattern |
| `.where_like_escaped(col, term)` | LIKE with %, _, \ escaped in `term` (safe for user input) |
| `.order_by(col, dir)` | ORDER BY col ASC/DESC |
| `.limit(n)` | LIMIT n |
| `.offset(n)` | OFFSET n |
| `.get().await` | `Result<Vec<serde_json::Value>, String>` |
| `.first().await` | `Result<Option<serde_json::Value>, String>` |
| `.count().await` | `Result<i64, String>` |
| `.with_where(relation, col, val)` | Constrain an eager-loaded relation |

#### Database backends

| `DATABASE_URL` value | Backend |
|---|---|
| Absent / empty | In-memory SQLite (default; data lost on exit) |
| File path | SQLite file |
| `postgres://...` or `postgresql://...` | Postgres via sqlx PgPool |

---

## 5. Validation

**Crates:** `rf-validation` (stable) — `crates/rf-validation/src/`
`rf-validation-derive` (stable) — `crates/rf-validation-derive/`

### `validate!` macro

Reads from the `capture_request` task-local. Returns `Result<ValidatedData, ValidationErrors>`.
Check with `.is_err()`.

Type prefixes: `string`, `int`, `float`, `boolean`, `email`, `url`, `uuid`, `date`, `image`, `file`.
Modifiers: `min(N)`, `max(N)`, `between(lo, hi)`, `required`, `optional`, `nullable`,
`confirmed`, `in`, `not_in`, `regex`, `alpha`, `alpha_num`, `numeric`, `json`, `image`, `mimes`,
`kb(N)`, `mb(N)`.

### `ValidatedJson<T>` extractor

```rust
// T must implement rf_validation::Validate (e.g. generated by Model! + validated).
async fn handler(ValidatedJson(dto): ValidatedJson<CreateUser>) -> impl IntoResponse { ... }
```

Validates before the handler body runs. Invalid body yields `422 Unprocessable Entity` with the
`{ "errors": { "field": [{ "code": "..", "message": ".." }] } }` JSON envelope.

### Key types

| Type | Location | Description |
|------|----------|-------------|
| `ValidationErrors` | `rf_validation::ValidationErrors` | Per-field error map |
| `FieldError` | `rf_validation::FieldError` | Single field error (code + message + params) |
| `Validate` (trait) | `rf_validation::Validate` | Implemented via `#[derive(Validate)]` or `Model! { validated }` |
| `ValidatedJson<T>` | `rf_validation::ValidatedJson` | axum extractor |
| `ValidatedData` | `rf_validation::ValidatedData` | Validated field map returned by `validate!` |

---

## 6. Authentication

**Crate:** `rf-auth` (stable) — `crates/rf-auth/src/`

### Middleware

| Item | Signature | Notes |
|------|-----------|-------|
| `require_auth` | `async fn require_auth(req: Request, next: Next) -> Response` | Reads `JwtManager` from an `Extension`; fail-closed (401) if absent |
| `require_auth_with(manager)` | `fn require_auth_with(manager: Arc<JwtManager>) -> impl Fn(Request, Next) -> BoxFuture<'static, Response> + Clone + Send + 'static` | Factory — pass the pre-built manager; returned closure is usable with `from_fn` |

Both middlewares:
1. Extract `Authorization: Bearer <jwt>` from headers only (never the body).
2. Validate via `JwtManager::validate_token`.
3. On success: open a per-request auth scope; inject `Extension<Claims>`.
4. On failure: return `AppError::Unauthorized` as JSON 401.

### `Auth` facade

`rf_auth::Auth` (re-exported as `rf::Auth` in the prelude). Reads from a per-request
task-local opened by `require_auth` / `require_auth_with`. Returns `None` outside that scope.

| Method | Return type | Notes |
|--------|-------------|-------|
| `Auth::check()` | `bool` | True if a user is authenticated in this request scope |
| `Auth::guest()` | `bool` | Inverse of `check()` |
| `Auth::user()` | `Option<serde_json::Value>` | Authenticated user as JSON |
| `Auth::id()` | `Option<i64>` | Authenticated user ID |
| `Auth::login(user)` | `Result<_, _>` | Log in a user in the current scope |
| `Auth::logout()` | `()` | Log out |

### `JwtManager`

`rf_auth::JwtManager` — JWT issue and verify.

| Method | Signature | Notes |
|--------|-----------|-------|
| `JwtManager::new(secret)` | `fn new(secret: &str) -> Result<JwtManager, _>` | Secret must be at least 32 characters |
| `jwt.generate_token(&claims)` | `fn generate_token(&self, claims: &Claims) -> Result<String, _>` | Issue a signed JWT |
| `jwt.validate_token(&token)` | `fn validate_token(&self, token: &str) -> Result<Claims, _>` | Verify signature and expiry |

### `Claims`

`rf_auth::Claims` — JWT payload.

```rust
let claims = Claims::new(
    user_id as i32,           // user_id: i32
    email.clone(),             // sub: String
    vec!["user".to_string()], // roles: Vec<String>
    24,                        // hours until expiry: u64
);
```

Fields available in `Extension<Claims>`: `claims.user_id: i32`, `claims.sub: String`,
`claims.roles: Vec<String>`.

### `PasswordHasher`

`rf_auth::PasswordHasher` — bcrypt/argon2 password hashing.

| Method | Notes |
|--------|-------|
| `PasswordHasher::bcrypt(cost)` | `fn bcrypt(cost: u32) -> Result<PasswordHasher, _>` |
| `hasher.hash(pw)` | `fn hash(&self, pw: &str) -> Result<String, _>` |
| `hasher.verify_timing_safe(pw, hash)` | `fn verify_timing_safe(&self, pw: &str, hash: &str) -> Result<bool, _>` |

For a simpler API without cost control: `Hash::make(pw) -> String` and
`Hash::check(pw, hash) -> bool` from `rf_global_helpers` (re-exported as `rf::Hash`).

---

## 7. Cache

**Crate:** `rf-cache` (stable) — `crates/rf-cache/src/`

Re-exported as `Cache` in the prelude (`rf_cache::CacheFacade`). All methods are synchronous
(AsyncBridge). Return type is `CacheResult<T>` = `Result<T, CacheError>`.

### Default backend

`MemoryCache` (in-process, zero-config). The default is selected at startup; switching to Redis
requires configuring the global manager:

```rust
// Redis backend (requires `redis` feature in rf-cache).
let backend = rf_cache::CacheConfig::redis("redis://127.0.0.1:6379", "myapp:")
    .build().await?;
rf_cache::CacheManager::set_global(backend);
```

### `CacheFacade` methods

| Method | Signature | Notes |
|--------|-----------|-------|
| `Cache::get::<T>(key)` | `fn get<T: DeserializeOwned>(key: &str) -> CacheResult<Option<T>>` | |
| `Cache::put(key, val, ttl)` | `fn put<T: Serialize, TTL: IntoTtl>(key: &str, val: T, ttl: TTL) -> CacheResult<()>` | TTL accepts `u64` seconds or `Duration` |
| `Cache::forever(key, val)` | `fn forever<T: Serialize>(key: &str, val: T) -> CacheResult<()>` | No expiry |
| `Cache::forget(key)` | `fn forget(key: &str) -> CacheResult<()>` | Delete one key |
| `Cache::flush()` | `fn flush() -> CacheResult<()>` | Delete all |
| `Cache::has(key)` | `fn has(key: &str) -> CacheResult<bool>` | |
| `Cache::touch(key, ttl)` | `fn touch<TTL: IntoTtl>(key: &str, ttl: TTL) -> CacheResult<bool>` | Extend TTL without rewriting value |
| `Cache::remember(key, ttl, f)` | `fn remember<T, F, Fut, TTL>(key, ttl, f) -> CacheResult<T>` | Cache-aside; calls `f` on miss |
| `Cache::remember_forever(key, f)` | Similar to `remember` with no expiry | |
| `Cache::pull::<T>(key)` | `fn pull<T: DeserializeOwned>(key: &str) -> CacheResult<Option<T>>` | Get + delete atomically |
| `Cache::add(key, val, ttl)` | `fn add<T, TTL>(key, val, ttl) -> CacheResult<bool>` | True if key was absent (atomic) |
| `Cache::increment(key, n)` | `fn increment(key: &str, n: i64) -> CacheResult<i64>` | |
| `Cache::decrement(key, n)` | `fn decrement(key: &str, n: i64) -> CacheResult<i64>` | |
| `Cache::tags(tags)` | `fn tags(tags: &[&str]) -> TaggedCache` | Tagged cache operations |

---

## 8. Mail

**Crate:** `rf-mail` (stable) — `crates/rf-mail/src/`

Re-exported as `Mail` in the prelude (`rf_mail::MailFacade`).

Default transport: `FileMailer` (writes `.eml` to `$MAIL_MAILBOX` or `/tmp/rustforge-mailbox`).
SMTP transport: set `SMTP_HOST` env var, or call `MailFacade::smtp(SmtpConfig)` at startup.

### `Mailable` trait

```rust
use rf_mail::{Address, MailBuilder, Mailable};

struct WelcomeMail { to: String }

impl Mailable for WelcomeMail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new(&self.to))
            .subject("Welcome!")
            .text("Welcome to the app!")
            // .html("<h1>Welcome!</h1>")
            // .attach("file.pdf", bytes)
    }
}
```

### `MailFacade` methods

| Method | Signature | Notes |
|--------|-----------|-------|
| `Mail::send(mailable)` | `fn send<M: Mailable>(m: M) -> Result<(), MailError>` | Sync send via configured transport |
| `Mail::to(addr)` | `fn to(addr: &str) -> Mailer` | Recipient-first builder handle |
| `Mailer::send(mailable)` | `fn send<M: Mailable>(m: M) -> Result<(), MailError>` | |
| `MailFacade::smtp(cfg)` | `fn smtp(cfg: SmtpConfig) -> Result<(), _>` | Configure SMTP transport at startup |
| `MailFacade::mailbox(path)` | `fn mailbox(path: &Path)` | Configure FileMailer directory at startup |

### Key types

| Type | Location | Description |
|------|----------|-------------|
| `Mailable` | `rf_mail::Mailable` | Trait: implement `build(&self) -> MailBuilder` |
| `MailBuilder` | `rf_mail::MailBuilder` | Chainable message builder |
| `Address` | `rf_mail::Address` | Email address wrapper |
| `SmtpConfig` | `rf_mail::SmtpConfig` | SMTP connection settings |
| `FileMailer` | `rf_mail::FileMailer` | File-system mail transport (test/dev) |

---

## 9. Storage

**Crate:** `rf-storage` (stable) — `crates/rf-storage/src/`

Re-exported as `Storage` in the prelude (`rf_storage::StorageFacade`).

Default backend: `MemoryStorage` (in-process). `LocalStorage` and `S3Storage` are also available.

### `StorageFacade` methods

| Method | Return type | Notes |
|--------|-------------|-------|
| `Storage::put(path, bytes)` | `Result<(), String>` | Write bytes to path |
| `Storage::get(path)` | `Result<Vec<u8>, String>` | Read bytes |
| `Storage::delete(path)` | `Result<(), String>` | Remove file |
| `Storage::exists(path)` | `bool` | Check presence |
| `Storage::url(path)` | `Result<String, String>` | Public URL for the path |

### Other backends

```rust
use rf_storage::{LocalStorage, Storage};

// Local filesystem (path-traversal-safe).
let store = LocalStorage::new(&root_dir, "http://localhost:3000").await?;
store.put("uploads/photo.jpg", bytes).await?;
let data = store.get("uploads/photo.jpg").await?;
```

For S3, enable the `s3` feature on `rf-storage` and use `rf_storage::S3Storage` with
`rf_storage::S3Config`.

---

## 10. Queue and Jobs

**Crate:** `rf-queue` (stable) — `crates/rf-queue/src/`

### `Job` trait

```rust
use async_trait::async_trait;
use rf_queue::{Job, QueueError};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
struct MyJob { payload: String }

#[async_trait]
impl Job for MyJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // actual work
        Ok(())
    }
    fn job_type(&self) -> &'static str { "my_job" }
    // Optional overrides:
    // fn max_attempts(&self) -> u32 { 3 }
    // fn backoff(&self) -> Duration { Duration::from_secs(30) }
}
```

### Dispatch

| Method | Return type | Notes |
|--------|-------------|-------|
| `Jobs::set_queue(queue)` | `()` | Install global queue (call once at startup) |
| `Jobs::dispatch(job)` | `Result<String, QueueError>` | Dispatch via global queue (no handle needed) |
| `Jobs::dispatch_later(job, delay)` | `Result<String, QueueError>` | Delayed dispatch |
| `job.dispatch_now()` | `Result<String, QueueError>` | Dispatch via global queue from the job itself |

### Worker (in-process draining)

```rust
let worker = Worker::new(queue)
    .register::<JobTypeA>()
    .register::<JobTypeB>();

// Drain all pending jobs (returns false when empty).
while worker.work_once().await? {}

// Continuous background draining (blocks until error).
worker.start().await?;
```

### Types

| Type | Location | Description |
|------|----------|-------------|
| `MemoryQueue` | `rf_queue::MemoryQueue` | In-process FIFO queue; priority, retry, DLQ |
| `Worker` | `rf_queue::Worker` | Drains a `Queue`, dispatches to registered job types |
| `Jobs` | `rf_queue::Jobs` | Process-global dispatch facade |
| `Queue` (trait) | `rf_queue::Queue` | Driver trait: `push`, `pop`, `failed` |
| `QueueError` | `rf_queue::QueueError` | Queue error type |

For Redis-backed production workers, use `rf-jobs` (stable, requires live Redis).
See `examples/jobs-offline/` for the no-Redis path and `examples/jobs-demo/` for Redis.

---

## 11. Events

**Crate:** `rf-events` (stable) — `crates/rf-events/src/`

Re-exported as `Event` in the prelude (`rf_events::EventFacade`).

### `EventFacade` methods

| Method | Signature | Notes |
|--------|-----------|-------|
| `Event::listen(name, f)` | `fn listen<F: Fn(&Value) + Send + Sync + 'static>(name: &str, f: F)` | Register a sync listener |
| `Event::dispatch(name, data)` | `fn dispatch<T: Serialize>(name: &str, data: T) -> Result<(), String>` | Fire event; listener panics are isolated |
| `Event::has_listeners(name)` | `fn has_listeners(name: &str) -> bool` | |
| `Event::listener_count(name)` | `fn listener_count(name: &str) -> usize` | |
| `Event::forget(name)` | `fn forget(name: &str)` | Remove all listeners for an event |
| `Event::forget_all()` | `fn forget_all()` | Remove all listeners |

For async, type-keyed dispatch use `rf_events::EventDispatcher` (also stable, see
`crates/rf-events/src/lib.rs`).

---

## 12. Error Handling

**Crate:** `rf-core` (stable) — `crates/rf-core/src/`

### Types

| Type | Location | Description |
|------|----------|-------------|
| `AppError` | `rf_core::AppError` | Framework error; implements `axum::IntoResponse` (RFC 7807 JSON) |
| `AppResult<T>` | `rf_core::AppResult` | `Result<T, AppError>` — handler return type for `?`-propagation |
| `OrNotFound` (trait) | `rf_core::OrNotFound` | `.or_404()` on `Option<T>` → `Result<T, AppError>` |
| `RfResult<T>` | prelude alias | `rf_errors::Result<T>` — named `RfResult` to avoid shadowing `std::result::Result` |
| `RustForgeError` | `rf_errors::RustForgeError` | Rich multi-variant error type with structured context |

### `AppError` variants and HTTP status codes

| Variant | HTTP status |
|---------|-------------|
| `AppError::NotFound { resource }` | 404 |
| `AppError::Unauthorized` | 401 |
| `AppError::Forbidden { reason }` | 403 |
| `AppError::BadRequest { message }` | 400 |
| `AppError::Validation(_)` | 422 |
| `AppError::Conflict { message }` | 409 |
| `AppError::RateLimitExceeded` | 429 |
| `AppError::ServiceUnavailable { service }` | 503 |
| `AppError::Internal(_)` | 500 |

### `?`-based handler pattern

```rust
use rf::prelude::*;

async fn show_post(axum::extract::Path(id): axum::extract::Path<i64>)
    -> rf_core::AppResult<impl axum::response::IntoResponse>
{
    let post = find!(Post, id)?
        .or_404()?;  // None -> AppError::NotFound -> 404 JSON
    Ok(json(post))
}
```

---

## 13. Helpers

**Crates:** `rf-global-helpers` (stable), `rf-collections` (beta)

### Password and CSRF

| Item | Signature | Notes |
|------|-----------|-------|
| `Hash::make(pw)` | `fn make(pw: &str) -> String` | BCrypt hash |
| `Hash::check(pw, hash)` | `fn check(pw: &str, hash: &str) -> bool` | BCrypt verify |
| `csrf_token()` | `fn csrf_token() -> String` | UUID v4 token |
| `csrf_field()` | `fn csrf_field() -> String` | HTML `<input type="hidden" ...>` string |

### Navigation

| Item | Signature | Notes |
|------|-----------|-------|
| `redirect(path)` | `fn redirect(path: &str) -> RedirectResponse` | |
| `back()` | `fn back() -> RedirectResponse` | Redirect to Referer or `/` |

### Collection

`rf_collections::Collection<T>` — Laravel-style chainable collection:

```rust
use rf::prelude::*;

let names: Vec<String> = collect(users.iter())
    .map(|u| u.name.clone())
    .filter(|n| !n.is_empty())
    .pluck("name")
    .paginate(10, 1)
    .to_vec();
```

Key methods: `map`, `filter`, `reduce`, `chunk`, `pluck`, `first`, `last`, `paginate`, `sort_by`,
`unique`, `flatten`, `to_vec`.

---

## 14. Configuration

**Crate:** `rf-config` (stable) — `crates/rf-config/src/`

| Item | Signature | Notes |
|------|-----------|-------|
| `Config::get(key)` | `fn get(key: &str) -> Option<String>` | Dot-path access |
| `Config::set(key, val)` | `fn set(key: &str, val: &str)` | |
| `Config::get_or(key, default)` | `fn get_or(key: &str, default: &str) -> String` | |
| `AppConfig::from_env()` | `fn from_env() -> AppConfig` | Load from `config/*.toml` + env vars (dotenvy) |

### Environment variables (reference)

| Variable | Default | Effect |
|----------|---------|--------|
| `DATABASE_URL` | (absent = in-memory SQLite) | Backend selection |
| `JWT_SECRET` | built-in dev secret | JWT signing key (>= 32 chars in prod) |
| `SMTP_HOST` | (absent = FileMailer) | Enable SMTP mail transport |
| `SMTP_PORT` | `587` | SMTP port |
| `SMTP_USER` / `SMTP_PASS` | (none) | SMTP credentials |
| `MAIL_MAILBOX` | `/tmp/rustforge-mailbox` | FileMailer output directory |
| `REDIS_URL` | (absent = MemoryCache) | Redis cache backend |
| `PORT` | `3000` | HTTP listen port |

---

## 15. Maturity and stability boundaries

RustForge uses four tiers for its 127 crates. See [docs/TIERS.md](../TIERS.md) for the full roster.

| Tier | SemVer | Examples |
|------|--------|---------|
| **stable** | Covered from v1.0 | `rf`, `rf-routing`, `rf-orm`, `rf-auth`, `rf-cache`, `rf-mail`, `rf-storage`, `rf-queue`, `rf-events`, `rf-validation`, `rf-core` |
| **beta** | Best-effort; API may shift in minor versions | `rf-graphql`, `rf-socialite`, `rf-pagination`, `rf-blade`, `rf-passport` |
| **experimental** | No SemVer guarantee; excluded from default build | `rf-nova`, `rf-swagger`, `rf-telescope`, `rf-cms`, `rf-breeze`, `rf-vite`, `rf-livereload` |
| **stub** | Superseded facade directories; unmaintained | `rf-auth-facade`, `rf-cache-facade`, `rf-db-facade`, … |

**Items not in the v1 contract:** `rf-nova` / `rf-swagger` / `rf-telescope` (experimental admin /
OpenAPI / debugging panels), `rf-breeze` / `rf-vite` / `rf-livereload` (experimental dev tooling),
`rf-socialite` / `rf-ai` / `rf-vector` (alpha — subject to removal or rename).

---

## 16. Explicit-core / escape-hatch

Every DX convenience is sugar over an ambient-state-free core. For library code, CLI tools,
background tasks, or any handler where you want compile-time proof of field presence:

```rust
use rf_request::extractors::RequestExtractor;

async fn create(RequestExtractor(req): RequestExtractor)
    -> axum::response::Result<impl axum::response::IntoResponse>
{
    let title: String = req.require("title")?;  // 400 if absent
    let body:  String = req.require("body")?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}
```

Or use any axum extractor directly (`Json<T>`, `Query<T>`, `Path<T>`, `Form<T>`,
`ValidatedJson<T>`) — these work with or without `capture_request` in the stack and satisfy
Rust's compile-time guarantees. See [API_PHILOSOPHY.md](../API_PHILOSOPHY.md) for the full
two-layer framing.

---

## See also

- [Laravel-Syntax.md](Laravel-Syntax.md) — the DX reference with annotated examples
- [docs/STABLE_CORE.md](../STABLE_CORE.md) — the v1 API contract (grep-verified entry points)
- [docs/API_PHILOSOPHY.md](../API_PHILOSOPHY.md) — two-layer framing and honest trade-offs
- [docs/TIERS.md](../TIERS.md) — maturity tiers for all crates
- [docs/COOKBOOK.md](../COOKBOOK.md) — CI-verified task-oriented recipes
- `examples/reference-app/` — flagship app exercising all stable capabilities end-to-end
