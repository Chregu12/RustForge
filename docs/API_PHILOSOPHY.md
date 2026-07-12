# RustForge API Philosophy — Two-Layer Architecture

RustForge is a **Laravel-style application framework core for Rust** built on two
deliberately separate layers. Understanding which layer to reach for, and why, leads
to cleaner, more maintainable code.

---

## Layer 1 — Explicit Rust-native core (the foundation)

The explicit layer is the **recommended default** for all new code. It consists of:

- **Typed handler arguments:** axum extractors such as `ValidatedJson<T>`,
  `rf_request::extractors::RequestExtractor`, or a plain `axum::Json<T>` receive the
  parsed request and hand it to the handler as a regular Rust value. The compiler
  enforces that the field exists and has the right type — a missing or mistyped field
  is a compile error, not a runtime `None`.

- **Explicit `Request` struct:** `rf_request::Request` holds the parsed fields, files,
  user, and session. Methods like `Request::input`, `Request::file`, `Request::has`,
  `Request::all`, `Request::user`, `Request::session` are ordinary method calls on a
  concrete value — no ambient state, no middleware dependency, no surprises in tests.

- **`Result`-returning handlers:** handlers return `AppResult<impl IntoResponse>` and
  use `?` for error propagation. `AppError` maps to the correct HTTP status code and
  JSON envelope automatically.

### When to use the explicit core

Use it when:

- Writing library code or shared middleware that may be called outside a request scope.
- Writing unit tests — no middleware setup required, just construct a `Request`.
- You want compile-time guarantees about which fields a handler reads.
- Code may run in background tasks, CLI, or queued jobs where no HTTP request exists.

### Example — explicit core

```rust
use rf_request::{Request, extractors::RequestExtractor};
use rf_request::error::RequestResult;

// The compiler knows exactly what this handler needs:
async fn create_post(RequestExtractor(req): RequestExtractor) -> RequestResult<impl axum::response::IntoResponse> {
    let title: String = req.require("title")?;
    let body: String  = req.require("body")?;
    // ... store in DB ...
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}
```

---

## Layer 2 — Laravel-style DX convenience (opt-in)

The convenience layer provides Laravel-familiar ergonomics: argument-less handlers,
global request helpers, and static facade calls. It is **opt-in** — each piece requires
a specific middleware layer to be present in the router.

### Request globals (`input` / `file` / `has` / `all`)

`rf_request::input`, `rf_request::has`, `rf_request::file`, and `rf_request::all` are
free functions that read a **per-request task-local** populated by the
`capture_request` middleware.

**Runtime caveat:** When called outside a request handler that is wrapped by
`capture_request`, these functions return `None` / `false` / empty map **silently**.
This is a *runtime* condition — the compiler cannot detect it. There is no compile
error, no panic, just a missing value.

```rust
// Requires capture_request in the router — silent None without it:
async fn handler() -> impl axum::response::IntoResponse {
    let title: Option<String> = rf_request::input("title");  // None outside scope
    let page:  Option<usize>  = rf_request::input("page");   // coerces "2" -> 2
    let has_file = rf_request::has("avatar");                 // false outside scope
}
```

### Session facade (`SessionFacade`)

`rf_web::SessionFacade` reads and writes the **current client's session**, identified
by a task-local set by the `session_scope` middleware.

**Runtime caveat:** Without `session_scope` in the router the facade falls back to a
single **process-local** session shared by all callers in that process. Multiple
concurrent HTTP requests served without `session_scope` share this single session —
data from one client can bleed into another. Always add `session_scope` when serving
concurrent HTTP traffic.

### Auth facade (`Auth`)

`rf_auth::Auth` reads the per-request authenticated user from a task-local set by
`require_auth` or `with_auth_scope`. Outside a scope, `Auth::user()` returns `None`.

### Global facades (`Cache`, `Mail`, `Storage`, `Queue`, `Event`, `Broadcast`)

These facades resolve to process-global singletons and are safe to call from anywhere
— including background tasks and CLI — because they do not depend on a per-request
task-local. They trade some of Rust's explicitness for Laravel-style convenience, and
are backed by real engines (Redis, SMTP, S3, etc.) over a deadlock-safe `AsyncBridge`.

### When to use the DX layer

Use it when:

- You are writing application-layer handler code (not library code).
- The router always has `capture_request` / `session_scope` / `require_auth` wired.
- You value Laravel-familiar syntax and argument-less handlers.
- You accept that missing-middleware bugs surface at runtime as silent empty values,
  not compile errors, and you cover that risk with integration tests.

---

## Honest trade-offs

| Concern | Explicit core | DX convenience layer |
|---|---|---|
| Compile-time guarantee of field presence | Yes — typed extractor | No — `None` at runtime |
| Works outside a request scope (tests, CLI, jobs) | Yes | Partially (globals silent; session falls back to process-local) |
| Familiar to Laravel developers | Less | Yes |
| Requires middleware in router | No | Yes (`capture_request`, `session_scope`, ...) |
| Testable without middleware setup | Yes | No (or with `with_request_context` in tests) |

The convenience layer is not a mistake — it makes simple handlers genuinely shorter
and more readable, and Laravel developers can be productive immediately. But it
**trades some of Rust's compile-time guarantees for ergonomic familiarity**, and
you should know you are making that trade.

---

## Mixing the two layers

The layers coexist in the same router. A single application can use typed extractors
for complex handlers and the global helpers for simple ones. The `capture_request`
middleware buffers and re-inserts the body so both the global helpers and a
downstream `axum::Json<T>` extractor can read the same body in the same request.

```rust
use axum::{Router, routing::post, middleware};
use rf_request::{capture_request, input};
use rf_validation::ValidatedJson;

// This handler uses BOTH layers in the same router:
async fn create(ValidatedJson(body): ValidatedJson<MyDto>) -> impl axum::response::IntoResponse {
    // explicit layer: ValidatedJson gives compile-time types
    let name = body.name;
    // DX layer: input() reads the same body via the task-local
    let page: Option<usize> = input("page");  // from query string
    axum::Json(serde_json::json!({ "name": name }))
}

let app = Router::new()
    .route("/items", post(create))
    .layer(middleware::from_fn(capture_request));  // enables the DX layer
```

---

## Summary

- **Explicit Rust-native core** — typed extractors, `Request`, `Result`-returning
  handlers. Foundation. No ambient state. Compile-time safe. Recommended default.
- **Laravel-style DX layer** — `input()`/`file()`/`has()`, `SessionFacade`, `Auth`
  facade, etc. Opt-in. Requires middleware. Returns empty/`None` silently outside
  scope. Trades compile-time guarantees for Laravel ergonomics.

Both are first-class citizens of RustForge. Choose the one that fits your context,
or mix them within the same application.
