# Examples

A curated tour of the real example applications under `examples/`. Every
directory listed here was verified to exist on disk at the time of writing.
Each example is runnable with `cargo run -p <name>` (some require environment
variables such as `DATABASE_URL` or `REDIS_URL`).

For API details, see `docs/STABLE_CORE.md`. For maturity of specific crates,
see `docs/TIERS.md`.

---

## Table of Contents

- [Flagship](#flagship)
  - [reference-app](#reference-app)
- [Core Primitives](#core-primitives)
  - [blog-slice](#blog-slice)
  - [rest-crud-resource](#rest-crud-resource)
  - [validated-signup](#validated-signup)
- [Authentication](#authentication)
  - [auth-demo](#auth-demo)
  - [auth-paginated-search](#auth-paginated-search)
- [Real-World App](#real-world-app)
  - [taskflow](#taskflow)
- [Facades](#facades)
  - [facades-demo](#facades-demo)
- [Queues and Jobs](#queues-and-jobs)
  - [jobs-offline](#jobs-offline)
  - [jobs-demo](#jobs-demo)
- [Mail](#mail)
  - [mail-demo](#mail-demo)
- [Internationalization](#internationalization)
  - [i18n-localized-api](#i18n-localized-api)
- [Real-Time](#real-time)
  - [realtime-chat](#realtime-chat)
- [Database — Explicit SeaORM](#database--explicit-seaorm)
  - [database-demo](#database-demo)
- [Getting Started](#getting-started)
  - [hello](#hello)
- [Syntax Showcase](#syntax-showcase)
  - [laravel-syntax-simple](#laravel-syntax-simple)
  - [laravel-syntax-complete](#laravel-syntax-complete)
  - [macros-demo](#macros-demo)
- [Tools and Scaffolding](#tools-and-scaffolding)
  - [scaffold-demo](#scaffold-demo)
- [Frontend Patterns](#frontend-patterns)
  - [htmx-livewire-alternative](#htmx-livewire-alternative)
  - [phase12-blog](#phase12-blog)
- [README-Only References](#readme-only-references)

---

## Flagship

### reference-app

**Path:** `examples/reference-app/`

The canonical end-to-end application that exercises every stable-core surface
in a single runnable binary: JWT auth (register + login + `GET /me`), full
CRUD with `Model!`/`create!`/`find!`/`update!`/`delete!`, inline migrations via
`DB::statement()`, `validate!` DSL, `Cache` facade (MemoryCache default),
`Storage` facade (MemoryStorage default), `MemoryQueue` + `Worker` for
background jobs, `Mail` facade (FileMailer default), health check (`GET /health`
via `rf-health`), and Prometheus metrics (`GET /metrics` via `rf-metrics`).

**Surfaces:** routing, request globals, validation, ORM, auth/JWT, cache, storage,
queue, mail, health, metrics, logging.

**Backends:** defaults to in-memory SQLite, MemoryCache, MemoryStorage, and FileMailer
— runs with zero external services. Switch backends by setting environment variables:
`DATABASE_URL=postgres://...` for Postgres, `SMTP_HOST=...` for real SMTP.

```bash
cargo run -p reference-app
# or with Postgres:
DATABASE_URL=postgres://user:pass@localhost/db cargo run -p reference-app
```

```
POST   /auth/register   {"email":"x@example.com","password":"secret123"}
POST   /auth/login      {"email":"x@example.com","password":"secret123"}
GET    /me              Authorization: Bearer <token>
GET    /posts
POST   /posts           Authorization: Bearer <token>
GET    /posts/{id}
PUT    /posts/{id}      Authorization: Bearer <token>
DELETE /posts/{id}      Authorization: Bearer <token>
POST   /upload          multipart/form-data, field "file"
GET    /health
GET    /metrics
```

---

## Core Primitives

### blog-slice

**Path:** `examples/blog-slice/`

The minimal vertical slice: HTTP request → `validate!` → `Model!`/`create!` → `json()`
response, written with only high-level primitives and no plumbing leaking into
handlers. Demonstrates the `capture_request` middleware enabling argument-less
handlers that call `input("field")` directly.

**Surfaces:** routing (`get`/`post`), request globals (`input`), `validate!` DSL,
`Model!`/`create!`/`find!` ORM macros, `json()` response helper.

```bash
cargo run -p blog-slice
# POST /posts  {"title":"Hello","body":"World"}
# GET  /posts
# GET  /posts/{id}
```

---

### rest-crud-resource

**Path:** `examples/rest-crud-resource/`

The canonical five-verb REST CRUD lifecycle for an `Article` resource with an
`Author` relation, eager-loaded via `Article::with(&["author"]).get()` (one
batched loader, no N+1). Shows correct REST status codes (201/200/204/404/422)
from argument-less handlers reading input via globals.

**Surfaces:** routing, `capture_request`, `validate!`, `Model!` with `belongsTo`
relation, `create!`/`find!`/`update!`/`delete!`, eager loading, `json()`.

```bash
cargo run -p rest-crud-resource
# POST   /articles        {"title":"...","body":"...","author_id":1}
# GET    /articles
# GET    /articles/{id}
# PUT    /articles/{id}   {"title":"...","body":"...","author_id":1}
# DELETE /articles/{id}
```

---

### validated-signup

**Path:** `examples/validated-signup/`

Demonstrates the `ValidatedJson<T>` extractor path: the `Model!` macro's `@`
rule DSL attaches per-field validation constraints (`@ email`, `@ min(N)`,
`@ alphanumeric`, `@ regex("...")`, `@ message("custom")`) that generate a real
`Validate` impl. Deserialization and validation both run inside the extractor —
there is no manual `validate!()` call in the handler body. Invalid input returns
`422 Unprocessable Entity` with structured per-field errors; valid input returns
`201 Created`.

**Surfaces:** `Model!` with `@ rule` DSL, `ValidatedJson<T>`, `rf-validation-derive`,
`create!`, `json()`, `rf::post` + `global_router()`.

```bash
cargo run -p validated-signup
# POST /signup  {"username":"ada42","email":"ada@example.com","password":"hunter2!","zipcode":"12345"}
```

---

## Authentication

### auth-demo

**Path:** `examples/auth-demo/`

A standalone auth demonstration covering password hashing with bcrypt, JWT
token generation and validation, user registration and login flows, protected
routes, and role-based access control using `require_role`. Uses an in-memory
user store (not a database) to keep the example self-contained.

**Surfaces:** `rf-auth` (`JwtManager`, `PasswordHasher`, `Claims`, `require_auth`,
`require_role`), `axum::Router`, structured JSON responses.

```bash
cargo run -p auth-demo
# POST /register  {"email":"...","name":"...","password":"..."}
# POST /login     {"email":"...","password":"..."}
# GET  /profile   Authorization: Bearer <token>
# GET  /admin     Authorization: Bearer <token with admin role>
```

---

### auth-paginated-search

**Path:** `examples/auth-paginated-search/`

Ties together per-request auth scope isolation (`with_auth_scope`), the `Auth`
facade (`Auth::check()`, `Auth::id()`), request globals (`input("q")`,
`input("page")`), and `QueryBuilder` search + pagination
(`DB::table("posts").where_eq("user_id", ...).where_like("title", ...)
.paginate(per_page, page)`). The handler returns `401` for unauthenticated
callers and `200` with the filtered paginated result for authenticated ones.
Documents an honest limitation: the current `QueryBuilder` renders OR without
parentheses, so the search is scoped to `title` only to keep user isolation
correct.

**Surfaces:** `rf-auth` (`with_auth_scope`, `Auth`), request globals, `DB`
`QueryBuilder` (search + pagination), `json()`.

---

## Real-World App

### taskflow

**Path:** `examples/taskflow/`

A small project/task manager that serves as a permanent regression guard for
six framework edges that were once broken and are now fixed. Specifically: (1)
bidirectional `hasMany`/`belongsTo` relations that previously caused an opaque-
type inference cycle (`E0391`), now resolved by generating concrete boxed futures;
(2) a `belongsTo` relation with a `foreign_key` override
(`Task belongsTo assignee: User (foreign_key = "user_id")`); (3) a single
`build_router` where argument-less `capture_request`-global handlers and
`ValidatedJson<T>` body extractors coexist; and three additional edges. The
`cargo test -p taskflow` suite guards all six regressions.

**Surfaces:** `Model!` with bidirectional relations and `foreign_key` override,
`ValidatedJson<T>`, request globals, `create!`/`find!`/`update!`/`delete!`,
`require_auth`, `json()`.

---

## Facades

### facades-demo

**Path:** `examples/facades-demo/`

A non-HTTP binary that exercises every Laravel-style facade in sequence —
`Auth`, `Cache`, `Storage`, `Mail`, `Event` — demonstrating that all facades
are synchronous (no `.await` needed anywhere). Uses the default in-process
backends (MemoryCache, MemoryStorage, etc.) so there is nothing to install.

**Surfaces:** `rf-facades` (`Auth`, `Cache`, `Storage`, `Mail`, `Event`),
`rf::prelude::*`, in-process backends.

```bash
cargo run -p facades-demo
```

---

## Queues and Jobs

### jobs-offline

**Path:** `examples/jobs-offline/`

The batteries-included, no-Redis job queue path: dispatches a real job onto
`MemoryQueue`, drains it with a `Worker`, and exits — nothing to install. This
is the recommended starting point for apps that do not need Redis. Shows the
`rf-queue` API: `Jobs::set_queue(queue)`, `job.dispatch_now()`, `Worker::new(queue).run()`.

**Surfaces:** `rf-queue` (`MemoryQueue`, `Jobs`, `Worker`, `Job` trait),
`async_trait`, no external services.

```bash
cargo run -p jobs-offline
```

---

### jobs-demo

**Path:** `examples/jobs-demo/`

The Redis-backed job queue path using `rf-jobs`: defines multiple job types
(`SendEmailJob`, etc.), shows `BackoffStrategy`, `JobRegistry`, `WorkerConfig`,
`WorkerPool`, `Scheduler`, and the `dispatch` free function. Requires a live
Redis instance; exits with a clear error message if Redis is unreachable.

**Surfaces:** `rf-jobs` (`QueueManager`, `WorkerPool`, `Job` trait, `dispatch`,
`BackoffStrategy`, `Scheduler`), Redis.

```bash
REDIS_URL=redis://127.0.0.1:6379 cargo run -p jobs-demo
```

---

## Mail

### mail-demo

**Path:** `examples/mail-demo/`

A standalone demonstration of `rf-mail`: basic email sending, `Mailable` trait
usage, template rendering, attachments, and switching between `MemoryMailer`
(captures emails in RAM for inspection) and `SmtpMailer`. No HTTP server —
runs as a binary that sends demo emails and prints a summary.

**Surfaces:** `rf-mail` (`MailBuilder`, `Mailable`, `MemoryMailer`, `SmtpMailer`,
`Address`, attachments).

```bash
cargo run -p mail-demo
```

---

## Internationalization

### i18n-localized-api

**Path:** `examples/i18n-localized-api/`

Showcases `rf-i18n`: the `AcceptLanguage` axum extractor negotiates a locale
from (1) a `?locale=` query parameter, (2) the `Accept-Language` header, or
(3) falls back to `"en"`. A shared `Arc<I18n>` injected via `Extension` provides
per-request views via `I18n::for_locale()`. Demonstrates `I18n::t_plural()` with
CLDR plural rules for German and English (`one`/`other`, `zero`/`one`/`other`).

**Surfaces:** `rf-i18n` (`AcceptLanguage`, `I18n`, `I18n::t_plural()`), axum
`Extension`, `Arc`.

```bash
cargo run -p i18n-localized-api
curl -H 'Accept-Language: de' http://127.0.0.1:3009/greet
curl -H 'Accept-Language: fr' 'http://127.0.0.1:3009/items?count=5'
```

---

## Real-Time

### realtime-chat

**Path:** `examples/realtime-chat/`

A runnable WebSocket broadcasting example using `rf-broadcast`'s
`MemoryBroadcaster` and `websocket_router`. Clients connect to
`ws://127.0.0.1:3030/ws`, subscribe to a channel with a JSON frame, and receive
every event broadcast to that channel. A background task periodically broadcasts
a "chat message" to `room-1` so you can watch events flow with any WebSocket
client. The `#[tokio::test]` at the bottom binds a real port, connects three
`tokio-tungstenite` clients (two on `room-1`, one on `room-2`), fires a
broadcast, and asserts isolation.

**Surfaces:** `rf-broadcast` (`MemoryBroadcaster`, `Channel`, `websocket_router`,
`SimpleEvent`), axum `Router`, `tokio-tungstenite` (test).

```bash
cargo run -p realtime-chat
# Then: websocat ws://127.0.0.1:3030/ws
# Send: {"type":"subscribe","channel":"room-1"}
```

---

## Database — Explicit SeaORM

### database-demo

**Path:** `examples/database-demo/`

Shows the explicit SeaORM path below the `Model!` macro abstraction: defining
entities with `DeriveEntityModel`, `DeriveRelation`, `ActiveValue`, raw
`ConnectionTrait::execute` for DDL, and soft deletes. Useful when you need
SeaORM's full query API that the macro layer does not surface. Uses an in-memory
SQLite connection; no external services.

**Surfaces:** `rf-orm` (SeaORM entity API, `ActiveModel`, `ConnectionTrait`),
soft deletes, transactions, filtering, ordering.

```bash
cargo run -p database-demo
```

---

## Getting Started

### hello

**Path:** `examples/hello/`

The minimal "hello world" that shows the low-level integration points: `rf-core`
(`AppError`), `rf-web` (axum routing + middleware), `rf-config`
(`AppConfig::from_env`), and `rf-container` (dependency injection with
`ServiceRegistry`). No database, no ORM macros — useful as a starting point for
understanding how the layers wire together.

**Surfaces:** `rf-core`, `rf-web`, `rf-config`, `rf-container`.

```bash
cargo run -p hello
# GET  /           → "Hello, RustForge!"
# GET  /health
# POST /echo       → echoes the JSON body
```

---

## Syntax Showcase

### laravel-syntax-simple

**Path:** `examples/laravel-syntax-simple/`

A CLI binary (no HTTP server) demonstrating the framework's core helpers in
isolation: `Hash::make()` / `Hash::check()` (bcrypt), `csrf_token()`,
the `rules!` validation macro, and `Route` facade registration. Good for
understanding what these utilities do without the noise of a full server setup.

**Surfaces:** `rf::Hash`, `rf::csrf_token`, `rf::rules!`, `rf::Route`.

```bash
cargo run -p laravel-syntax-simple
```

---

### laravel-syntax-complete

**Path:** `examples/laravel-syntax-complete/`

A larger Laravel-style blog example demonstrating `Route::get/post/put/delete`,
the `rules!` pipe-syntax validation, `Hash::make()`, `csrf_token()`, and a real
`models/` + `database/` module structure. Shows how to organize a multi-file
project that uses the DX layer throughout.

**Surfaces:** `rf_global_helpers` (`Hash`, `csrf_token`, `__`), `rf::Route`,
`rules!`, multi-module project layout.

```bash
cargo run --bin blog -p laravel-syntax-complete
```

---

### macros-demo

**Path:** `examples/macros-demo/`

A showcase of the `rf-macros` crate's lower-level building blocks: the
`#[controller]` attribute macro, the `function!` macro for route handler
conversion, and the `rules!` validation macro. Demonstrates the macro API
surface without a running HTTP server. Most of the items it shows are used
internally by the higher-level DX layer.

**Surfaces:** `rf-macros` (`controller` attribute, `function!` macro, `rules!`),
`rf-request`, `rf-response`.

```bash
cargo run -p macros-demo
```

---

## Tools and Scaffolding

### scaffold-demo

**Path:** `examples/scaffold-demo/`

Exercises `rf-scaffold`'s `ScaffoldEngine` API: generating a model, controller,
migration, and request struct for a given entity definition — the same operations
that `forge make:model`/`make:controller` perform under the hood. Writes files to
a temporary directory and prints what was generated. Not an HTTP server; runs as
a binary.

**Surfaces:** `rf-scaffold` (`ScaffoldEngine`, `ModelOptions`).

```bash
cargo run -p scaffold-demo
```

---

## Frontend Patterns

### htmx-livewire-alternative

**Path:** `examples/htmx-livewire-alternative/`

Demonstrates Livewire-style patterns (interactive counter, form validation,
file upload, real-time polling, loading states, lazy loading) using htmx and
server-side HTML fragments from RustForge — no JavaScript framework required.
Uses `tower-sessions` for state and returns HTML fragments directly. A useful
reference for anyone preferring hypermedia over an SPA frontend.

**Surfaces:** axum `Router` with HTML fragment handlers, `tower-sessions`
(`Session`, `SessionManagerLayer`), `axum::extract::Multipart`, `axum::response::Html`.

```bash
cargo run -p htmx-livewire-alternative
# Open http://localhost:3000
```

---

### phase12-blog

**Path:** `examples/phase12-blog/`

A full-stack blog application that demonstrates `rf-blade` (template inheritance,
`@extends`/`@section`, `{{ }}` interpolation, `@if`/`@foreach`), `rf-vite`
(asset pipeline with HMR), `rf-livereload` (WebSocket-based live reload), and
`rf-cms` (media library and content storage).

**Maturity note:** `rf-blade` is **beta** tier. `rf-vite`, `rf-livereload`, and
`rf-cms` are **experimental** tier — excluded from the 1.0 stable surface, with
no SemVer guarantees. This example is a preview of those capabilities, not a
stable reference. See `docs/TIERS.md`.

**Surfaces:** `rf-blade`, `rf-vite` (experimental), `rf-livereload` (experimental),
`rf-cms` (experimental).

---

## README-Only References

The following directories contain a `README.md` describing intended features but
do not have a runnable `src/` implementation. They are concept references, not
working examples.

| Directory | Notes |
|-----------|-------|
| `examples/blog-complete/` | README describing a "production-ready" blog feature set |
| `examples/phase12-admin/` | README describing an admin panel using Phase 12 features |

Do not run `cargo run` on these; there is no binary to build.

---

## Running Any Example

```bash
# In the repo root, run by package name:
cargo run -p reference-app
cargo run -p blog-slice
cargo run -p rest-crud-resource
# etc.

# Run an example's tests:
cargo test -p taskflow
cargo test -p reference-app
```

Most examples bind to a port in the `3000–3030` range and print it on startup.
Set `PORT=<n>` to override where supported (reference-app respects `PORT`).

---

## Next Steps

- **[Migration Guide](Migration-Guide)** — Laravel concept → RustForge equivalent
- **[Quick Start](Quick-Start)** — build your first app
- **[API Documentation](API-Documentation)** — detailed API reference
- `docs/STABLE_CORE.md` — the definitive v1 API contract with entry points per capability
- `docs/COOKBOOK.md` — task-oriented recipes with CI-verified code snippets
