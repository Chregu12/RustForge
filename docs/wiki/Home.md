# RustForge

[![Crates.io](https://img.shields.io/badge/crates.io-rustforge-blue)](https://crates.io/crates/rustforge)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-green)](https://github.com/Chregu12/RustForge/blob/main/LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange)](https://www.rust-lang.org/)

**RustForge** is a Laravel-style application framework for Rust. Its identity is the **Laravel-DX layer**: terse handlers, an `Model!` / `validate!` macro DSL, global facades (`Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`), and request helpers that let you write application code in less total code than the explicit equivalent. Underneath sits a fully **explicit Rust-native core** you can always drop to for compile-time strictness — see [docs/API_PHILOSOPHY.md](../../API_PHILOSOPHY.md).

The v1 stable surface is formed by **34 crates** and is defined precisely in [docs/STABLE_CORE.md](../../STABLE_CORE.md). An additional 76 beta crates and 8 experimental crates exist in the workspace; their maturity is documented in [docs/TIERS.md](../../TIERS.md).

Current release: **1.0.0-rc.1** (2026-07-11).

---

## Getting started in 60 seconds

Add the framework to your `Cargo.toml`:

```toml
[dependencies]
rf = "1.0"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
```

A minimal API handler:

```rust
use rf::prelude::*;
use axum::response::IntoResponse;

// Declare a model — table name is `posts` (pluralised automatically).
Model!(Post {
    title: String,
    body:  String,
});

// GET /posts — Laravel-style, no Request argument.
async fn index() -> impl IntoResponse {
    match Post::all().await {
        Ok(posts) => json(posts),
        Err(e)    => json(serde_json::json!({ "error": e.to_string() })),
    }
}

// POST /posts — reads body via request globals.
async fn store() -> impl IntoResponse {
    if validate! { title: string.max(200), body: string }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }))
            .status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
    let title: String = input("title").unwrap_or_default();
    let body:  String = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(row) => json(row).status(axum::http::StatusCode::CREATED),
        Err(e)  => json(serde_json::json!({ "error": e.to_string() })),
    }
}

#[tokio::main]
async fn main() {
    get("/posts",  index);
    post("/posts", store);
    let app = rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

`use rf::prelude::*` is the only import you need for handler files. One line gives you all stable facades, macros, and helpers. See [docs/STABLE_CORE.md](../../STABLE_CORE.md) for the complete prelude inventory.

---

## Architecture: two layers

| | Laravel-style DX (default) | Explicit Rust-native core |
|---|---|---|
| Import | `use rf::prelude::*` | crate-level types, axum extractors |
| Handler args | none — `input()` / `file()` / `has()` globals | `ValidatedJson<T>`, `RequestExtractor`, axum `Path` |
| Auth | `Auth::user()` (requires `require_auth` layer) | `JwtManager::verify(token)` |
| Errors | `AppError` implements `IntoResponse` | `Result<T, AppError>` with `?` |
| Works outside HTTP | partially (globals silent without middleware) | yes — CLI, tests, background jobs |

Both layers coexist in the same router. Full philosophy at [docs/API_PHILOSOPHY.md](../../API_PHILOSOPHY.md).

---

## Database support

The DB facade (`Model!`, `create!`, `find!`, `update!`, `delete!`, `DB::table(...)`) uses:

- **SQLite** (rusqlite) — the default, including in-memory (`DATABASE_URL` absent or a file path)
- **Postgres** (sqlx PgPool) — selected when `DATABASE_URL` is a `postgres://` or `postgresql://` URL

Both backends are CI-tested. MySQL code exists in the schema builder but is not part of the v1 CI suite.

---

## Honest status

- **34 stable crates** — v1 API contract; no breaking changes without a major-version bump. Listed in [docs/TIERS.md](../../TIERS.md).
- **76 beta crates** — real implementations; API may shift in minor versions.
- **8 experimental crates** — excluded from `default-members` and the 1.0 supported surface: `rf-nova`, `rf-nova-macros`, `rf-swagger`, `rf-telescope`, `rf-cms`, `rf-breeze`, `rf-vite`, `rf-livereload`. No SemVer guarantee.
- **9 stub crates** — superseded facade directories kept for path-reference compatibility only.

See [docs/TIERS.md](../../TIERS.md) for the full crate-by-crate breakdown and [CHANGELOG.md](../../CHANGELOG.md) for what changed in 1.0.0-rc.1.

---

## Wiki pages

| Page | What you'll find |
|------|-----------------|
| [Installation](Installation) | Cargo setup, `forge` CLI install, env vars |
| [Quick-Start](Quick-Start) | Step-by-step first app tutorial |
| [Laravel-Syntax](Laravel-Syntax) | DX-layer guide: Model!, validate!, facades, request globals |
| [Features](Features) | Full capability list with maturity tags (stable / beta / experimental) |
| [API-Documentation](API-Documentation) | Per-module API reference tables |
| [Examples](Examples) | Annotated example apps from `examples/` |
| [Migration-Guide](Migration-Guide) | Moving from axum, Actix-web, or Rocket |

---

## Key repo docs

| Doc | Purpose |
|-----|---------|
| [docs/STABLE_CORE.md](../../STABLE_CORE.md) | Exact v1 API contract — every stable entry point, grep-verified |
| [docs/API_PHILOSOPHY.md](../../API_PHILOSOPHY.md) | Two-layer framing and honest trade-offs |
| [docs/TIERS.md](../../TIERS.md) | Complete crate maturity roster (34 stable / 76 beta / 8 experimental) |
| [docs/COOKBOOK.md](../../COOKBOOK.md) | Task-oriented recipes with CI-tested snippets |
| [docs/RELEASING.md](../../RELEASING.md) | SemVer policy, MSRV, deprecation policy |
| [SECURITY.md](../../SECURITY.md) | Security policy and responsible disclosure |
| [CHANGELOG.md](../../CHANGELOG.md) | Release history and known limitations |

---

## Example applications

The `examples/` directory contains runnable apps, each compiling in CI:

| App | Surfaces exercised |
|-----|--------------------|
| `examples/reference-app/` | Auth, CRUD, Cache, Storage, Queue, Mail, Health, Metrics — the flagship |
| `examples/rest-crud-resource/` | Full CRUD lifecycle, `resource()` routing |
| `examples/taskflow/` | Relations, pagination, search, `require_auth` |
| `examples/validated-signup/` | `validate!` DSL and `ValidatedJson<T>` extractor |
| `examples/auth-demo/` | JWT login flow, `require_auth` middleware |
| `examples/realtime-chat/` | WebSocket broadcast, room isolation |
| `examples/jobs-offline/` | `MemoryQueue` + `Worker`, no Redis required |
| `examples/i18n-localized-api/` | `AcceptLanguage` extractor, plural rules |
| `examples/facades-demo/` | All facades wired together |

---

## System requirements

- **Rust**: 1.82 or higher (MSRV per [docs/RELEASING.md](../../RELEASING.md))
- **Database**: SQLite 3.35+ (default) or Postgres 14+
- **Cache/Queue** (optional): Redis 6+
- **OS**: Linux, macOS, or Windows

---

## Community

- **Repository**: https://github.com/Chregu12/RustForge
- **Issues**: https://github.com/Chregu12/RustForge/issues
- **Security**: see [SECURITY.md](../../SECURITY.md)
- **License**: MIT OR Apache-2.0
