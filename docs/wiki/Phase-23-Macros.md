# Helper Macros Reference

This page is a reference for `rf-macros`, the stable procedural-macro crate that powers
RustForge's Laravel-style syntax. The **canonical DX reference** — routing, request globals,
model declaration, validation, ORM, auth, cache, mail, and more — lives in
[Laravel-Syntax.md](Laravel-Syntax.md). This page focuses on macro expansion details and
documents each macro's real maturity based on what is actually in `crates/rf-macros`.

**Crate:** `rf-macros` — **stable** workspace member (tier per [docs/TIERS.md](../TIERS.md))
**Import:** `use rf::prelude::*` — the `rf` umbrella re-exports every macro listed here
unless stated otherwise.

---

## Stability overview

| Tier | Meaning |
|------|---------|
| **stable-verified** | In `rf::prelude`, proc-macro is real, used in a CI-passing example. |
| **stable-unverified** | In `rf::prelude`, proc-macro is real, but no shipped example exercises it end-to-end. |
| **broken** | Proc-macro exists in source but its expansion references a missing or stub crate; will not compile. |

---

## Stable, CI-verified macros

These macros are used by the CI-passing examples (`examples/blog-slice`,
`examples/phase12-blog`, `examples/rest-crud-resource`, `examples/reference-app`,
`examples/taskflow`) and form part of the v1 stable surface.

---

### `Model!` — declare a model struct and table mapping

**Tier:** stable-verified

The concise form infers `String` for every field:

```rust
use rf::prelude::*;

// Table name: "posts" (plural snake-case auto-derived).
// Generated: Post struct, FILLABLE constant, CreatePost / UpdatePost DTOs.
Model!(Post: title, body);
```

The explicit form supports non-String types, relations, scopes, and inline validation
constraints:

```rust
use rf::prelude::*;

Model!(Article {
    title:     String,
    body:      String,
    author_id: i64,
    views:     i64,
    belongsTo author: Author,   // eager-loadable, N+1-free
    hasMany comments: Comment,
    scope published: where("status", "published"),
});
```

Inline validation with the `@` DSL (requires `validated` marker; makes the generated DTO
implement `rf_validation::Validate`):

```rust
Model!(User {
    validated,
    name:  String @ min(1) max(80),
    email: String @ email message("A valid email address is required"),
    age:   i32    @ range(18.0, 120.0),
    slug:  String @ regex("^[a-z0-9-]+$"),
});
```

Available `@` constraints: `email`, `url`, `uuid`, `ip`, `min(N)`, `max(N)`,
`range(lo, hi)`, `regex("pattern")`, `alpha`, `alphanumeric`,
`starts_with("prefix")`, `ends_with("suffix")`, `message("text")`.

Source: `crates/rf-macros/src/simple_model.rs` (136 k lines), `examples/blog-slice/src/main.rs` line 15.

---

### `validate!` — typed fluent validation DSL

**Tier:** stable-verified

Validates the current request (reads the `capture_request` task-local) and returns
`Result<ValidatedData, ValidationErrors>`. Returns a structured 422 on failure when used
with `?` in a handler returning `AppResult`.

```rust
use rf::prelude::*;

async fn store() -> impl axum::response::IntoResponse {
    if validate! {
        title:     string.max(200),
        body:      string,
        author_id: int.min(1),
        email:     email,
        website:   url.optional,
        avatar:    image.max(mb(5)).min(kb(100)),
    }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }));
    }
    // proceed with validated input
    json(serde_json::json!({ "ok": true }))
}
```

**Type keywords:** `string` / `text`, `email`, `url`, `uuid`, `ip`,
`int` / `integer`, `float` / `decimal` / `number`,
`date` / `datetime`, `array`, `bool` / `boolean`,
`image` (validates upload MIME), `file` (any uploaded file).

**Modifiers:** `.min(N)`, `.max(N)`, `.between(lo, hi)`, `.optional`, `.nullable`,
`.unique("table", "col")`, `.exists("table", "col")`.
For `image` / `file`: `.mime("image/png")`, `.min(kb(100))`, `.max(mb(5))`.

**Runtime caveat:** `validate!` reads a per-request task-local populated by the
`capture_request` middleware. Called outside a `capture_request`-wrapped handler, the
macro silently sees an empty request and every field fails as absent. Always wire
`capture_request` and cover it with integration tests.

Source: `crates/rf-macros/src/validate_macro.rs`, `examples/blog-slice/src/main.rs` line 22.

---

### `create!` / `find!` / `update!` / `delete!` — CRUD without `.await`

**Tier:** stable-verified

