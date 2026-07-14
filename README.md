# RustForge

**A Laravel-style application framework core for Rust.**

Laravel-familiar ergonomics — `Model!`, `validate!`, global facades, argument-less
handlers — compiled to a single native binary on **axum 0.8** + **Tokio**.

[![Rust](https://img.shields.io/badge/rust-1.96%2B-orange)]()
[![Release](https://img.shields.io/badge/release-v1.0.0--rc.2-blue)](https://github.com/Chregu12/RustForge/releases/tag/v1.0.0-rc.2)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)]()
[![Wiki](https://img.shields.io/badge/docs-wiki-brightgreen)](https://github.com/Chregu12/RustForge/wiki)

---

## What you get

- **Write less** — `Model!(Post: title, body)` generates the struct, table mapping, and `create!`/`find!`/`update!`/`delete!` macros in one line.
- **Hide async** — argument-less handlers read the request through `input()`/`file()`/`has()`; `.await` is tucked inside the macros.
- **Global facades** — `Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`, `Broadcast` as static calls, backed by real engines over a deadlock-safe `AsyncBridge`.
- **Typed validation DSL** — `validate! { email: email, age: int.min(18) }` builds real rules; 422 with per-field errors.

Stable surface (34 crates): routing · request/response · validation · **ORM (SQLite default, Postgres via `DATABASE_URL`)** · **auth (JWT `require_auth`)** · cache · queue · mail · storage · observability. Two layers coexist — the Laravel-style DX you write by default, and an explicit Rust-native core as the escape-hatch ([API_PHILOSOPHY.md](docs/API_PHILOSOPHY.md)).

> **Status: `v1.0.0-rc.2` — a release candidate, not yet production-proven** (0 external users). Honest self-assessment ~6.0–6.5/10, see [REVIEW_RESPONSE.md](docs/REVIEW_RESPONSE.md). Pin to a final tag before production use.

---

## Install

RustForge is a git monorepo (not on crates.io). Add the umbrella crate:

```toml
[dependencies]
rf = { git = "https://github.com/Chregu12/RustForge", tag = "v1.0.0-rc.2" }
```

MSRV **1.79.0** (built and tested on 1.96.0). See the [Installation wiki](https://github.com/Chregu12/RustForge/wiki/Installation) for path deps and individual crates.

---

## Example

A complete request → validate → model → response slice, backed by a real SQLite
database — no config file, no env var, no running service. (From the CI-tested
`examples/blog-slice`; run it with `cargo run -p blog-slice`.)

```rust
use rf::prelude::*;

// Struct + `posts` table + INSERT/SELECT/UPDATE/DELETE macros, in one line.
Model!(Post: title, body);

// No `Request` parameter — input/validate!/create!/json come from the prelude.
async fn create_post() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(100), body: string }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }));
    }
    let title: String = input("title").unwrap_or_default();
    let body:  String = input("body").unwrap_or_default();
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

#[tokio::main]
async fn main() {
    DB::statement("CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT, body TEXT)")
        .expect("create table");

    post("/posts", create_post);
    get("/posts",  list_posts);
    let app = rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

More: [validation](https://github.com/Chregu12/RustForge/wiki/Laravel-Syntax) · auth + Postgres CRUD in `examples/reference-app` · full [Quick-Start](https://github.com/Chregu12/RustForge/wiki/Quick-Start).

---

## Documentation → [the Wiki](https://github.com/Chregu12/RustForge/wiki)

| Page | What's in it |
|------|--------------|
| [Home](https://github.com/Chregu12/RustForge/wiki/Home) | Overview + orientation |
| [Installation](https://github.com/Chregu12/RustForge/wiki/Installation) | Add RustForge to a project (git/path deps, MSRV) |
| [Quick-Start](https://github.com/Chregu12/RustForge/wiki/Quick-Start) | Build your first app end-to-end |
| [Laravel-Syntax](https://github.com/Chregu12/RustForge/wiki/Laravel-Syntax) | `Model!`, `validate!`, facades, request globals, routing, auth |
| [API-Documentation](https://github.com/Chregu12/RustForge/wiki/API-Documentation) | Reference by capability |
| [Features](https://github.com/Chregu12/RustForge/wiki/Features) | Capability list tagged by maturity |
| [Examples](https://github.com/Chregu12/RustForge/wiki/Examples) | Tour of the shipped example apps |
| [Migration-Guide](https://github.com/Chregu12/RustForge/wiki/Migration-Guide) | For Laravel developers |
| [Tinker](https://github.com/Chregu12/RustForge/wiki/Tinker) | Interactive REPL |

In-repo deep-dives: [STABLE_CORE.md](docs/STABLE_CORE.md) (the v1 API contract) · [TIERS.md](docs/TIERS.md) (per-crate maturity: 34 stable / 76 beta / 8 experimental / 9 stub) · [API_PHILOSOPHY.md](docs/API_PHILOSOPHY.md) · [COOKBOOK.md](docs/COOKBOOK.md) · [RELEASING.md](docs/RELEASING.md) · [SECURITY.md](SECURITY.md) · [CHANGELOG.md](CHANGELOG.md).

---

## Build

```bash
cargo check --workspace     # 0 warnings required (CI-enforced, incl. clippy on the stable surface)
cargo test  --workspace     # live-backend tests skip gracefully without services
```

CI runs the workspace gate (`-D warnings` + clippy), a committed convergence probe-sweep, a full-suite job, and live-backend integration (Postgres/Redis/MailHog/MinIO via Docker) on every push.

---

## License

MIT OR Apache-2.0.
