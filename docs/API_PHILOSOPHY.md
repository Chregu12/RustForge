# RustForge API Philosophy — Two Equal, First-Class Paths

RustForge gives you **two fully supported ways to write application code**, and
you choose based on what the situation calls for:

| | DX façade path | Typed DI path |
|---|---|---|
| **Best for** | Small apps, rapid handlers, solo developer, Laravel migration | Larger apps, teams, anywhere testability and explicit dependencies matter |
| **Dominant style** | `Auth::user()`, `Cache::get(...)`, argument-less handlers | `State<AppState>`, `Arc<dyn Service>`, `Result`-returning handlers |
| **Canonical example** | `examples/facades-demo` | `examples/typed-service` |

Neither path is an escape-hatch or a fallback. Both are designed and maintained
as first-class options. The right choice depends on your app's scale and the
degree of testability you need.

---

## Path A — Laravel-style DX façades

This path gives you Laravel-familiar ergonomics with native-Rust performance.
It is the fastest way to get an endpoint working and the most readable path for
developers coming from Laravel.

### What it looks like

```rust
// No explicit arguments, no State<>, no Arc — the framework does the plumbing.
async fn store() -> impl axum::response::IntoResponse {
    let title: Option<String> = rf_request::input("title");
    Auth::user();          // current authenticated user
    Cache::get("key");     // Redis-backed cache
    Mail::send(mailable);  // sends via configured driver
    // ...
}
```

### Feature catalogue

- **The `Model!` ORM:** `Model!(Post: title, body)` then `Post::all().await`,
  `Post::find(id).await`, `create!(Post, ...)`, `update!`, `delete!` —
  Eloquent-style data access with real SeaORM underneath.

- **The `validate!` DSL:** typed, declarative validation that returns a 422
  with per-field structured errors — no boilerplate.

- **Request globals (`input` / `file` / `has` / `all`):** free functions that
  read the current request, so simple handlers need no arguments.

- **Global façades (`Auth`, `Cache`, `Mail`, `Storage`, `Queue`, `Event`,
  `Broadcast`):** static calls from anywhere — backed by real engines (Redis,
  SMTP, S3, ...) over a deadlock-safe `AsyncBridge`.

### When to use this path

- Small to medium applications where a single developer or a tight-knit team
  owns the whole codebase.
- Rapid prototyping or migrating a Laravel application where familiar syntax
  accelerates the work.
- Handler code where explicit DI would add boilerplate with no practical
  benefit.

### Runtime caveats — fail-fast, not silent

The DX layer trades some of Rust's compile-time guarantees for ergonomic
familiarity. Where that could hide a bug, missing middleware now **fails fast**
(loud panic with an actionable message) instead of silently returning a wrong
value:

