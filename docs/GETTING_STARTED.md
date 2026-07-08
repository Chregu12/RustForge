# Getting Started with RustForge

RustForge brings Laravel-style developer experience to Rust: one `Model!`
declaration, ambient request globals, a typed validation DSL, and real
facades — compiled to native Rust.

This guide is **honest**: every snippet below matches a real, current API and is
lifted from the two shipped, CI-tested reference examples
([`examples/blog-slice`](../examples/blog-slice/src/main.rs) and
[`examples/rest-crud-resource`](../examples/rest-crud-resource/src/main.rs)). The
[feature-maturity matrix](#feature-maturity-matrix) at the end grades what is
production-ready vs. partial vs. deferred so you always know what you are
standing on.

---

## 5-minute quickstart

We will build a tiny blog API — the exact shape of `examples/blog-slice`. Run
that example any time with `cargo run -p blog-slice` (serves on
`http://127.0.0.1:3000`).

### 1. One import

```rust
use rf::prelude::*;
```

The prelude brings in everything used below: `get`/`post`/`put`/`delete` +
`build_router`, the `capture_request` middleware, the `input`/`file`/`has`
request globals, `validate!`, `Model!`/`create!`/`find!`/`update!`/`delete!`,
and the `json` response helper.

### 2. Declare a model

One `Model!` line generates the struct, the typed CRUD surface, and (with
`validated`) validated Create/Update DTOs:

```rust
// A model backed by the real (SQLite) DB.
Model!(Post: title, body);
```

### 3. Write a handler — no `Request` argument

Handlers are argument-less and read the request through ambient globals.
`validate!` checks the current request; `input("field")` reads a typed value;
`create!` persists a real row; `json(..)` builds an `application/json` response:

```rust
/// POST /posts — validate, persist a real row, return it as JSON.
async fn create_post() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(100), body: string }.is_err() {
        return json(serde_json::json!({"error": "validation failed"}));
    }
    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();
    match create!(Post, title = title, body = body) {
        Ok(created) => json(created),
        Err(e) => json(serde_json::json!({"error": e.to_string()})),
    }
}

/// GET /posts — list every post (a real SELECT).
async fn list_posts() -> impl axum::response::IntoResponse {
    match Post::all().await {
        Ok(posts) => json(posts),
        Err(e) => json(serde_json::json!({"error": e.to_string()})),
    }
}

/// GET /posts/:id — the `:id` path param reaches the handler via `input`.
async fn show_post() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => return json(serde_json::json!({"error": "invalid id"})),
    };
    match Post::find(id).await {
        Ok(Some(post)) => json(post),
        Ok(None) => json(serde_json::json!({"error": "not found"})),
        Err(e) => json(serde_json::json!({"error": e.to_string()})),
    }
}
```

### 4. Wire the routes and serve

Register routes with the free `get`/`post` functions, then mount the global
router and the `capture_request` middleware (this is what makes `input`/`file`
work inside argument-less handlers):

```rust
fn build_app() -> axum::Router {
    post("/posts", create_post);
    get("/posts", list_posts);
    get("/posts/:id", show_post);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}

#[tokio::main]
async fn main() {
    DB::statement("CREATE TABLE IF NOT EXISTS posts (id INTEGER PRIMARY KEY, title TEXT, body TEXT)")
        .expect("create table");
    let app = build_app();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
```

That is a complete, real request → validate → model → response slice.

---

## Building a real REST resource

The `examples/rest-crud-resource` example is the canonical full CRUD story:
all five verbs, correct status codes, validation, and an eager-loaded relation.

### Model with a relation

```rust
use axum::http::StatusCode;
use rf::prelude::*;

Model!(Author {
    name: String,
});

Model!(Article {
    title: String,
    body: String,
    author_id: i64,

    belongsTo author: Author,
});
```

### The five verbs with status codes

`json(..)` returns a `ResponseBuilder`, so you can attach a REST status code.
`find!`/`update!`/`delete!` mirror `create!`. `update!`/`delete!` report the
number of affected rows, so `Ok(0)` cleanly maps to `404`:

```rust
/// POST /articles — 201 on success, 422 on validation failure.
async fn store() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(200), body: string, author_id: int.min(1) }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let title: String = input("title").unwrap_or_default();
    let body: String = input("body").unwrap_or_default();
    let author_id: i64 = input("author_id").unwrap_or_default();
    match create!(Article, title = title, body = body, author_id = author_id) {
        Ok(created) => json(created).status(StatusCode::CREATED),
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// GET /articles/:id — 200 or 404.
async fn show() -> impl axum::response::IntoResponse {
    let id: i64 = match input("id") {
        Some(id) => id,
        None => return json(serde_json::json!({ "error": "invalid id" }))
            .status(StatusCode::BAD_REQUEST),
    };
    match find!(Article, id) {
        Ok(Some(article)) => json(article).status(StatusCode::OK),
        Ok(None) => json(serde_json::json!({ "error": "not found" })).status(StatusCode::NOT_FOUND),
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// PUT  /articles/:id -> update!(Article, id, title = .., body = .., author_id = ..)
// DELETE /articles/:id -> delete!(Article, id), then Response::no_content() (204)
```

Routes are wired exactly as in the quickstart, adding `put` and `delete`:

```rust
fn build_app() -> axum::Router {
    get("/articles", index);
    post("/articles", store);
    get("/articles/:id", show);
    put("/articles/:id", update);
    delete("/articles/:id", destroy);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}
```

### Eager-loading a relation (no N+1)

The typed fetch-time builder `Model::with(names).get()` runs ONE fetch plus one
batched loader query per relation and returns typed rows with the relation
**field** populated:

```rust
/// GET /articles — each article's `author` is eager-loaded (no N+1).
async fn index() -> impl axum::response::IntoResponse {
    match Article::with(&["author"]).get().await {
        Ok(articles) => json(articles),  // each `articles[i].author` is populated
        Err(e) => json(serde_json::json!({ "error": e })).status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

All four relation kinds (`belongsTo`, `hasOne`, `hasMany`, `belongsToMany`)
hydrate as populated fields. You can request several at once:

```rust
let posts = Post::with(&["user", "comments"]).get().await?; // typed, N+1-free
```

### Typed reads and pagination

Beyond the trait `Post::all()` (which yields `Vec<serde_json::Value>`), each
`Model!` also generates typed inherent reads:

```rust
let posts: Vec<Post> = Post::all_typed().await?;        // concrete typed rows

// Generated typed page struct `<Name>Page { data, total, per_page, current_page, last_page }`
let page = Post::paginate(15, 2).await?;                // per_page = 15, page = 2
let rows: &Vec<Post> = &page.data;
```

### Validated DTOs via the `@` DSL

Add the `validated` marker to opt the generated Create/Update DTOs into a real
`Validate` impl, then layer explicit checks with the `@` field DSL. A
`ValidatedJson<CreateArticle>` extractor then deserializes **and** validates the
body in one step:

```rust
Model!(Article {
    validated,
    title: String @ min(1) max(20),
    email: String @ email message("Please enter a valid email"),
    website: String @ url,
    token: String @ uuid,
    client_ip: String @ ip,             // IPv4 or IPv6
    zipcode: String @ regex("^\\d{5}$"),
    slug: String @ alpha,
    username: String @ alphanumeric,
    code: String @ starts_with("SKU-"),
    rating: i64 @ range(1, 5),
    body: String,
});

async fn create_article(ValidatedJson(article): ValidatedJson<CreateArticle>) -> String {
    // If we get here, the body already validated.
    format!("created: {}", article.title)
}
```

The `@` DSL exposes `email`/`url`/`uuid`/`ip`/`regex`/`alpha`/`alphanumeric`/
`starts_with`/`ends_with` + string `min`/`max` + numeric `range` + a per-field
custom `message`.

---

## Feature-maturity matrix

Graded against the real state recorded in `VISION_GAP.md` (Re-audits 1–5;
overall ~89% of the vision is genuinely real). **Done** = real engine, on a
primary path, probe/CI-verified. **Partial** = real but with a documented gap.
**Deferred** = intentionally not built yet, with a reason (never faked).

| Capability | Status | Note |
|---|---|---|
| Routing — `get`/`post`/`put`/`delete` + `build_router` | ✅ Done | Real axum serving via `global_router().build_router()`. (The old string `"Controller@index"` facade is legacy and deliberately superseded.) |
| Request globals — `input`/`file`/`has` + `capture_request` | ✅ Done | Ambient task-local set by the `capture_request` middleware; multipart `file()` and path params are real. |
| ORM CRUD — `Model!` + `create!`/`find!`/`update!`/`delete!` | ✅ Done | Real SQLite-backed rows through `rf-db-facade`; not mock data. |
| Typed reads — `all_typed()` / `paginate()` / `with().get()` | ✅ Done | Typed `Vec<Self>`, a generated `<Name>Page`, and a fetch-time eager builder (runs 13–14). |
| Relations (eager) — `belongsTo`/`hasOne`/`hasMany`/`belongsToMany` | ✅ Done | All four kinds hydrate as populated fields via N+1-free batch loaders; `with(names).get()` hydrates during the fetch. |
| Validation — `validate!` + the `Model!` `@` DSL | ✅ Done | Typed DSL; `@` exposes email/url/uuid/ip/regex/alpha/alphanumeric/starts_with/ends_with + min/max + range + custom message. |
| Facades — DB/Cache/Mail/Storage/Auth/Queue/Event/Broadcast/Notifications/AI | ✅ Done | All real over the deadlock-safe `AsyncBridge` (no raw `block_on`). |
| Live backends — Redis / SMTP / S3 | 🟨 Partial | Bridges are real and proven; the live round-trip tests **graceful-skip** unless you bring services up (`docker compose -f docker-compose.test.yml up`). |
| Blade-style views — `view(name, data)` / `blade!` | 🟨 Partial | Real render with `{{ var }}` interpolation and `@if`/`@foreach`; not the full Blade feature set. |
| Auth / RBAC | 🟨 Partial | Per-request auth state (an earlier cross-request global was fixed), JWT/session guards, and gates/policies exist; not exhaustively audited end-to-end. |
| Nested / constrained eager loads + query scopes | ✅ Done | One-level nested dot-paths (`with(&["comments.author"])`), constrained eager loads (`.with_where(relation, col, val)`), and Laravel-style local query scopes (a `scope name: …` line generates `Model::name() -> QueryBuilder`) all ship as generated code and pass tests. One documented edge: a nested `a.b` combined with `with_where` constrains only the first segment (`a`) — see Known limitations. |
| Mass-assignment guard | 🟥 Deferred | `FILLABLE`/`HIDDEN` consts are declared but `create()` is not yet gated on them (behavior change, done carefully). |
| `Result` / `Option` hiding | 🟥 Deferred | A documented **language ceiling**: `.await` is hidden via `#[auto_await]`, but `?` / `Result` / `Option` stay visible (hiding them needs one uniform error type). |
| Peripheral stubs — `load_session` / `readiness_check` / `init_telemetry` | 🟥 Deferred | Each needs external infra or a design decision; left honestly stubbed rather than faked. |

---

## Known limitations

Read these before you build — they are the real ceilings, stated plainly:

- **`Result`/`Option` are not hidden.** RustForge hides `.await` (via a name-list
  `#[auto_await]`), but error handling stays visible: you still write `?`, match
  on `Result`, and handle `Option`. This is a deliberate language ceiling, not a
  bug — hiding it would require a single uniform error type and lossy `anyhow`
  coercion.
- **Ambient globals are request-scoped.** `input()`/`file()`/`has()` read a
  task-local set by the `capture_request` middleware. Calling them **outside** a
  request context returns empty/None (a runtime condition, not a compile error),
  so keep them inside handlers.
- **Live-backend tests skip by default.** Redis/SMTP/S3 round-trips are wired and
  real, but their tests skip gracefully unless you start the services from
  `docker-compose.test.yml`. Offline `cargo test` is green because those paths
  no-op, not because they are fake.
- **Eager loading is deep-but-bounded.** Top-level relations (`with(&["author"])`),
  one-level nested dot-paths (`with(&["comments.author"])`), constrained eager
  loads (`.with_where("comments", "approved", true)`), and Laravel-style local
  query scopes (`Post::active()`, returning the real `QueryBuilder` to keep
  chaining) all ship and are N+1-free (one batched query per relation). The real
  sharp edges: (1) combining a nested `a.b` with `.with_where(...)` applies the
  constraint to the **first** segment (`a`) only — constraining the deeper
  segment (`b`) is a documented follow-up; (2) `with_where` records one equality
  constraint per relation (repeating it replaces the prior one; multiple
  constraints and operators are a follow-up); and (3) nesting is one level deep
  per arm, with deeper paths (`a.b.c`) handled by re-entering the child model's
  own loaders.
- **Not yet 1.0-stable across the board.** The engine phase is essentially done
  (~89% of the vision), but several peripheral crates carry warnings or stubs and
  most core crates still lack in-crate unit tests. The two examples above are the
  blessed, CI-tested reference paths — start from them.
</content>
</invoke>
