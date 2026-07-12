# RustForge — Architecture

> **Status:** Current. Reflects the verified state after 8 production-loop rounds and
> the v1.0.0 consolidation (see `VISION_GAP.md` for the full per-area audit and the
> README for the graded maturity matrix). The previous file at this path was an
> outdated v0.2.0 German-language "Foundry Core" blueprint; it is superseded by this
> document.

---

## Four pillars

| # | Pillar | What it means in practice |
|---|---|---|
| 1 | **Write less** | `Model!` declares the struct, its DB table mapping, CRUD macros, typed reads, relation loaders, and companion DTOs in one declaration — less ceremony than equivalent raw SeaORM + axum handler code |
| 2 | **Hide async** | Argument-less handlers read the current request through ambient globals (`input()`, `file()`, `has()`) set by the `capture_request` middleware; `.await` is hidden inside framework macros for common operations via `#[auto_await]` |
| 3 | **Global facades** | `Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`, `Broadcast` are callable as static methods from anywhere in a request cycle, backed by real engines over a deadlock-safe `AsyncBridge` (no `block_on` panic inside an axum handler) |
| 4 | **Typed DSL** | `validate! { title: string.max(200), email: email }` and the `Model!` `@` field DSL (`@ email message("...")`, `@ min(3) max(20) alphanumeric`) build real `rf_validation` rules at compile time |

---

## Crate map

### HTTP stack (axum 0.8)

| Crate | Role |
|---|---|
| `rf-routing` | Route registration via `get()`/`post()`/`put()`/`patch()`/`delete()` free fns; `GlobalRouter` accumulates them; `build_router()` assembles a real axum `Router`. Default JSON 404/405 handlers. |
| `rf-request` | `capture_request` middleware (buffers body, sets `tokio::task_local!`); `input()`/`file()`/`has()`/`all()`/`capture_path_params` globals. |
| `rf-response` | `json()`/`redirect()`/`download()` free helpers; `ResponseBuilder`; axum `IntoResponse` wrappers. `AppError` renders 400/401/403/404/422/500 with a JSON envelope. |
| `rf-web` | CORS middleware (`rf_web::cors`); session middleware + `SessionFacade` (cookie session-id, flash); `SecurityHeadersLayer` (HSTS/CSP/X-Frame-Options/Referrer-Policy); RFC 7807 error responses; `RouterBuilder`. |

### ORM and persistence

| Crate | Role |
|---|---|
| `rf-orm` | SeaORM-based ORM. Laravel-style query builder (`all`, `find`, `where_eq`, `where_like`, `paginate`, scopes, typed reads `all_typed()`). Eager-load builder (`with(&["relation"]).get()`). Real SQLite/Postgres/MySQL. |
| `rf-eloquent` | Relation accessor builders (`HasManyBuilder`, `BelongsToBuilder`, pivot/through helpers), eager-loading optimizer, query helpers. |
| `rf-macros` | Procedural macros: `Model!` (model struct + DB-table mapping + companion DTOs + `@` validation DSL), `create!`/`find!`/`update!`/`delete!` CRUD macros, `validate!` typed DSL, `routes!`, `#[auto_await]`/`#[await_calls]`, `#[controller]`, `rustforge!{}`, and more. |
| `rf-model-macro` | `#[model]` attribute derive: injects SeaORM `DeriveEntityModel`, table-name pluralization, auto-timestamps. |
| `rf-validation-derive` | `#[derive(Validate)]` + `#[validate(required, email, max=255, unique="table")]` attributes → real `rf_validation` rule impls, including `DbUniqueRule`/`DbExistsRule` (live DB COUNT). |
| `rf-soft-deletes` | Soft-delete trait and scopes (`deleted_at` column logic). |
| `rf-search` | Search query helpers. |

### Validation

| Crate | Role |
|---|---|
| `rf-validation` | 48+ validation rules (string, numeric, date, array, database, conditional). Async `Validator` runner. `ValidatedJson<T>` axum 0.8 extractor (deserialize + validate in one step; 422 on failure). |
| `rf-validation-derive` | (also listed under ORM — `#[derive(Validate)]` derive macro) |

### Auth, sanctum, authorization