These macros expand to real database operations (INSERT / SELECT / UPDATE / DELETE) via
`rf-orm`. The macro handles the `.await` internally so handlers stay synchronous-looking.

```rust
use rf::prelude::*;

Model!(Post: title, body);

async fn store_handler() -> impl axum::response::IntoResponse {
    // INSERT — returns Result<serde_json::Value, String>.
    // The returned Value contains the auto-assigned id and all persisted columns.
    let post = match create!(Post, title = "Hello", body = "World") {
        Ok(row) => row,
        Err(e)  => return json(serde_json::json!({ "error": e })),
    };
    json(post)
}

async fn show_handler() -> impl axum::response::IntoResponse {
    let id: i64 = input("id").unwrap_or(0);

    // SELECT by PK — returns Result<Option<serde_json::Value>, String>.
    match Post::find(id).await {
        Ok(Some(post)) => json(post),
        Ok(None)       => json(serde_json::json!({ "error": "not found" })),
        Err(e)         => json(serde_json::json!({ "error": e })),
    }
}

async fn update_handler() -> impl axum::response::IntoResponse {
    let id: i64 = input("id").unwrap_or(0);

    // UPDATE by PK — returns Result<u64, String> (affected rows, NOT the row).
    let affected = update!(Post, id, title = "Updated title").unwrap_or(0);
    json(serde_json::json!({ "affected": affected }))
}

async fn destroy_handler() -> impl axum::response::IntoResponse {
    let id: i64 = input("id").unwrap_or(0);

    // DELETE by PK — returns Result<u64, String> (affected rows).
    let deleted = delete!(Post, id).unwrap_or(0);
    json(serde_json::json!({ "deleted": deleted }))
}
```

**Return type distinction:**
- `create!` → `Result<serde_json::Value, String>` (the inserted row, with its generated `id`)
- `update!` → `Result<u64, String>` (rows affected; 0 means row was not found)
- `delete!` → `Result<u64, String>` (rows affected; 0 means row was not found)
- `find!` is thin sugar over `Model::find(id).await`

Source: `crates/rf-macros/src/helpers.rs` lines 1320–1450, `examples/blog-slice/src/main.rs` line 27.

---

### `#[auto_await]` and `#[await_calls]` — transparent async

**Tier:** stable-verified (in-crate integration tests; `crates/rf-macros/tests/`)

These attribute macros rewrite framework calls to add `.await` automatically, so you can
write handler bodies without explicit `.await`. Apply to a single function, an `impl`
block, or a whole `mod`:

```rust
use rf::prelude::*;

#[auto_await]
async fn list_users() -> impl axum::response::IntoResponse {
    // No `.await` needed; the macro inserts it for each framework call.
    let users = User::all();         // becomes User::all().await
    let count = DB::table("users").count();  // becomes .count().await
    json(users)
}

// Named alias — clearer when you list your own async methods:
#[await_calls(fetch_report, charge_stripe)]
async fn process() {
    let report = fetch_report();   // awaited
    let ok     = charge_stripe();  // awaited
}
```

`#[auto_await]` works on non-`async fn` too: it promotes them to `async` automatically
if it inserts any `.await` call.

**Known limitation:** the transformer is name-list-driven, not type-driven. It awaits any
call whose method name matches the built-in allowlist (`find`, `all`, `create`, `get`,
`put`, `send`, `dispatch`, …). A user-defined sync method with the same name will
silently have `.await` appended, which is unsound if that method is not `async`. This is
documented in `VISION_GAP.md`. For library code or handlers where this matters, use
explicit `.await`.

Source: `crates/rf-macros/src/await_transformer.rs`, tests in
`crates/rf-macros/tests/auto_await_sync_async.rs` and
`crates/rf-macros/tests/auto_await_makes_async.rs`.

---

### `#[controller]` — async controller impl blocks

**Tier:** stable (minimal usage in `examples/macros-demo`)

Decorates an `impl` block so each public method becomes `async` automatically:

```rust
use rf_macros::controller;
use rf_request::Request;
use rf_response::ResponseBuilder;

struct PostController;

#[controller]
impl PostController {
    pub fn index(_req: Request) -> ResponseBuilder {
        // Public methods are made async; `#[auto_await]` is applied.
        rf_response::Response::json(&serde_json::json!([]))
    }
}
```

Source: `crates/rf-macros/src/controller_macro.rs`, `examples/macros-demo/src/main.rs` line 43.

---

## In-prelude macros — present but no CI-verified example

These macros exist, are exported by `rf::prelude`, and generate real code. However, no
shipped example currently exercises them end-to-end. Treat them as functional DX sugar on
top of the stable engine; API may evolve.

---

### `routes!` — closure-free route registration

Route definitions without the `||` closure syntax (helpful on keyboard layouts where `|`
is awkward):

```rust
use rf::prelude::*;

