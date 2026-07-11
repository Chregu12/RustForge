# RustForge Cookbook

Task-oriented recipes. Each recipe has a one-line intent, a minimal snippet taken
verbatim from a CI-tested example or probe, and a pointer to the source that proves
it. Every API name in every snippet is grep-verified against `crates/`.

Maturity labels follow the README matrix:
**Stable** = probe-verified and used in a CI-tested example.
**Usable** = real core, documented minor gaps.
**Experimental** = compiles and passes isolated tests; not end-to-end on all paths.

---

## Table of contents

1. [REST resource](#1-rest-resource)
2. [Relations and eager loading](#2-relations-and-eager-loading)
3. [Validation](#3-validation)
4. [Auth middleware and Sanctum tokens](#4-auth-middleware-and-sanctum-tokens)
5. [Pagination and search](#5-pagination-and-search)
6. [Sessions and flash](#6-sessions-and-flash)
7. [File upload and storage](#7-file-upload-and-storage)
8. [Background jobs](#8-background-jobs)
9. [WebSocket broadcast](#9-websocket-broadcast)
10. [Mail and notifications](#10-mail-and-notifications)
11. [i18n](#11-i18n)
12. [API resource transformers](#12-api-resource-transformers)
13. [Rate limiting, security headers, and CORS](#13-rate-limiting-security-headers-and-cors)
14. [Health, readiness, and observability](#14-health-readiness-and-observability)
15. [CLI: scaffold, migrate, run](#15-cli-scaffold-migrate-run)
16. [Deploy artifacts](#16-deploy-artifacts)

---

## 1. REST resource

**Intent.** Five-verb CRUD for `Article` with validated creation, eager-loaded
relations, and correct HTTP status codes (201/200/204/404/422). No `Request`
argument, no `Result<_, StatusCode>` in handler signatures.

```rust
use axum::http::StatusCode;
use rf::prelude::*;

Model!(Author { name: String });

Model!(Article {
    title: String,
    body: String,
    author_id: i64,
    belongsTo author: Author,
});

async fn index() -> impl axum::response::IntoResponse {
    match Article::with(&["author"]).get().await {
        Ok(articles) => json(articles),
        Err(e) => json(serde_json::json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn store() -> impl axum::response::IntoResponse {
    if validate! { title: string.max(200), body: string, author_id: int.min(1) }.is_err() {
        return json(serde_json::json!({ "error": "validation failed" }))
            .status(StatusCode::UNPROCESSABLE_ENTITY);
    }
    let title: String = input("title").unwrap_or_default();
    let body:  String = input("body").unwrap_or_default();
    let author_id: i64 = input("author_id").unwrap_or_default();
    match create!(Article, title = title, body = body, author_id = author_id) {
        Ok(created) => json(created).status(StatusCode::CREATED),
        Err(e) => json(serde_json::json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

async fn destroy() -> impl axum::response::IntoResponse {
    let id: i64 = input("id").expect("path param");
    match delete!(Article, id) {
        Ok(0)  => json(serde_json::json!({ "error": "not found" })).status(StatusCode::NOT_FOUND),
        Ok(_)  => Response::no_content(),
        Err(e) => json(serde_json::json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn build_app() -> axum::Router {
    get("/articles",       index);
    post("/articles",      store);
    get("/articles/{id}",  show);
    put("/articles/{id}",  update);
    delete("/articles/{id}", destroy);
    rf::global_router()
        .build_router()
        .layer(axum::middleware::from_fn(rf::web::capture_request))
}
```

Source: `examples/rest-crud-resource/src/main.rs` (Stable — full lifecycle integration test passes in CI).

---

## 2. Relations and eager loading

**Intent.** Declare bidirectional `belongsTo`/`hasMany` pairs, eager-load with
`with().get()`, use dot-path nested loads, and constrain a child relation with
`with_where`.

### 2a. Model declaration

```rust
Model!(Project {
    name: String,
    hasMany tasks: Task,
});

Model!(Task {
    title: String,
    status: String,
    project_id: i64,
    user_id: i64,
    belongsTo project: Project,
    // Foreign-key override: relation named `assignee` but column is `user_id`.
    belongsTo assignee: User (foreign_key = "user_id"),
    scope open: where("status", "open"),
});
```

### 2b. Eager loading

```rust
// All tasks for every project (hasMany, N+1-free).
let projects = Project::with(&["tasks"]).get().await?;

// Both relations on one task (bidirectional + FK override).
let tasks = Task::with(&["project", "assignee"])
    .r#where("id", id)
    .get()
    .await?;

// Nested: project -> tasks -> each task's assignee.
let project = Project::with(&["tasks.assignee"])
    .r#where("id", id)
    .get()
    .await?;

// Constrained: only the project's *open* tasks (constraint on first path segment).
let project = Project::with(&["tasks"])
    .r#where("id", id)
    .with_where("tasks", "status", "open")
    .get()
    .await?;

// Combined nested + constrained: open tasks, each with assignee.
let project = Project::with(&["tasks.assignee"])
    .r#where("id", id)
    .with_where("tasks", "status", "open")
    .get()
    .await?;
```

Source: `examples/taskflow/src/main.rs` (Stable — all seven edges asserted in the integration test).

---

## 3. Validation

**Intent.** Use `validate!` for inline rules, the `@` DSL for per-field rules on
the model, and `ValidatedJson` for automatic 422 rejection before the handler runs.

### 3a. Inline validate! macro

```rust
// Inside a handler — validation failure short-circuits to 422.
if validate! {
    title: string.max(200),
    body:  string,
    author_id: int.min(1),
}.is_err() {
    return json(serde_json::json!({ "error": "validation failed" }))
        .status(StatusCode::UNPROCESSABLE_ENTITY);
}
let title: String = input("title").unwrap_or_default();
```

### 3b. Per-field @ DSL with ValidatedJson

```rust
use rf::prelude::*;
use rf_validation::ValidatedJson;

Model!(User {
    validated,
    username: String @ min(3) max(20) alphanumeric,
    email:    String @ email message("Please enter a valid email address"),
    password: String @ min(8),
    zipcode:  String @ regex("^\\d{5}$"),
});

// The ValidatedJson extractor validates before the handler runs.
// Invalid body -> 422 with per-field errors map; never reaches this body.
async fn signup(ValidatedJson(user): ValidatedJson<CreateUser>) -> impl IntoResponse {
    (StatusCode::CREATED, Json(serde_json::json!({
        "username": user.username,
        "email":    user.email,
    })))
}
```

Available `@` rules: `min(N)`, `max(N)`, `email`, `url`, `uuid`, `ip`, `regex("…")`,
`alpha`, `alphanumeric`, `starts_with("…")`, `ends_with("…")`, `range(lo, hi)`,
`message("…")` (custom error text override).

Source: `examples/validated-signup/src/main.rs` and `examples/taskflow/src/main.rs` (Stable).

---

## 4. Auth middleware and Sanctum tokens

### 4a. require_auth — 401 before body extraction

**Intent.** Attach `require_auth` as a route-layer middleware so unauthenticated
requests are rejected with 401 *before* any body extractor runs (401 wins over 422).

```rust
use rf_auth::{require_auth, Auth, UserProvider};
use axum::{routing::post, Router, middleware::from_fn};

// Plug in your user provider once at startup.
Auth::set_provider(std::sync::Arc::new(MyUserProvider));

let app = Router::new().route(
    "/tasks",
    post(create_task).route_layer(from_fn(require_auth)),
);
// No token -> 401; phantom-user bearer -> 401 (verifying login).
// A guest posting an invalid body still gets 401, NOT 422.
```

Source: `sandbox/probes/require_auth_guard/src/main.rs` (Stable).

### 4b. Per-request auth scope

```rust
// Middleware that opens an isolated auth scope per request and bridges a
// bearer token to a verifying login (rejects phantom user ids).
async fn auth_scope_login(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    rf_auth::auth_manager::with_auth_scope(async move {
        if let Some(id) = req.headers()
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .and_then(|t| t.trim().parse::<u64>().ok())
        {
            // Verifying path: only succeeds if the id maps to a real stored user.
            let _ = Auth::login_using_id_verified(id, false);
        }
        next.run(req).await
    }).await
}
```

Source: `examples/taskflow/src/main.rs` (Stable).

### 4c. Sanctum personal-access tokens

**Intent.** Create a DB-backed personal-access token with scoped abilities, look it
up by its SHA-256 hash, check abilities, and revoke it.

```rust
use rf::services::sanctum::{PersonalAccessToken, TokenRepository};

let repo = TokenRepository::new(conn); // `conn: &DatabaseConnection`

// Create a token with abilities.
let new_token = repo.create(
    "User", 42, "mobile-app",
    vec!["posts:read".to_string()],
    None, // no expiry
).await?;
let plaintext = new_token.access_token.clone(); // store and hand to the client

// On every request: hash the bearer and look it up.
let hash   = PersonalAccessToken::hash_token(&plaintext);
let stored = repo.find_by_token(&hash).await?.expect("valid token");
assert!(stored.can("posts:read"));
assert!(!stored.can("posts:write"));

// Revoke.
repo.revoke(stored.id).await?;
// After revoke: find_by_token returns None.
```

Source: `sandbox/probes/sanctum_tokens/src/main.rs` (Stable — real SQLite, SHA-256 hash verified).

---

## 5. Pagination and search

**Intent.** Paginate a model, coerce `?page=` from a query-string string to `usize`,
and search by title using `where_like` (or `where_like_escaped` for user input).

```rust
// GET /tasks?page=2&q=engine
async fn tasks_index() -> impl IntoResponse {
    // Query params arrive as strings; parse explicitly.
    let page: usize = input::<String>("page")
        .and_then(|s| s.parse().ok())
        .unwrap_or(1)
        .max(1);
    const PER_PAGE: usize = 3;

    match Task::paginate(PER_PAGE, page).await {
        Ok(p) => json(serde_json::json!({
            "data":         p.data,
            "total":        p.total,
            "per_page":     p.per_page,
            "current_page": p.current_page,
            "last_page":    p.last_page,
        })).status(StatusCode::OK),
        Err(e) => json(serde_json::json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

// Safe title search with escape of LIKE metacharacters (%, _, \).
async fn tasks_search() -> impl IntoResponse {
    let q: String = input("q").unwrap_or_default();
    // where_like_escaped treats the term as a literal substring, not a pattern.
    match DB::table("tasks")
        .where_like_escaped("title", &q) // escapes % and _ in the user term
        .order_by("id", "asc")
        .get()
        .await
    {
        Ok(rows) => json(rows).status(StatusCode::OK),
        Err(e)   => json(serde_json::json!({ "error": e }))
            .status(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
```

Source: `examples/taskflow/src/main.rs` for `Task::paginate` and query-string coercion;
`sandbox/probes/like_escape_fix/src/main.rs` for `where_like_escaped` (Stable).

---

## 6. Sessions and flash

**Intent.** Per-client isolated session store, one-request flash, and the
`session_scope` middleware that ties a cookie to a request-scoped in-memory session.

```rust
use axum::{Router, routing::{get, post}, extract::Path, middleware};
use rf::core::{session_scope, SessionFacade};
use serde_json::json;

fn app() -> Router {
    Router::new()
        .route("/put/{key}/{val}", post(|Path((k, v)): Path<(String, String)>| async move {
            SessionFacade::put(k, json!(v));
            "ok".to_string()
        }))
        .route("/flash/{key}/{val}", post(|Path((k, v)): Path<(String, String)>| async move {
            SessionFacade::flash(k, json!(v));     // available on next request only
            "ok".to_string()
        }))
        .route("/get/{key}", get(|Path(k): Path<String>| async move {
            match SessionFacade::get(&k) {
                Some(v) => v.as_str().unwrap_or("none").to_string(),
                None    => "none".to_string(),
            }
        }))
        .layer(middleware::from_fn(session_scope)) // issues rf_session cookie
}
```

The `session_scope` middleware:
- mints a new `rf_session` cookie for first-time visitors;
- returns the same session for returning visitors carrying the cookie;
- ensures no cross-client data bleed (each request gets its own scope).
- `flash(k, v)` is readable on the **next** request via `SessionFacade::get(&k)` and
  gone on the request after that.

Source: `sandbox/probes/session_per_client/src/main.rs` (Stable).

---

## 7. File upload and storage

### 7a. Validate an uploaded file

**Intent.** Validate MIME type and byte-size bounds on an uploaded file inside the
`validate!` DSL; no `Request` argument required.

```rust
// Handler runs behind capture_request; validate! reads the current upload.
async fn upload() -> &'static str {
    let result = validate! {
        title:  string.max(50),
        avatar: image.max(mb(5)).min(kb(1)), // must be an image MIME, 1 KB–5 MB
        resume: file.optional,               // any file, or absent
    };
    if result.is_err() { "invalid" } else { "valid" }
}
```

Source: `sandbox/probes/validate_file/src/main.rs` (Stable).

### 7b. Store and retrieve files

**Intent.** Write bytes to local storage; read back; delete.

```rust
use rf::services::storage::{LocalStorage, StorageFacade, Storage};

// Async trait path — explicit LocalStorage instance.
let storage = LocalStorage::new(&root, "http://localhost:3000").await?;
storage.put("uploads/report.pdf", bytes).await?;
let bytes_back = storage.get("uploads/report.pdf").await?;
storage.delete("uploads/report.pdf").await?;

// Sync facade — process-global root (configured once at startup).
StorageFacade::set_root(&base);
StorageFacade::put("uploads/report.txt", bytes)?;
let text = StorageFacade::get_string("uploads/report.txt")?;
let size = StorageFacade::size("uploads/report.txt")?;
StorageFacade::delete("uploads/report.txt")?;
```

Source: `sandbox/probes/storage/src/main.rs` (Stable — real FS writes verified).

---

## 8. Background jobs

**Intent.** Dispatch and run jobs in-process with no Redis required, using the
`rf-queue` crate's `MemoryQueue` and `Worker`.

```rust
use async_trait::async_trait;
use rf_queue::{Job, Jobs, MemoryQueue, Queue, QueueError, Worker};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessUpload { file: String }

#[async_trait]
impl Job for ProcessUpload {
    async fn handle(&self) -> Result<(), QueueError> {
        println!("processing: {}", self.file);
        Ok(())
    }
    fn job_type(&self) -> &'static str { "process_upload" }
}

async fn run() -> Result<(), QueueError> {
    let queue: Arc<dyn Queue> = Arc::new(MemoryQueue::new());

    // Install as the process-global default so Jobs facade works from anywhere.
    Jobs::set_queue(Arc::clone(&queue));

    // Dispatch via facade (no queue handle needed at the call site).
    Jobs::dispatch(ProcessUpload { file: "avatar.png".into() })?;
    // Or via the job itself:
    ProcessUpload { file: "invoice.pdf".into() }.dispatch_now()?;

    // Drain in-process: work_once() returns Ok(false) when the queue is empty.
    let worker = Worker::new(queue).register::<ProcessUpload>();
    while worker.work_once().await? {}
    Ok(())
}
```

Source: `examples/jobs-offline/src/main.rs` (Stable — no Redis, self-contained).

---

## 9. WebSocket broadcast

**Intent.** Mount the framework WebSocket router, broadcast an event to a named
room from any code path, and deliver it only to subscribed clients.

```rust
use axum::Router;
use rf_broadcast::{websocket_router, Broadcaster, Channel, MemoryBroadcaster, SimpleEvent};
use serde_json::json;
use std::sync::Arc;

fn build_app() -> (Router, Arc<MemoryBroadcaster>) {
    let broadcaster = Arc::new(MemoryBroadcaster::new());
    let app = Router::new().merge(websocket_router(Arc::clone(&broadcaster)));
    (app, broadcaster)
}

// From any background task or handler:
let room  = Channel::public("room-1");
let event = SimpleEvent::new(
    "message.posted",
    json!({ "id": 1, "text": "hello" }),
    vec![room.clone()],
);
broadcaster.broadcast(&room, &event).await?;
```

A client subscribes by sending a JSON WebSocket frame:
```json
{"type": "subscribe", "channel": "room-1"}
```

The server sends a `{"type":"subscribed"}` ack frame, then event frames of the form:
```json
{"type": "event", "channel": "room-1", "event": "message.posted", "data": {...}}
```

Source: `examples/realtime-chat/src/main.rs` (Stable — real TCP, three-client isolation test).

---

## 10. Mail and notifications

### 10a. Send mail via facade or FileMailer

**Intent.** Render a Handlebars template and deliver it as a real `.eml` file, or
use the `MailFacade` Laravel-style API.

```rust
use rf::services::mail::{
    Address, FileMailer, MailBuilder, MailFacade, Mailable, TemplateEngine,
};
use serde_json::json;

// --- Facade path (sync, writes a .eml to the configured mailbox). ---
MailFacade::mailbox(&mailbox_path);
MailFacade::to("alice@example.com").send(WelcomeMail {
    to: "alice@example.com".into(),
    name: "Alice".into(),
})?;

// --- Builder path (async, real FileMailer). ---
let mut engine = TemplateEngine::new();
engine.register_template("welcome", "<h1>Hello, {{name}}!</h1>")?;

let message = MailBuilder::new()
    .from(Address::new("noreply@example.com"))
    .to(Address::new("bob@example.com"))
    .subject("Welcome")
    .with_template_engine(engine)
    .view("welcome", json!({ "name": "Bob" }))?
    .build()?;

let mailer = FileMailer::new(&mailbox_path);
let eml_path = mailer.deliver(&message)?; // writes a real RFC 5322 .eml file

// --- Test path: fake() intercepts sends, get_fake() queries what was captured. ---
use rf::services::mail::testing::{fake, get_fake};
let _guard   = fake();
let recorder = get_fake().unwrap();
message_mailable.send(recorder.as_ref()).await?;
recorder.assert_sent_count(1);
recorder.assert_sent(|m| m.subject.contains("Welcome"));
```

Source: `sandbox/probes/mail/src/main.rs` (Stable — `.eml` content verified on disk).

### 10b. Multi-channel notifications

**Intent.** Implement `Notification`, route to Database and Mail channels, persist
a row, and mark it read.

```rust
use rf_notifications::{
    Channel, DatabaseNotification, MailMessage, Notifiable, Notification,
    NotifierBuilder,
};
use rf_notifications::channels::{DatabaseChannel, MailChannel};
use rf_mail::FileMailer;
use std::sync::Arc;

struct User { id: i64, email: String }

impl Notifiable for User {
    fn route_notification_for_database(&self) -> Option<i64>  { Some(self.id) }
    fn route_notification_for_mail(&self)     -> Option<String> { Some(self.email.clone()) }
}

struct InvoicePaid { invoice_id: u64, amount: f64 }

#[async_trait::async_trait]
impl Notification for InvoicePaid {
    fn via(&self) -> Vec<Channel> { vec![Channel::Database, Channel::Mail] }

    async fn to_database(&self) -> Option<DatabaseNotification> {
        Some(DatabaseNotification {
            title:   "Invoice Paid".into(),
            message: format!("Invoice #{} has been paid", self.invoice_id),
            data:    serde_json::json!({ "invoice_id": self.invoice_id }),
        })
    }

    async fn to_mail(&self) -> Option<MailMessage> {
        Some(MailMessage::new()
            .subject("Invoice Paid")
            .line(format!("Invoice #{} has been paid", self.invoice_id)))
    }
}

// Wire the notifier with real channels.
let file_mailer = Arc::new(FileMailer::new(&mailbox));
let notifier = NotifierBuilder::new()
    .channel(Channel::Database, Arc::new(DatabaseChannel::new(conn.clone())))
    .channel(Channel::Mail, Arc::new(MailChannel::new(file_mailer, "no-reply@example.com")))
    .build();

user.notify(InvoicePaid { invoice_id: 7, amount: 99.99 }, &notifier).await?;

// Query and mark read.
let store = DatabaseChannel::new(conn);
let rows  = store.get_unread_notifications(user.id).await?;
store.mark_as_read(rows[0].id).await?;
```

Source: `sandbox/probes/notifications/src/main.rs` (Stable — real SQLite, `.eml` written and asserted).

---

## 11. i18n

**Intent.** Resolve the best-match locale from `?locale=` or `Accept-Language`,
look up a translation, and apply plural rules.

```rust
use std::sync::Arc;
use axum::{extract::{Extension, Query}, routing::get, Json, Router};
use rf_i18n::{AcceptLanguage, I18n, TranslationCatalog};

fn build_i18n() -> Arc<I18n> {
    let en = TranslationCatalog::new("en")
        .add("greeting", serde_json::Value::String("Welcome!".into()))
        .add("items", serde_json::json!({
            "zero": "No items", "one": "1 item", "other": "{{count}} items"
        }));
    let de = TranslationCatalog::new("de")
        .add("greeting", serde_json::Value::String("Willkommen!".into()))
        .add("items", serde_json::json!({
            "one": "1 Artikel", "other": "{{count}} Artikel"
        }));
    Arc::new(I18n::new("en").fallback("en").add_catalog(en).add_catalog(de))
}

// Handler: AcceptLanguage picks the locale; Extension carries the shared I18n.
async fn greet(
    AcceptLanguage(locale): AcceptLanguage,
    Extension(i18n): Extension<Arc<I18n>>,
) -> Json<serde_json::Value> {
    let local   = i18n.for_locale(&locale);
    let message = local.t("greeting", None).unwrap_or_else(|_| "Welcome!".into());
    Json(serde_json::json!({ "locale": locale, "message": message }))
}

async fn items(
    AcceptLanguage(locale): AcceptLanguage,
    Extension(i18n): Extension<Arc<I18n>>,
    Query(params): Query<ItemsQuery>,
) -> Json<serde_json::Value> {
    let count   = params.count.unwrap_or(0);
    let local   = i18n.for_locale(&locale);
    let summary = local.t_plural("items", count).unwrap_or_default();
    Json(serde_json::json!({ "locale": locale, "count": count, "summary": summary }))
}

fn build_app(i18n: Arc<I18n>) -> Router {
    Router::new()
        .route("/greet", get(greet))
        .route("/items", get(items))
        .layer(Extension(i18n))
}
```

Locale resolution order: `?locale=` query param > `Accept-Language` header primary tag > fallback.
Primary tags are normalised (`de-DE` -> `de`). Unknown locales fall back to the configured default.

Source: `examples/i18n-localized-api/src/main.rs` (Stable — 9 assertion tests covering EN/DE/FR and plural rules).

---

## 12. API resource transformers

**Intent.** Shape handler output with a `data` envelope, conditional attributes,
nested related resources, and paginated collections.

```rust
use rf_api_resources::{
    when, Collection, NestedResource, PaginatedCollection, PaginationLinks,
    PaginationMeta, Resource,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
struct UserResource {
    id:          i64,
    name:        String,
    email:       String,
    // Included only when `admin == true`.
    #[serde(skip_serializing_if = "Option::is_none")]
    admin_token: Option<String>,
    // Omitted entirely when not loaded; recurses when loaded.
    #[serde(skip_serializing_if = "is_not_loaded")]
    posts: NestedResource<Vec<PostResource>>,
}

impl Resource for UserResource {}

// Build a resource conditionally:
let resource = UserResource {
    id:          1,
    name:        "Alice".into(),
    email:       "alice@example.com".into(),
    admin_token: when!(is_admin, "secret".to_string()), // None when false
    posts:       NestedResource::loaded(vec![/* ... */]),
};

// Single item wrapped in { "data": { ... } }:
let json_val = resource.wrap("data").to_json().unwrap();

// Paginated collection with meta and links:
let meta  = PaginationMeta::new(/*current_page*/ 2, /*per_page*/ 10, /*total*/ 25);
let links = PaginationLinks::new("/api/users", &meta);
let coll  = PaginatedCollection::new(vec![resource], meta).with_links(links);
let v = serde_json::to_value(&coll).unwrap();
// v["data"], v["meta"]["total"], v["meta"]["last_page"], v["links"]["next"]
```

Source: `sandbox/probes/api_resources/src/main.rs` (Usable — all assertions pass; see README matrix for `rf-api-resources` status).

---

## 13. Rate limiting, security headers, and CORS

### 13a. Rate limiting

**Intent.** In-memory per-key rate limiter with configurable window; non-destructive
`info()` peek; `reset()` / `clear()` for tests.

```rust
use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimiter};

let config  = RateLimitConfig::per_minute(5);
let limiter = MemoryRateLimiter::new(config);

// Check (and consume a slot for) a key.
let result = limiter.check("user:42").await?;
if !result.allowed {
    // result.retry_after == Some(60) for a per-minute window.
    return Err("rate limited");
}

// Non-destructive peek (does NOT consume a slot).
let info = limiter.info("user:42").await?;
println!("remaining: {}", info.remaining);

// Reset one key; clear all keys.
limiter.reset("user:42").await?;
limiter.clear();
```

Source: `sandbox/probes/rate_limiting/src/main.rs` (Stable).

### 13b. Security headers

**Intent.** Opt-in security-header middleware: X-Content-Type-Options, X-Frame-Options,
Referrer-Policy, optional HSTS, optional CSP.

```rust
use axum::Router;
use rf_web::{security_headers_layer, HstsConfig, SecurityHeadersConfig};

// Zero-config (nosniff + DENY + no-referrer).
let app = Router::new()
    .route("/", axum::routing::get(|| async { "OK" }))
    .layer(security_headers_layer(SecurityHeadersConfig::default()));

// With HSTS and a CSP:
let config = SecurityHeadersConfig::new()
    .hsts(HstsConfig { max_age_secs: 31_536_000, include_subdomains: true, preload: false })
    .content_security_policy("default-src 'self'");
let app = Router::new()
    .route("/", axum::routing::get(|| async { "OK" }))
    .layer(security_headers_layer(config));
```

Source: `sandbox/probes/security_headers/src/main.rs` (Stable).

### 13c. CORS

**Intent.** Add the framework CORS layer — wraps tower-http's `CorsLayer` with a
`CorsConfig` struct.

```rust
use axum::Router;
use rf_web::{cors_layer, CorsConfig};
use axum::http::Method;
use std::time::Duration;

// Allow any origin (development default).
let app = Router::new()
    .merge(routes())
    .layer(cors_layer(CorsConfig::default()));

// Specific origins for production:
let config = CorsConfig {
    allowed_origins: vec!["https://app.example.com".to_string()],
    allowed_methods: vec![Method::GET, Method::POST, Method::PUT, Method::DELETE],
    allowed_headers: vec!["content-type".to_string(), "authorization".to_string()],
    max_age: Some(Duration::from_secs(3600)),
};
let app = Router::new()
    .merge(routes())
    .layer(cors_layer(config));
```

Source: `crates/rf-web/src/middleware/cors.rs` (Stable — tests in same file).

---

## 14. Health, readiness, and observability

### 14a. Health and readiness checks

**Intent.** Aggregate liveness and readiness checks; worst-of rollup returns 200
(Healthy/Degraded) or 503 (Unhealthy).

```rust
use async_trait::async_trait;
use rf_health::{CheckResult, HealthCheck, HealthChecker, HealthStatus};
use rf_health::checks::{DiskCheck, MemoryCheck};

// Built-in checks (thresholds as fractions of 1.0; readiness-only by default).
let checker = HealthChecker::new()
    .add_check(MemoryCheck::new(0.8, 0.95)) // warn at 80%, crit at 95%
    .add_check(DiskCheck::new("/", 0.8, 0.95));

// Custom check:
struct MyDbCheck;
#[async_trait]
impl HealthCheck for MyDbCheck {
    fn name(&self) -> &str { "database" }
    async fn check(&self) -> CheckResult {
        // Probe the DB; return healthy/degraded/unhealthy.
        CheckResult::healthy("database")
            .with_metadata("pool_size", serde_json::json!(5))
    }
}

let checker = checker.add_check(MyDbCheck);
let response = checker.check_all().await;

// response.status: Healthy | Degraded | Unhealthy (worst-of rollup)
// response.http_status(): 200 for Healthy/Degraded, 503 for Unhealthy

// Liveness subset only (liveness: true checks):
let live  = checker.check_liveness().await;
// Readiness subset only:
let ready = checker.check_readiness().await;
```

Source: `sandbox/probes/health_checks/src/main.rs` (Stable).

### 14b. Prometheus metrics

**Intent.** Increment the built-in HTTP counter/histogram, define custom metrics,
and render the Prometheus text exposition format.

```rust
use prometheus::{Encoder, TextEncoder};
use rf::services::metrics::{Counter, CustomGauge, Histogram, HTTP_REQUEST_COUNT, HTTP_REQUEST_DURATION};

// Built-in labeled counters and histograms (process-global).
HTTP_REQUEST_COUNT
    .with_label_values(&["GET", "/articles", "200"])
    .inc();
HTTP_REQUEST_DURATION
    .with_label_values(&["GET", "/articles", "200"])
    .observe(0.042);

// Custom counter:
let orders = Counter::new("orders_total", "Total orders processed")?;
orders.inc();
orders.inc_by(4.0);

// Custom gauge:
let queue_depth = CustomGauge::new("queue_depth", "Worker queue depth")?;
queue_depth.set(42.0);

// Custom histogram with a timer:
let latency = Histogram::new("handler_latency_seconds", "Handler latency")?;
let _timer  = latency.start_timer(); // records elapsed on drop

// Render Prometheus text exposition (wire this to GET /metrics):
let encoder = TextEncoder::new();
let mut buf = Vec::new();
encoder.encode(&prometheus::gather(), &mut buf)?;
let text = String::from_utf8(buf)?;
```

Source: `sandbox/probes/metrics_prometheus/src/main.rs` (Stable).

---

## 15. CLI: scaffold, migrate, run

**Intent.** Use the `forge` CLI to generate a model, migration, and controller;
run migrations; start the dev server.

```sh
# Create a new project.
forge new my-app
cd my-app

# Generate a model (Model!() DSL, plural table name auto-derived).
forge make model Post

# Generate a model + its migration in one step.
forge make model Comment --migration

# Generate a migration manually.
forge make migration create_posts_table

# Generate a controller.
forge make controller PostController

# Run all pending migrations.
forge migrate

# Start the dev server (default port 8000).
forge serve
forge serve --port 3001
```

The generated model produces a `Model!(Post { name: String })` with the table
name `posts` (pluralised from the struct name). The generated migration SQL
targets the same plural table name, so `create!` / `find!` / `update!` / `delete!`
work with no hand-editing.

Verify after generation:

```rust
// Compile-time proof that the plural table name matches.
assert_eq!(<Post as rf_db_facade::Model>::TABLE, "posts");
```

Source: `sandbox/probes/scaffold_codegen_verify/src/main.rs` and
`crates/forge-cli/src/main.rs` (Stable — generated code is warning-clean and
compile-verified in CI).

---

## 16. Deploy artifacts

**Intent.** Generate a `Dockerfile`, `docker-compose.yml`, and optional Kubernetes
manifests from the CLI. The generated K8s manifests assume `GET /health/live` and
`GET /health/ready` health-check endpoints (wire them via `rf-health` recipe 14a).

```sh
# Minimal: Dockerfile + docker-compose.yml.
forge deploy generate my-app --port 3001

# With a Postgres service in docker-compose.yml.
forge deploy generate my-app --port 3001 --with-postgres 15

# With Redis.
forge deploy generate my-app --port 3001 --with-redis

# Full Kubernetes manifests in k8s/.
forge deploy generate my-app --port 3001 --kubernetes \
  --image my-app:v1.0.0

# Custom health-check paths (default: /health/live and /health/ready).
forge deploy generate my-app --port 3001 --kubernetes \
  --liveness-path /live --readiness-path /ready
```

What is generated:
- `Dockerfile` — multi-stage Rust build (default toolchain 1.82).
- `docker-compose.yml` — app service, optional Postgres and Redis services.
- `k8s/deployment.yaml` and `k8s/service.yaml` — basic K8s Deployment + Service
  with the liveness/readiness probes wired to your health endpoints.

Source: `crates/forge-cli/src/commands/deploy.rs` and `crates/rf-deploy/src/lib.rs`
(Usable — artifact files are generated; no live cluster integration tested in CI).