| Crate | Role |
|---|---|
| `rf-auth` | `Auth` facade: `Auth::attempt()`, `Auth::user::<T>()`, `Auth::check()`, `Auth::id()`. Per-request state via `tokio::task_local!` (no cross-request bleed). JWT `get_claims` extractor. `require_auth` middleware (401 before body extractors fire). |
| `rf-sanctum` | Personal access token (PAT) auth: SHA-256 hashing, per-token abilities/scopes, expiration, device tracking. DB-free transient token mode. |
| `rf-authorization` | RBAC gates and policies. |
| `rf-encryption` | AES-256-GCM encrypted casts. |
| `rf-audit` | Audit log trail. |

### Mail, notifications, and queue

| Crate | Role |
|---|---|
| `rf-mail` | `Mail` facade: `Mail::to(addr).subject(s).body(b).send()`. Real backends: SMTP (lettre), FileMailer, Log, Memory/Mock. Bridges to async driver via `rf-async-bridge`. `MailQueue` for deferred delivery. |
| `rf-notifications` | Multi-channel notifier: email, database channels. `Notifier` attempts all channels and returns an aggregate report; no abort on first channel failure. |
| `rf-queue` | In-process `MemoryQueue` + `Worker`: priority FIFO, retry, dead-letter (`Queue::failed()`), panic isolation (`catch_unwind`). Handle-free `Jobs` facade over `rf-async-bridge`. `#[derive(Job)]` static `dispatch`. No Redis required. |
| `rf-jobs` | Redis-backed `WorkerPool`; batch, chain, routing, DLQ, scheduler. Requires live Redis. |

### WebSocket and broadcast

| Crate | Role |
|---|---|
| `rf-broadcast` | `Broadcast` facade; `RoomRegistry` (room isolation, `Subscribed` ack, `Lagged` skip-and-continue on buffer overflow); process-global callable from background jobs. |
| `rf-sse` | Server-Sent Events helpers. |

### Cache and storage