routes! {
    get  "/posts"      => post_controller::index,
    post "/posts"      => post_controller::store,
    get  "/posts/{id}" => post_controller::show,
    put  "/posts/{id}" => post_controller::update,
    delete "/posts/{id}" => post_controller::destroy,

    middleware ["auth"] {
        get "/profile" => profile_controller::show,
        put "/profile" => profile_controller::update,
    }

    prefix "/api/v1" {
        get  "/users" => api::users::index,
        post "/users" => api::users::store,
    }
}
```

The macro expands to the standard `get(path, handler)` / `post(path, handler)` calls
from `rf-routing`. It is an ergonomic alias — the underlying engine is identical to
writing the route functions directly.

Source: `crates/rf-macros/src/laravel_macros.rs`, re-exported in `rf::prelude` line 239.

---

### `migration!` — table migration DSL

```rust
use rf::prelude::*;

migration! {
    create_table users {
        id:         primary,
        email:      string unique,
        name:       string,
        password:   string,
        role:       string = "user",
        timestamps,
    }
}
```

Expands to `DB::statement(...)` DDL calls. No separate migration runner is invoked; the
expanded call runs the DDL inline. Not a replacement for a migration versioning system.

Source: `crates/rf-macros/src/laravel_macros.rs`.

---

### `request!` — inline form-request validation struct

```rust
use rf::prelude::*;

request! {
    CreateUser {
        email:    email,
        name:     length(3, 50),
        password: length(8),
        age:      range(18, 120) | optional,
    }
}
```

Generates a validated form-request struct. For the more ergonomic `ValidatedJson`
extractor (compile-time safe, no ambient state), see the `Model!` `@` DSL or the
`#[validated]` attribute macro from `rf-macros`.

Source: `crates/rf-macros/src/laravel_macros.rs`, re-exported in `rf::prelude`.

---

### `dispatch!` — event dispatch sugar

```rust
use rf::prelude::*;

// Named-event form — dispatches to the string-keyed Event bus.
dispatch!(user.registered, user_id = 1, email = "john@example.com");

// Struct form — dispatches through the type-keyed event() helper.
dispatch!(UserRegistered { user_id: 1, email: "john@example.com".into() });

// Delayed struct dispatch (named events cannot be delayed).
dispatch!(delay: 3600, OrderShipped { order_id: 123 });
```

The named form expands to `Event::dispatch("user.registered", json_payload)` and the
struct form to `event(payload)` — both of which are real stable-tier facades. The
`delay` variant requires a type-erased struct event; the macro emits `compile_error!` if
you combine `delay:` with a named event.

Source: `crates/rf-macros/src/helpers.rs` line 1598, re-exported in `rf::prelude`.

---

### `controller_block!` — vision controller syntax

Generates a unit struct plus argument-less async handler methods that read the request
through the implicit-request globals:

```rust
use rf::prelude::*;

controller_block! {
    PostController {
        index() { json(Post::all().await.unwrap_or_default()) }
        show()  { json(Post::find(input::<i64>("id").unwrap()).await) }
        store() { json(create!(Post, title = input("title").unwrap_or_default())) }
    }
}

get("/posts",        PostController::index);
get("/posts/{id}",   PostController::show);
post("/posts",       PostController::store);
let app = global_router().build_router();
```

Source: `crates/rf-macros/src/controller_block_macro.rs`, re-exported in `rf::prelude`.

---

## `#[derive(Job)]` — ergonomic job definitions

**Tier:** stable-verified (in-crate; the `rf-queue` crate documents this derive)

The derive generates the mechanical wiring of `rf_queue::Job`. You implement only
`rf_queue::JobHandler` (the one method that matters):

```rust
use rf_queue::{Job, JobHandler, QueueError, Jobs, MemoryQueue, Queue, Worker};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Job)]
#[job(queue = "emails", retries = 5)]
struct SendEmail { to: String }

#[async_trait]
impl JobHandler for SendEmail {
    async fn handle(&self) -> Result<(), QueueError> {
        println!("sending email to {}", self.to);
        Ok(())
    }
}

// At startup:
let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());
Jobs::set_queue(Arc::clone(&queue));
let worker = Worker::new(queue).register::<SendEmail>();
tokio::spawn(async move { worker.start().await.ok(); });

// Dispatch from anywhere (no queue handle threaded through):
Jobs::dispatch(SendEmail { to: "user@example.com".into() })?;
SendEmail { to: "user@example.com".into() }.dispatch_now()?;
```

