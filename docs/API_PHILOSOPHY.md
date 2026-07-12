# RustForge API Philosophy — Two-Layer Architecture

RustForge is a **Laravel-style application framework core for Rust**. Its identity —
the reason you reach for it over raw axum — is the **Laravel-style DX layer**: terse
handlers, `Model!` / `validate!`, global facades, and request helpers that let you
write an application in far less code than the explicit equivalent.

Underneath that ergonomic surface sits a fully **explicit Rust-native core**. You never
lose access to it: when you want full compile-time strictness, or you are running
outside an HTTP request (tests, CLI, background jobs), you drop down to the explicit
layer. Both are first-class — but the DX layer is the default you write, and the
explicit core is the honest foundation you can always fall back to.

---

## Layer 1 — Laravel-style DX (the default you write)

This is the primary surface and the framework's identity. It gives you Laravel-familiar
ergonomics with native-Rust performance:

- **The `Model!` ORM:** `Model!(Post: title, body)` then `Post::all().await`,
  `Post::find(id).await`, `create!(Post, ...)`, `update!`, `delete!` — Eloquent-style
  data access with real SeaORM underneath.

- **The `validate!` DSL:** typed, declarative validation that returns a 422 with
  per-field structured errors — no boilerplate.

- **Request globals (`input` / `file` / `has` / `all`):** free functions that read the
  current request, so simple handlers need no arguments:
  ```rust
  async fn store() -> impl axum::response::IntoResponse {
      let title: Option<String> = rf_request::input("title");
      let page:  Option<usize>  = rf_request::input("page");  // coerces "2" -> 2
      let has_avatar = rf_request::has("avatar");
      // ...
  }
  ```

- **Global facades (`Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`, `Broadcast`):**
  static calls from anywhere — `Auth::user()`, `Cache::get(...)`, `Mail::send(...)` —
  backed by real engines (Redis, SMTP, S3, ...) over a deadlock-safe `AsyncBridge`.

### When to use the DX layer

This is the **default** for application-layer handler code — the code you write day to
day. Use it whenever your router has the enabling middleware wired (`capture_request`,
`session_scope`, `require_auth`), which the framework's own scaffolding and the
reference app set up for you.

### Honest runtime caveats (kept transparent, by design)

The DX layer trades some of Rust's compile-time guarantees for ergonomic familiarity.
We do not hide that:

- **Request globals** (`input`/`file`/`has`/`all`) read a per-request task-local set by
  the `capture_request` middleware. Called **outside** a `capture_request`-wrapped
  handler, they return `None` / `false` / empty map **silently** — a runtime condition
  the compiler cannot catch. Wire `capture_request`, and cover the risk with
  integration tests.

- **`SessionFacade`** reads the current client's session via a `session_scope`
  task-local. Without `session_scope`, it falls back to a single **process-local**
  session shared by all callers — concurrent clients can bleed into each other. Always
  add `session_scope` when serving concurrent HTTP traffic.

- **`Auth::user()`** returns `None` outside a `require_auth` / `with_auth_scope` scope.

- The always-safe facades (`Cache`, `Mail`, `Storage`, `Queue`, `Event`, `Broadcast`)
  resolve to process-global singletons and are safe from anywhere, including CLI and
  background tasks — they do not depend on a per-request task-local.

When a handler needs a hard compile-time guarantee that a field is present, reach for
the explicit core (Layer 2) for that handler.

---

## Layer 2 — Explicit Rust-native core (the foundation / escape-hatch)

Every DX convenience is sugar over an explicit, ambient-state-free core. You drop to it
when you want maximum strictness or you are outside a request scope:

- **Typed handler arguments:** axum extractors such as `ValidatedJson<T>`,
  `rf_request::extractors::RequestExtractor`, or a plain `axum::Json<T>`. A missing or
  mistyped field is a **compile error**, not a runtime `None`.

- **Explicit `Request` struct:** `rf_request::Request` holds fields, files, user, and
  session as a concrete value — `Request::input`, `Request::file`, `Request::user`,
  `Request::session` are ordinary method calls with no ambient state and no middleware
  dependency.

- **`Result`-returning handlers:** return `AppResult<impl IntoResponse>` and use `?`;
  `AppError` maps to the correct HTTP status + JSON envelope automatically.

### When to drop to the explicit core

- Library code or shared middleware that may run outside a request scope.
- Unit tests — construct a `Request` directly, no middleware setup.
- Background tasks, CLI, or queued jobs where no HTTP request exists.
- A specific handler where you want compile-time proof of which fields it reads.

### Example — explicit core

```rust
use rf_request::extractors::RequestExtractor;
use rf_request::error::RequestResult;

// The compiler knows exactly what this handler needs:
async fn create_post(RequestExtractor(req): RequestExtractor) -> RequestResult<impl axum::response::IntoResponse> {
    let title: String = req.require("title")?;  // compile-time typed, ?-propagated
    let body: String  = req.require("body")?;
    Ok(axum::Json(serde_json::json!({ "ok": true })))
}
```

---

## Honest trade-offs

| Concern | Laravel-style DX (Layer 1) | Explicit core (Layer 2) |
|---|---|---|
| Lines of code for a simple handler | Fewest (argument-less, facades) | More (typed extractors, explicit `Request`) |
| Familiar to Laravel developers | Yes — the point | Less |
| Compile-time guarantee of field presence | No — `None` at runtime | Yes — typed extractor |
| Works outside a request scope (tests, CLI, jobs) | Partially (globals silent; session process-local) | Yes |
| Requires middleware in router | Yes (`capture_request`, `session_scope`, ...) | No |

The DX layer is the identity and the default — it makes handlers genuinely shorter and
lets Laravel developers be productive immediately. The explicit core is always there as
the honest foundation for the moments you want Rust's full compile-time strictness.

---

## Mixing the two layers

The layers coexist in the same router — mix them per handler. The `capture_request`
middleware buffers and re-inserts the body so the DX globals and a downstream
`axum::Json<T>` extractor can both read the same body in the same request.

```rust
use axum::{Router, routing::post, middleware};
use rf_request::{capture_request, input};
use rf_validation::ValidatedJson;

// One handler, both layers:
async fn create(ValidatedJson(body): ValidatedJson<MyDto>) -> impl axum::response::IntoResponse {
    let name = body.name;                       // explicit: compile-time typed
    let page: Option<usize> = input("page");    // DX: reads the same request
    axum::Json(serde_json::json!({ "name": name }))
}

let app = Router::new()
    .route("/items", post(create))
    .layer(middleware::from_fn(capture_request));  // enables the DX layer
```

---

## Summary

- **Laravel-style DX (Layer 1)** — `Model!`, `validate!`, `input()`/`file()`/`has()`,
  the `Auth`/`Cache`/`Mail`/`Storage`/`Queue` facades. The framework's identity and the
  default you write. Fewest lines, Laravel-familiar. Some correctness surfaces at
  runtime (documented caveats), not compile time.
- **Explicit Rust-native core (Layer 2)** — typed extractors, `Request`,
  `Result`-returning handlers. The honest foundation and escape-hatch. No ambient
  state, compile-time safe, works anywhere. Drop to it for strictness or outside a
  request scope.

Both are first-class. Write the DX layer by default; fall back to the explicit core
when you want Rust's full compile-time guarantees.
