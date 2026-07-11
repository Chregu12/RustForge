# RustForge Example Gallery

All examples listed here are workspace members (`examples/` in root `Cargo.toml`) and compile
against the current framework.  Run any of them with:

```
cargo run -p <example-name>
```

Examples marked **tested** carry at least one `#[tokio::test]` that exercises the real
framework layers end-to-end and is part of `cargo test --workspace`.

---

## Core CRUD / REST

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `blog-slice` | Flagship vertical slice: `get`/`post` routing, `capture_request` middleware, `validate!` DSL, `Model!`/`create!` ORM macros, `DB`, SQLite-backed persistence. The minimal blueprint for a real RustForge API handler. | `cargo run -p blog-slice` (port 3000) | Yes |
| `rest-crud-resource` | Full 5-verb CRUD (GET/POST/PUT/DELETE): `Article belongsTo Author`, eager-loaded relation via `with(&["author"]).get()`, `find!`/`update!`/`delete!` ORM macros, 201/204/404/422 status codes. | `cargo run -p rest-crud-resource` (port 3001) | Yes |
| `taskflow` | Project/task manager: bidirectional relations (`Project hasMany tasks`, `Task belongsTo project`), FK override (`foreign_key = "user_id"`), `ValidatedJson<CreateX>` and `capture_request` handlers coexisting in one `build_router`. Regression guard for six fixed framework edges. | `cargo run -p taskflow` (port 3005) | Yes |
| `phase12-blog` | A blog originally written against raw axum, migrated to use only RustForge's high-level primitives (`get`/`post`, implicit-request globals, `validate!`, `Model!`/`create!`/`find!`, `json`/`view`). Shows how to migrate off raw axum. | `cargo run -p phase12-blog` (port 3004) | Yes |

---

## Validation

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `validated-signup` | `Model!` `@` DSL: `@ min(N) max(N) alphanumeric email regex("…") message("…")`. The generated `CreateUser` DTO implements the real `rf_validation::Validate` trait. `ValidatedJson<CreateUser>` auto-validates in the extractor — no manual `validate!()` call in the handler. | `cargo run -p validated-signup` (port 3002) | Yes |

---

## Auth

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `auth-paginated-search` | `rf_auth` per-request scope (`with_auth_scope`), `Auth::login_using_id` / `Auth::check` / `Auth::id`, bearer-token bridge, `QueryBuilder` search + pagination (`where_eq` + `where_like` + `paginate`), 401 for unauthenticated callers. | `cargo run -p auth-paginated-search` (port 3003) | Yes |
| `auth-demo` | JWT token generation/validation, bcrypt password hashing, role-based access control (`require_role` middleware), raw `rf-auth` usage without the implicit-request globals. | `cargo run -p auth-demo` (port 8080) | No |

---

## Realtime / WebSocket

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `realtime-chat` | `MemoryBroadcaster` + `websocket_router` from `rf-broadcast`. Clients subscribe to a channel (`room-1`) over WebSocket; a background task publishes chat messages independently. Connect with any WebSocket client on `ws://127.0.0.1:3030/ws`. | `cargo run -p realtime-chat` (port 3030) | Yes |

---

## i18n / Localization

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `i18n-localized-api` | `rf-i18n` axum integration: `AcceptLanguage` extractor resolves locale from `?locale=` query param or `Accept-Language` header. Shared `Arc<I18n>` extension. Per-request locale views via `I18n::for_locale()`. Plural-sensitive translations (`t_plural`) across English, German, French. | `cargo run -p i18n-localized-api` (port 3009) | Yes |

Routes (once running):

```
# German greeting via Accept-Language header
curl -H 'Accept-Language: de' http://127.0.0.1:3009/greet
# {"locale":"de","message":"Willkommen!"}

# French items count (plural)
curl -H 'Accept-Language: fr' 'http://127.0.0.1:3009/items?count=5'
# {"locale":"fr","count":5,"summary":"5 articles"}

# locale= query param overrides the header
curl 'http://127.0.0.1:3009/greet?locale=de'
# {"locale":"de","message":"Willkommen!"}
```

---

## Background Jobs

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `jobs-offline` | In-process job queue via `rf-queue` (`MemoryQueue` + `Jobs` facade + `Worker`). **No Redis required.** Dispatches and drains real jobs; exits when done. | `cargo run -p jobs-offline` | Yes |
| `jobs-demo` | Redis-backed queue via `rf-jobs` (`QueueManager`, `WorkerPool`, `Scheduler`, retry/backoff). Requires a running Redis instance; early-returns with a clear message if Redis is unreachable. | `cargo run -p jobs-demo` | No |

---

## Infrastructure / Utilities

| Example | Demonstrates | Run | Tested |
|---------|--------------|-----|--------|
| `database-demo` | SeaORM entity definitions, CRUD via `rf-orm`, query filtering, soft-deletes, transaction support. | `cargo run -p database-demo` | No |
| `mail-demo` | `rf-mail` mailable API: basic email, templates, attachments, multiple backends (SMTP/log). | `cargo run -p mail-demo` | No |
| `scaffold-demo` | `rf-scaffold` code generation: `ScaffoldEngine` generates model files from `ModelOptions`. | `cargo run -p scaffold-demo` | No |
| `macros-demo` | `rf-macros` `#[controller]` attribute, `rf-request`/`rf-response` raw request/response API. | `cargo run -p macros-demo` | No |
| `facades-demo` | Synchronous Laravel-style facades (`rf::prelude::*`): `Hash`, `Route`, `DB`, `Auth`, `Cache`. No `.await` in facade calls. | `cargo run -p facades-demo` | No |
| `laravel-syntax-simple` | Minimal: `Hash::make`/`Hash::check`, `csrf_token()`, `rules!` validation macro, `Route` facade — imported from `rf`. | `cargo run -p laravel-syntax-simple` | No |
| `laravel-syntax-complete` | Complete blog demo using `rf_global_helpers::{Hash, csrf_token, __}` with models, database layer, and route registration. | `cargo run -p laravel-syntax-complete` | No |
| `hello` | "Hello world" server using `rf-core`, `rf-web`, `rf-config`, `rf-container` directly — shows the raw Phase 2 modular architecture before the high-level `rf` umbrella. | `cargo run -p hello` | No |

---

## Non-workspace items (not in the example set)

The following directories exist under `examples/` but are **not** workspace members and are
not part of `cargo check --workspace`. They are noted here for completeness.

| Directory | Status | Notes |
|-----------|--------|-------|
| `htmx-livewire-alternative/` | Not a workspace member. Has `Cargo.toml` and `src/main.rs` but does not use any `rf-*` crates despite declaring `rf-blade`/`rf-sse` as dependencies; the code is standalone axum + htmx. Excluded to avoid workspace check noise. | Standalone axum/htmx demo; no RF integration. |
| `blog-complete/` | README only, no Cargo.toml or src/. | Aspirational design doc, not an executable crate. |
| `phase12-admin/` | README only, no Cargo.toml or src/. | Architecture sketch for rf-admin + rf-cms + rf-blade, not yet implemented. |
| `polymorphic_relationships.rs` | Loose `.rs` file at `examples/` root, not a crate. | Quick scratch note for polymorphic relation design; not runnable. |

---

## Running tests

```
# All examples with tests:
cargo test -p blog-slice
cargo test -p rest-crud-resource
cargo test -p taskflow
cargo test -p phase12-blog
cargo test -p validated-signup
cargo test -p auth-paginated-search
cargo test -p realtime-chat
cargo test -p jobs-offline
cargo test -p i18n-localized-api

# Or run the full workspace test suite (includes all of the above):
cargo test --workspace
```