Supported `#[job(..)]` keys: `job_type` (alias `name`), `queue`, `retries`
(alias `max_retries`), `timeout` (seconds), `priority`.

Source: `crates/rf-macros/src/job_derive.rs`, documented in `crates/rf-queue/src/job.rs` line 109.

---

## Broken macros — do not use

### `send_mail!`

Exists in `rf-macros` and is re-exported in `rf::prelude`, but its expansion references
`rf_mail_facade::Mail::new()`. The `rf_mail_facade` crate is a **non-workspace stub**
(superseded when `rf-mail` absorbed the facade in the Phase 20 merge — see
[docs/TIERS.md](../TIERS.md)). Using this macro produces a compile error.

**Use instead:** the `Mail` facade from `rf_mail::MailFacade` (aliased as `Mail` in
`rf::prelude`). See the Mail facade section in [Laravel-Syntax.md](Laravel-Syntax.md#mail-facade).

### `job!` (function-like)

Exists in `rf-macros` and is re-exported at the `rf` crate top-level, but its expansion
references `rf_job_facade::Job` — a crate that does not exist in the workspace. Using this
macro produces a compile error. `VISION_GAP.md` explicitly documents this as a known
defect ("fix the broken `job!` macro: `rf-job-facade` → `rf-jobs`").

This macro is intentionally **not** re-exported in `rf::prelude`.

**Use instead:** `#[derive(Job)]` + `impl JobHandler` as shown above.

---

## Macros in `rf-macros` NOT exported from `rf::prelude`

The following proc-macros exist in `crates/rf-macros/src/` and compile, but are not
re-exported from `rf` or `rf::prelude`. They are available by importing `rf_macros`
directly, though none has a CI-verified end-to-end example:

| Macro | Source | Notes |
|-------|--------|-------|
| `cache!` | `helpers.rs:951` | Cache get/put/forget sugar; not in prelude |
| `mailable!` | `mailable_macro.rs` | Mailable email struct DSL; not in prelude |
| `notification!` | `mailable_macro.rs` | Notification DSL; not in prelude |
| `form_request!` | `form_request_macro.rs` | Full form-request with authorize(); not in prelude |
| `exception_handler!` | `exception_handler.rs` | Global exception handler; not in prelude |
| `blade!` | `blade_macro.rs` | Blade-like HTML template macro; not in prelude |
| `laravel!` | `laravel_syntax.rs` | PHP-style class … extends Model syntax; not in prelude |
| `rustforge!` | `rustforge_block.rs` | Block that applies auto_await to all inner fns |
| `function!` | `function_macro.rs` | Converts fn syntax to async closure; not in prelude |
| `rules!` | `rules_macro.rs` | Pipe-separated validation rules; re-exported in prelude via `rf_macros::rules` |
| `query!` | `query_macro.rs` | Transforms `where` → `r#where`; use `r#where` directly |
| `abort!`, `abort_if!`, `abort_unless!` | `exception_handler.rs` | HTTP abort helpers; not in prelude |

---

## Dropped from the old page

The previous version of this page claimed the following as working macros. After
grepping `crates/rf-macros` and all `examples/`:

| Old claim | Status |
|-----------|--------|
| `send_mail!` working | EXISTS but BROKEN — references non-workspace stub `rf_mail_facade` |
| `job!` working | EXISTS but BROKEN — references missing `rf_job_facade`; not in prelude |
| `mailable!` in prelude | EXISTS in `rf-macros` source but NOT in `rf::prelude` or `rf` re-exports |
| `request!` non-existent | EXISTS and IS in `rf::prelude` — old docs incorrectly omitted it |
| `dispatch!` non-existent | EXISTS and IS in `rf::prelude` — old docs incorrectly omitted it |
| "Version 1.0.4" / "Phase 23 adds…" | Removed — version/phase framing is marketing, not capability docs |

---

## See also

- [Laravel-Syntax.md](Laravel-Syntax.md) — canonical DX reference (the page to read first)
- [API-Documentation.md](API-Documentation.md) — type-level reference (function signatures, crate locations)
- [Features.md](Features.md) — full feature matrix with tier tags
- [docs/TIERS.md](../TIERS.md) — maturity tiers for every workspace crate
- `examples/blog-slice/` — minimal slice showing `Model!` + `validate!` + `create!`
- `examples/rest-crud-resource/` — five-verb CRUD with all four macros in use
- `examples/reference-app/` — flagship app exercising the full stable surface