| Crate | Role |
|---|---|
| `rf-cache` | `Cache` facade: `Cache::get`/`put`/`remember`. Memory backend (default) + optional Redis. `RedisPubSub` for inter-service pub/sub events. Backed by `rf-async-bridge`. |
| `rf-storage` | `Storage` facade: `Storage::put`/`get`/`delete`/`url`. Local-filesystem + S3 backends. Path-traversal-safe. `BridgedStorage` via `rf-async-bridge`. |
| `rf-upload` | `FileUpload` from multipart; MIME and size validation; `file("field")` request helper; 413 on oversize upload. |
| `rf-async-bridge` | **Shared, deadlock-safe sync-over-async bridge.** One dedicated OS thread with its own current-thread Tokio runtime; sync callers submit via channel and block on a reply channel. Used by `rf-cache`, `rf-mail`, `rf-storage`. See [AsyncBridge pattern](#the-asyncbridge-pattern) below. |

### Ops (observability, rate limiting, health)

| Crate | Role |
|---|---|
| `rf-observability` | OpenTelemetry tracing integration; unified Prometheus registry with HTTP request timings; real trace/span IDs in log correlation. |
| `rf-health` | `HealthChecker`: DB, Redis, and custom probes. Fail-closed by default; `Degraded` status can return 503. |
| `rf-metrics` | Prometheus-compatible HTTP metrics middleware layer; scheduler task execution metrics. |
| `rf-ratelimit` | `RateLimitLayer`: per-client IP default key; JSON 429 response; non-destructive `info()` peek. |

### Scaffolding and CLI

| Crate | Role |
|---|---|
| `rf-scaffold` | `make:model`, `make:controller`, `make:request`, `make:migration` code generators; produces compiling, warning-clean stubs using real `Model!` DSL and axum 0.8 route patterns. |
| `foundry-cli` / `forge-cli` | CLI entry points: `make:*` commands, `forge deploy generate`, migration runner, seeder, scaffold. |
| `rf-cli-gen` | Internal code-generation primitives used by the scaffolding layer. |

### Extended ecosystem crates

| Crate | Role | Maturity |
|---|---|---|
| `rf-i18n` | `AcceptLanguage` axum extractor (header + `?locale=`); CLDR plural rules (Slavic/Arabic); Handlebars rendering; `Arc`-shareable. | Stable |
| `rf-tenancy` | Multi-tenancy: `Tenant::current()`, axum 0.8 Layer, `spawn_with_tenant()` for background tasks; missing-header → 400. | Usable |
| `rf-inertia` | Inertia.js protocol: X-Inertia-Location 409, browser→HTML/XHR→JSON finalization, SharedProps wiring. | Usable |
| `rf-graphql` | Per-request auth context injected so `AuthGuard`/`RoleGuard` are reachable via async-graphql. | Usable |
| `rf-swagger` | Thin utoipa integration: real spec serving via `swagger_ui`/`redoc`. Routes must be annotated; not an auto-scanner. | Usable |
| `rf-nova` | Admin UI: single-resource CRUD over the real DB. Multi-resource type-erased dispatch is Experimental. | Usable |
| `rf-api-resources` | `WrappedResource`/`WrappedCollection` transform layer; manual `Serialize` (no silent wrapper drop). | Usable |
| `rf-deploy` / `rf-config` / `rf-env` | DockerCompose serialization; `AppConfig::from_env`; dotenvy-backed `.env` loading. | Usable |
| `rf-ai` | Anthropic provider (real reqwest POST to `/v1/messages`), Agent tool loop, AI attachments (`ContentBlock::Image`/`Document`, base64 `file()` helper), `.prompt().attachment().run()` builder. | Experimental |
| `rf-blade` | Blade template engine (`@for` and other directives). | Experimental |
| `rf-view` / `rf-views` | Tera-based view rendering. Two variants; `rf-views` renders synchronously. | Usable |
| `rf-mcp` | MCP server binding for LLM-agent integration. | Experimental |

Crates in the workspace that are not yet production-ready: `rf-breeze`, `rf-cashier`,
`rf-telescope`, `rf-vite`, `rf-livereload`, `rf-spark`. Stubs exist; do not claim
production readiness for these.

---

## Request flow

A typical HTTP request through a fully-wired RustForge application follows this path:

```
HTTP request arrives
        │
        ▼
axum Router  (assembled by rf-routing global_router().build_router())
  — routes registered via get()/post()/put()/patch()/delete() free fns
        │
        ▼
Tower middleware stack  (applied outside-in; innermost runs first for the request)
  1. capture_request   (rf-request)   — buffers body bytes once; parses JSON / form /
                                        query / path params into a tokio::task_local!;
                                        re-inserts bytes so downstream extractors work
  2. session           (rf-web)       — optional: load/persist session cookie + flash
  3. tenant            (rf-tenancy)   — optional: identify tenant; set Tenant::current()
  4. require_auth      (rf-auth)      — optional: validate bearer/PAT; 401 fires before
                                        any body extractor (no 422 leakage on auth fail)
        │
        ▼
argument-less handler  (async fn handler() -> impl IntoResponse)
  — reads request data via: input("field")  file("field")  has("field")
  — runs validation:        validate! { title: string.max(200), body: string }
  — persists via macros:    create!(Post, title=t)  Post::all().await  Post::find(id).await
  — reads facades:          Auth::user()  Cache::get("k")?  Storage::put(path, bytes)?
  — sends mail:             Mail::to(addr).subject(s).body(b).send()?
  — builds response:        json(data)  redirect("/path")  AppError (→ JSON envelope)
        │
        ▼
Response
```

The `capture_request` middleware is the key enabler of argument-less handlers. It:
- Reads the body **once** (async) and stores it in a `tokio::task_local!`.
- Re-inserts the bytes as a new body so that downstream axum extractors (`Json<T>`,
  `ValidatedJson<T>`) still have a body to consume on the same router.
- Parses query params, path captures, and the buffered body into typed globals so
  `input("field")` and `file("field")` can be called without a `Request` parameter.

---

## The AsyncBridge pattern

Several facades expose a **synchronous** API (`Cache::get`, `Mail::to(..).send()`,
`Storage::put`) while their real drivers are async (Redis, lettre SMTP, S3). Bridging
sync → async naively with `Handle::current().block_on(...)` **panics** inside an
existing Tokio runtime ("Cannot start a runtime from within a runtime").

`rf-async-bridge` (`crates/rf-async-bridge/src/lib.rs`) solves this with one
dedicated OS thread running its own current-thread Tokio runtime. Sync callers submit
an async job over a channel and block on a reply channel. Because the future runs on a
**separate thread** with a **separate runtime**, blocking the caller thread never
blocks the Tokio executor making progress on the rest of the application.

```
Sync caller (any thread — including an axum handler)
        │  mpsc channel: sends  async job closure
        ▼
AsyncBridge thread ──── owns one long-lived current-thread Tokio runtime
        │  drives the future to completion
        │  oneshot channel: sends  Result<T>
        ▼
Sync caller unblocks, receives Result<T>
```

`rf-cache`, `rf-mail`, and `rf-storage` each re-export `AsyncBridge` from
`rf-async-bridge` and build their own `BridgedCache`/`BridgedMailer`/`BridgedStorage`
wrappers on top. The three bridge files (`rf-cache/src/bridge.rs`,
`rf-mail/src/bridge.rs`, `rf-storage/src/bridge.rs`) are thin re-exports — no
implementation is duplicated.

---

## Design ceilings (honest limits)

These are Rust language boundaries, not planned work items. They are documented rather
than worked around.

| Ceiling | What it means | Why it is a ceiling |
|---|---|---|
| `Result`/`Option` visible | `.await` is hidden by `#[auto_await]`; `?`/`Result`/`Option` stay | Auto-injecting `?` requires one uniform error type across heterogeneous facades; impossible without a lossy `Box<dyn Error>` collapse and unacceptable error-span confusion |
| `::` not `.` | Write `Post::all()`, `Mail::to(...)`, `Cache::put(...)` — not `Post.all()` | Rust has no dot-on-type syntax; static calls must use `::` |
| Eager-only bare relation fields | `post.user` is only valid for an eagerly-hydrated `Option<User>` field populated by `with().get()` | Lazy field reads cannot be async or fallible in Rust; `post.user` for lazy loading is simply not expressible |
| Ambient globals are request-scoped | `input()`/`file()`/`has()` return empty/`None` outside a handler | `tokio::task_local!` is not set until `capture_request` fires; a runtime condition, not a compile error |
| `class`/`controller` as top-level keywords | These are not valid Rust items at module level | Rust's grammar admits only fixed item keywords; macro-wrapped forms (function-like proc-macros) lose IDE goto-def inside the token body |

---

## Relationship to other architecture documents

| Document | Content | Status |
|---|---|---|
| `VISION_GAP.md` (repo root) | Full per-area audit (16 areas); Re-audit 1 and Re-audit 2 scoring; gap analysis; remaining roadmap. | **Primary grounding document.** Read alongside this file. |
| `README.md` (repo root) | North-star; quickstart; graded maturity matrix. | Current and accurate. |
| `docs/adr/` | Architecture Decision Records for key framework choices. | Current; see table below. |
| `docs/architecture/roadmap.md` | Deleted (cycle-3 cleanup): used "Foundry" era crate names throughout; superseded by `VISION_GAP.md`. | Removed — see tombstone in `docs/README.md`. |

### ADR summary

| ADR | Decision | Current status |
|---|---|---|
| 001 | Axum + Tower as web framework | Active — axum 0.8 throughout |
| 002 | RFC 7807 error responses | Active — `AppError`/`AppResult` JSON envelope |
| 003 | Dependency injection via service container | Active — `rf-container`; also `OnceCell`/`Lazy` process-globals |
| 004 | OpenTelemetry + Prometheus observability | Active — `rf-observability`/`rf-metrics` |
| 005 | Configuration via environment variables | Active — `rf-env`/`rf-config` (dotenvy) |
| 006 | SeaORM as ORM choice | Active — `rf-orm` and `rf-eloquent` build on SeaORM/rusqlite |
| 007 | Redis-backed job queue | Active — `rf-jobs` (Redis); `rf-queue` `MemoryQueue` is the in-process alternative |

No ADR contradicts the current implementation.

---

_For graded maturity on each surface, see the [README maturity matrix](../README.md#feature-maturity-matrix).
For per-area engine analysis, gaps, and the remaining roadmap, see [`VISION_GAP.md`](../VISION_GAP.md).
For task-oriented recipes verified against the source, see [`docs/COOKBOOK.md`](./COOKBOOK.md)._