- **Request globals** (`input`/`file`/`has`/`all`) read a per-request
  task-local set by the `capture_request` middleware. Called **outside** a
  `capture_request` scope they **panic** with a clear message ("add the
  `capture_request` middleware").

- **`SessionFacade`** reads the current client's session via a `session_scope`
  task-local. Without `session_scope` it **panics immediately**.

- **The `Auth` facade** (`user()`, `check()`, `guest()`, `id()`, `login()`,
  `logout()`, ...) **panics** when called with no `require_auth` /
  `with_auth_scope` established — every request-state method fails fast, and there
  is no process-global fallback. Inside a scope with no logged-in user, `check()`
  is `false`, `guest()` is `true`, `user()`/`id()` are `None` (a legitimate guest).
  (`Auth::set_provider()` is the one intentional outside-scope call — startup-only
  provider config, not per-request state.)

- The always-safe façades (`Cache`, `Mail`, `Storage`, `Queue`, `Event`,
  `Broadcast`) resolve to process-global singletons and are safe from anywhere,
  including CLI and background tasks.

---

## Path B — Typed DI path (recommended for larger / team / testable apps)

The typed DI path makes every dependency explicit in the function signature. It
uses standard Rust composition: traits, `Arc<dyn Trait>`, and axum's
`State<AppState>` extractor. No global state is read anywhere in the handler
body.

This is the path the review explicitly called out as deserving equal, first-class
status. It is recommended whenever testability, dependency-visibility, or team
scale matters.

**Canonical example:** [`examples/typed-service`](../examples/typed-service/src/main.rs)

### What it looks like

```rust
// Every dependency is visible in the signature.  The compiler enforces them.
pub async fn create_post(
    State(state): State<AppState>,
    ValidatedJson(input): ValidatedJson<CreatePost>,
) -> Result<Json<Post>, AppError> {
    let post = state.post_service.create(input).await?;
    Ok(Json(post))
}
```

All of `AppState`, `PostService`, and `CreatePost` are injected; the handler
body calls through a trait. Swapping the real database for a mock in a test
requires zero changes to this function.

### The DI container pattern

```rust
#[derive(Clone)]
pub struct AppState {
    pub post_service: Arc<dyn PostService>,  // injected at startup
}

// Wired at startup with the real implementation:
let db = Arc::new(DatabaseManager::connect(DatabaseConfig::default()).await?);
let state = AppState {
    post_service: Arc::new(DbPostService::new(db)),
};
let app = build_router(state);
```

The router factory accepts state, so tests can inject a completely different
implementation (see §Testability comparison below).

### When to use this path

- **Team applications** where multiple developers work in the same codebase and
  dependency visibility helps during code review.
- **Any code that needs unit tests** without spinning up real services.
- **Library or shared middleware code** that may run outside an HTTP-request
  scope.
- **Background tasks, CLI commands, or queued jobs** where no HTTP request
  exists.
- **Handlers where compile-time proof of field presence matters** — a missing
  `ValidatedJson` field is a compile error, not a runtime `None`.

### ORM status — `DatabaseManager` IS instance-based

`rf_orm::DatabaseManager` uses an owned SeaORM pool and is fully injectable:

```rust
// Instance-based — no global state involved:
let db = Arc::new(DatabaseManager::connect(config).await?);
let conn = db.connection();   // &DatabaseConnection — ordinary method call
```

The `DB` / `GLOBAL_DB` constants in `rf_orm::facade` are a **separate,
optional convenience layer** backed by a different driver (rusqlite). They are
NOT required and NOT used by `DatabaseManager`. The typed DI path is
first-class end-to-end: from the HTTP handler all the way through to the
database layer.

---

## Testability comparison — the core difference

This is the central reason the review called for equal first-class status: the
two paths have fundamentally different testing stories.

### Typed DI path — trivial unit testing, zero global setup

From `examples/typed-service/src/main.rs`:

```rust
// A zero-dependency mock — a plain struct implementing the service trait.
struct MockPostService {
    next_id: AtomicI64,
}

#[async_trait]
impl PostService for MockPostService {
    async fn create(&self, input: CreatePost) -> Result<Post, AppError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        Ok(Post { id, title: input.title, body: input.body })
    }
    async fn list(&self) -> Result<Vec<Post>, AppError> { Ok(vec![]) }
}

// Unit test: no database, no server, no global state, no middleware.
#[tokio::test]
async fn test_mock_service_create() {
    let state = AppState {
        post_service: Arc::new(MockPostService::new()),
    };
    let post = state.post_service.create(CreatePost {
        title: "Hello DI".into(),
        body: "no global state was harmed".into(),
    }).await.unwrap();
    assert_eq!(post.id, 1);
}
```

The test is 15 lines. It does not touch the network, a database, Redis, or any
global singleton. It compiles and runs in milliseconds. Any test runner can
parallelize it freely.

### DX façade path — global setup required

To test a handler that calls `Auth::user()` or `Cache::get(...)` you must
initialise the global singletons (configure a real or in-process Redis, wire
the auth store, etc.) and, for `input()`/`has()`, construct a fake HTTP request
and run it through the `capture_request` middleware. This is not impossible, but
it is significantly heavier than the typed DI approach.

**Bottom line:** if unit-testability without global setup is a requirement,
choose the typed DI path. If rapid iteration and minimal boilerplate matter
more, choose the DX façade path.

---

## Honest trade-offs

| Concern | DX façades (Path A) | Typed DI (Path B) |
|---|---|---|
| Lines of code for a simple handler | Fewest (argument-less, façades) | More (typed extractors, explicit `Result`) |
| Familiar to Laravel developers | Yes — the point | Less (standard axum/Rust patterns) |
| Compile-time guarantee of field presence | No — `None` at runtime | Yes — typed extractor |
| Works outside a request scope (tests, CLI, jobs) | No — fails fast (loud panic) | Yes |
| Requires middleware in router | Yes (`capture_request`, `session_scope`, ...) | No |
| **Unit-testable without global setup** | **No — globals require real or mocked singletons** | **Yes — swap `Arc<dyn Service>` for a mock struct** |
| **Dependency visibility (code review / audit)** | **Implicit — hidden in global state** | **Explicit — visible in every function signature** |
| ORM layer — instance-based? | Optional (façade uses global rusqlite) | Yes — `DatabaseManager` is fully injectable (SeaORM pool) |

---

## Mixing the two paths

The paths coexist in the same router — mix them per handler. The
`capture_request` middleware buffers and re-inserts the body so the DX globals
and a downstream `axum::Json<T>` / `ValidatedJson<T>` extractor can both read
the same body in the same request.

```rust
// Mixed handler: typed extractor + DX global in the same function.
async fn create(
    ValidatedJson(body): ValidatedJson<MyDto>,
) -> impl axum::response::IntoResponse {
    let name = body.name;                       // typed: compile-time guaranteed
    let page: Option<usize> = input("page");    // DX global: reads same request
    axum::Json(serde_json::json!({ "name": name }))
}

let app = Router::new()
    .route("/items", post(create))
    .layer(middleware::from_fn(capture_request));  // enables DX globals
```

A common real-world pattern is to use typed `AppState` + injected services for
business logic, while still using DX request helpers (`input`, `file`) for
reading loose extra parameters.

---

## Choosing a path — quick guide

```
Is this a larger app, a team project, or does testability matter?
  YES → Typed DI path (Path B)
       - define service traits
       - inject Arc<dyn Service> into AppState
       - handlers take State<AppState> + typed extractors
       - return Result<_, AppError>
       - unit-test by injecting a mock

Is this a small app, a quick prototype, or a Laravel migration?
  YES → DX façade path (Path A)
       - use Model!, validate!, input(), Auth::, Cache:: etc.
       - add capture_request + session_scope middleware
       - fewest lines of code, Laravel-familiar
       - note: unit tests require global setup (see trade-offs above)
```

---

## Summary

- **DX façade path (Path A)** — `Model!`, `validate!`, `input()`/`file()`/
  `has()`, the `Auth`/`Cache`/`Mail`/`Storage`/`Queue` façades. Fewest lines,
  Laravel-familiar, best for small apps and rapid development. Unit-testing
  handlers requires global setup; some correctness surfaces at runtime, not
  compile time.

- **Typed DI path (Path B)** — `State<AppState>`, `Arc<dyn Service>`, typed
  extractors, `Result`-returning handlers. Explicit dependencies, compile-time
  safe, trivially unit-testable with mock implementations (zero global setup).
  The right default for team apps, anything that needs a test suite, and code
  that runs outside an HTTP-request scope. The ORM layer is fully
  instance-based (`DatabaseManager`). See `examples/typed-service` for the
  canonical example.

Both paths are first-class. Neither is an escape-hatch. Choose based on your
app's scale and what "fast" means to you right now.
