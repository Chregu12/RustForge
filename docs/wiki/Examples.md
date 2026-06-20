# Examples

This page contains practical code examples for common use cases in RustForge.

## Table of Contents

- [REST API](#rest-api)
- [Authentication](#authentication)
- [File Upload](#file-upload)
- [Real-time Chat](#real-time-chat)
- [Job Queue](#job-queue)
- [Email System](#email-system)
- [Caching Strategy](#caching-strategy)
- [GraphQL API](#graphql-api)
- [Testing](#testing)

---

## REST API

Complete CRUD API for a blog application.

### Project Structure

```
src/
├── main.rs
├── models/
│   ├── mod.rs
│   ├── user.rs
│   └── post.rs
├── controllers/
│   ├── mod.rs
│   └── post_controller.rs
└── requests/
    ├── mod.rs
    └── post_request.rs
```

### Models (src/models/post.rs)

```rust
use rf_orm::prelude::*;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub user_id: i32,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub published: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn generate_slug(title: &str) -> String {
        title
            .to_lowercase()
            .replace(" ", "-")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '-')
            .collect()
    }
}
```

### Request Validation (src/requests/post_request.rs)

```rust
use rf_validation::Validate;
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(min = 3, max = 200))]
    pub title: String,

    #[validate(length(min = 50))]
    pub content: String,

    pub published: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[validate(length(min = 3, max = 200))]
    pub title: Option<String>,

    #[validate(length(min = 50))]
    pub content: Option<String>,

    pub published: Option<bool>,
}
```

### Controller (src/controllers/post_controller.rs)

```rust
use rf::prelude::*;
use rf_jobs::{dispatch, QueueManager};
use axum::extract::{Extension, Json, Path, Query};
use axum::http::StatusCode;
use rf_orm::DatabaseConnection;
use std::time::Duration;
use serde::{Deserialize, Serialize};
use crate::requests::post_request::*;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    page: Option<usize>,
    per_page: Option<usize>,
    published: Option<bool>,
}

// `#[auto_await]` once on the impl block: every model/facade call below
// (`remember`, `find`, `all`, `create`, `update_by_id`, `delete`, `flush`,
// `dispatch`, `id`, ...) is awaited for you, so the bodies read await-free.
#[auto_await]
impl PostController {
    pub async fn index(
        Query(params): Query<ListQuery>,
    ) -> Result<Response> {
        let page = params.page.unwrap_or(1);
        let per_page = params.per_page.unwrap_or(15);

        let cache_key = format!(
            "posts:page:{}:{}:{}",
            page, per_page, params.published.unwrap_or(true)
        );

        // `remember` is in the auto-await set — no `.await`.
        let posts: Vec<Post> = Cache::remember(&cache_key, Duration::from_secs(300), || async {
            let mut query = Post::query().order_by_desc("created_at");

            if let Some(published) = params.published {
                query = query.r#where("published", published);
            }

            Ok(query.get()?)
        })?;

        Ok(Response::json(&posts))
    }

    pub async fn show(
        Path(id): Path<i32>,
        Extension(queue): Extension<QueueManager>,
    ) -> Result<Response> {
        let post = Post::find_or_fail(id)?;

        // Dispatch a job (in the auto-await set).
        dispatch(&queue, IncrementViewsJob { post_id: id })?;

        Ok(Response::json(&post))
    }

    pub async fn store(
        Json(payload): Json<CreatePostRequest>,
    ) -> Result<Response> {
        payload.validate()?;

        let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        // Laravel-style create — `create` is in the auto-await set.
        let post = Post::create(json!({
            "user_id": user_id as i32,
            "title": payload.title.clone(),
            "slug": Model::generate_slug(&payload.title),
            "content": payload.content,
            "published": payload.published.unwrap_or(false),
        }))?;

        // Clear tagged cache. `flush` is in the auto-await set.
        Cache::tags(&["posts"]).flush()?;

        Ok(Response::json(&post).status(StatusCode::CREATED))
    }

    pub async fn update(
        Path(id): Path<i32>,
        Json(payload): Json<UpdatePostRequest>,
    ) -> Result<Response> {
        payload.validate()?;

        let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        let post = Post::find_or_fail(id)?;

        if post.user_id != user_id as i32 {
            return Err(anyhow::anyhow!("Not authorized").into());
        }

        let mut changes = json!({});
        if let Some(title) = payload.title {
            changes["slug"] = json!(Model::generate_slug(&title));
            changes["title"] = json!(title);
        }
        if let Some(content) = payload.content {
            changes["content"] = json!(content);
        }
        if let Some(published) = payload.published {
            changes["published"] = json!(published);
        }

        // `update_by_id` is in the auto-await set.
        let post = Post::update_by_id(id, changes)?;

        Cache::tags(&["posts"]).flush()?;

        Ok(Response::json(&post))
    }

    pub async fn destroy(
        Path(id): Path<i32>,
    ) -> Result<Response> {
        let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

        let post = Post::find_or_fail(id)?;

        if post.user_id != user_id as i32 {
            return Err(anyhow::anyhow!("Not authorized").into());
        }

        // `delete` is in the auto-await set.
        post.delete()?;

        Cache::tags(&["posts"]).flush()?;

        Ok(Response::no_content())
    }
}
```

> **Note:** With `#[auto_await]` on the `impl` block you write Laravel-style, await-free code:
> the macro inserts `.await` after model/facade calls like `find_or_fail`, `create`,
> `update_by_id`, `delete`, `remember`, `flush`, `dispatch`, and `id`, and rewrites
> `where(...)` to `r#where(...)`. `Response::json` takes a reference (`&post`) and
> `.status(...)` takes a `StatusCode`, not an integer.

### Routes (src/main.rs)

```rust
mod models;
mod controllers;
mod requests;

use rf::prelude::*;
use rf_orm::Database;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv()?;

    let app = Application::new();
    // `Database::connect` is infra/bootstrap (not in the auto-await set), so it keeps `.await`.
    let db = Database::connect(&std::env::var("DATABASE_URL")?).await?;

    // Public routes using Laravel-style Route facade
    Route::get("/posts", PostController::index);
    Route::get("/posts/:id", PostController::show);

    // Protected routes
    Route::middleware(&["auth"]).group(|| {
        Route::post("/posts", PostController::store);
        Route::put("/posts/:id", PostController::update);
        Route::delete("/posts/:id", PostController::destroy);
    });

    // `app.serve(...)` is bootstrap infra (not in the auto-await set), so it keeps `.await`.
    app.serve(Route::router()).with_database(db).await?;

    Ok(())
}
```

---

## Authentication

Complete authentication system with registration, login, and password reset using Laravel-style facades.

### Auth Controller

```rust
use rf::prelude::*;
use rf_validation::Validate;
use axum::extract::Json;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Validate)]
pub struct RegisterRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 3))]
    pub name: String,

    #[validate(length(min = 8), regex = "^(?=.*[A-Z])(?=.*[0-9]).*$")]
    pub password: String,

    #[validate(confirmed)]
    pub password_confirmation: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user: User,
}

// `#[auto_await]` once on the impl block. Model lookups (`first`, `create`),
// `Mail::send`/`send`, and `Auth::login`/`logout`/`user`/`id` are all in the
// auto-await set, and `where(...)` is rewritten to `r#where(...)`.
#[auto_await]
impl AuthController {
    pub async fn register(
        Json(payload): Json<RegisterRequest>,
    ) -> Result<Response> {
        payload.validate()?;

        // Check if email exists — `first` is in the auto-await set.
        let existing = User::query()
            .r#where("email", &payload.email)
            .first()?;

        if existing.is_some() {
            return Err(anyhow::anyhow!("Email already registered").into());
        }

        // Hash password — `Hash::make` returns a String directly (no Result, no `?`).
        let password_hash = Hash::make(&payload.password);

        // Create user — `create` is in the auto-await set.
        let user = User::create(json!({
            "email": payload.email.clone(),
            "name": payload.name,
            "password": password_hash,
            "email_verified_at": null,
        }))?;

        // Send verification email using a Mailable (see the Email System section).
        Mail::send(VerifyEmail { user: user.clone() })?;

        // Login using the Auth facade — `login` is in the auto-await set.
        Auth::login(user.clone()).map_err(|e| anyhow::anyhow!(e))?;

        Ok(Response::json(&AuthResponse {
            message: "Registration successful".to_string(),
            user
        }).status(StatusCode::CREATED))
    }

    pub async fn login(
        Json(payload): Json<LoginRequest>,
    ) -> Result<Response> {
        payload.validate()?;

        let user = User::query()
            .r#where("email", &payload.email)
            .first()?
            .ok_or_else(|| anyhow::anyhow!("Invalid credentials"))?;

        // `Hash::check` returns a bool directly (no Result, no `?`).
        if !Hash::check(&payload.password, &user.password) {
            return Err(anyhow::anyhow!("Invalid credentials").into());
        }

        Auth::login(user.clone()).map_err(|e| anyhow::anyhow!(e))?;

        Ok(Response::json(&AuthResponse {
            message: "Login successful".to_string(),
            user
        }))
    }

    pub async fn logout() -> Result<Response> {
        // `logout` is in the auto-await set.
        Auth::logout();
        Ok(Response::json(&json!({ "message": "Logged out successfully" })))
    }

    pub async fn me() -> Result<Response> {
        // `user` is in the auto-await set.
        if let Some(user) = Auth::user::<User>() {
            Ok(Response::json(&user))
        } else {
            Err(anyhow::anyhow!("Not authenticated").into())
        }
    }

    pub async fn forgot_password(
        Json(payload): Json<ForgotPasswordRequest>,
    ) -> Result<Response> {
        payload.validate()?;

        let user = User::query()
            .r#where("email", &payload.email)
            .first()?;

        if let Some(user) = user {
            // Generate reset token
            let token = uuid::Uuid::new_v4().to_string();

            // Store token — `create` is in the auto-await set.
            PasswordReset::create(json!({
                "email": user.email.clone(),
                "token": token.clone(),
                "created_at": Utc::now(),
            }))?;

            // Send email via a Mailable — `send` is in the auto-await set.
            Mail::to(&user.email)
                .send(PasswordResetMail { user: user.clone(), token: token.clone() })?;
        }

        Ok(Response::json(&json!({
            "message": "If the email exists, a reset link will be sent"
        })))
    }
}
```

---

## File Upload

Handle file uploads with validation and storage.

RustForge uses axum's `Multipart` extractor for uploads, and the `Storage` facade
(`rf::Storage` = `rf_storage::StorageFacade`) to persist the bytes. Every `Storage`
facade call is **synchronous** — no `.await` — and `Auth::id()` is await-free too, so the
business logic reads await-free without needing `#[auto_await]`. `Storage::exists` returns
a bool, and `Storage::put` takes the path plus a `Vec<u8>` and returns `Result<(), String>`.
(The only remaining `.await` calls are on axum's `Multipart` reader — `next_field`, `text`,
`bytes` — which are transport infra, not RustForge facade calls.)

```rust
use rf::prelude::*;          // brings in Storage, Response, Auth, json!
use axum::extract::Multipart;
use axum::http::StatusCode;

pub async fn upload_image(mut multipart: Multipart) -> Result<Response> {
    let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let mut title = None;
    let mut stored_path = None;

    while let Some(field) = multipart.next_field().await
        .map_err(|e| anyhow::anyhow!(e.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "title" => {
                title = Some(
                    field.text().await.map_err(|e| anyhow::anyhow!(e.to_string()))?,
                );
            }
            "image" => {
                let filename = field
                    .file_name()
                    .ok_or_else(|| anyhow::anyhow!("No filename"))?
                    .to_string();

                if !filename.ends_with(".jpg") && !filename.ends_with(".png") {
                    return Err(anyhow::anyhow!("Only JPG/PNG allowed").into());
                }

                let data = field
                    .bytes()
                    .await
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .to_vec();

                let path = format!(
                    "uploads/{}/{}/{}",
                    user_id,
                    Utc::now().timestamp(),
                    filename
                );

                // Storage facade: synchronous, takes a Vec<u8>, returns Result<(), String>.
                Storage::put(&path, data).map_err(|e| anyhow::anyhow!(e))?;

                stored_path = Some(path);
            }
            _ => {}
        }
    }

    let path = stored_path.ok_or_else(|| anyhow::anyhow!("No file uploaded"))?;
    let title = title.ok_or_else(|| anyhow::anyhow!("No title provided"))?;

    // `Storage::exists` returns a bool — no `?`.
    let exists = Storage::exists(&path);

    Ok(Response::json(&json!({
        "path": path,
        "title": title,
        "stored": exists,
    })).status(StatusCode::CREATED))
}
```

---

## Real-time Chat

Broadcasting in RustForge is built on `rf_broadcast`, which exposes a `Broadcaster`
trait (with the in-memory `MemoryBroadcaster`), a `Channel` type, and `SimpleEvent`.
The synchronous `broadcast`, `subscribe`, and `presence` free functions wrap the async
broadcaster, so the public API reads cleanly.

```rust
use rf_broadcast::{
    broadcast, subscribe, Channel, MemoryBroadcaster, SimpleEvent,
};
use serde_json::json;
use std::sync::Arc;

// A shared broadcaster is created once (e.g. stored in app state).
let broadcaster = Arc::new(MemoryBroadcaster::new());

// Presence channel for a chat room.
let channel = Channel::presence("chat.42");

// Subscribe a connection (the user_id is optional presence info).
subscribe(
    broadcaster.clone(),
    &channel,
    "conn-123".to_string(),
    Some("user-7".to_string()),
)?;

// Broadcast a chat message to everyone on the channel.
let event = SimpleEvent::new(
    "chat.message",
    json!({
        "user_id": 7,
        "username": "alice",
        "message": "Hello, room!",
    }),
    vec![channel.clone()],
);
broadcast(broadcaster.clone(), &channel, &event)?;
```

For the WebSocket transport itself, `rf_broadcasting` provides `WebSocketServer` and
`WebSocketConfig`:

```rust
use rf_broadcasting::{WebSocketConfig, WebSocketServer};

let server = WebSocketServer::new(WebSocketConfig::default());

// Fan a message out to all connections on a channel.
server.registry().broadcast("chat.42", "Hello, room!".to_string()).await?;
```

> **Note:** There is no `Broadcast` facade and no `WebSocket`/`Message` HTTP extractor.
> The real primitives are `rf_broadcast::{Broadcaster, Channel, SimpleEvent}` plus the
> sync `broadcast`/`subscribe`/`presence` helpers, and `rf_broadcasting::WebSocketServer`
> for the transport.

---

## Job Queue

Background job processing with email notifications.

```rust
use rf::prelude::*;
use rf_jobs::{dispatch, dispatch_later, Job, JobContext, JobResult, QueueManager};
use rf::Mail;
use axum::extract::{Extension, Json};
use axum::http::StatusCode;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOrderJob {
    pub order_id: i32,
}

#[async_trait]
#[auto_await]
impl Job for ProcessOrderJob {
    // `handle` takes `JobContext` BY VALUE and returns `JobResult`.
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Processing order {}", self.order_id));

        // Load order (`find_or_fail` is in the auto-await set).
        let order = Order::find_or_fail(self.order_id)?;

        // Send confirmation email via a Mailable — `send` is in the auto-await set.
        Mail::send(OrderConfirmation {
            order: order.clone(),
            customer: order.customer.clone(),
        })?;

        Ok(())
    }

    fn queue(&self) -> &str {
        "orders"
    }

    // Maximum retry attempts (Laravel's `tries`).
    fn max_attempts(&self) -> u32 {
        3
    }

    // `timeout` and `backoff` both return `Duration`.
    fn timeout(&self) -> Duration {
        Duration::from_secs(300) // 5 minutes
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(60)
    }
}

// Dispatch jobs. With `#[auto_await]` on the handler, `Order::create`, `dispatch`,
// and `Auth::id` are all awaited for you. `dispatch`/`dispatch_later` are free
// functions that take a `&QueueManager` and return the job's `Uuid`.
#[auto_await]
pub async fn checkout(
    Extension(queue): Extension<QueueManager>,
    Json(payload): Json<CheckoutRequest>,
) -> Result<Response> {
    let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    // `create` is in the auto-await set.
    let order = Order::create(json!({
        "user_id": user_id,
        "items": payload.items,
    }))?;

    // Queue immediate processing — `dispatch` is in the auto-await set.
    dispatch(&queue, ProcessOrderJob { order_id: order.id })?;

    // Queue a follow-up 24 hours later.
    dispatch_later(
        &queue,
        SendFeedbackRequestJob { order_id: order.id },
        Duration::from_secs(86_400),
    )?;

    Ok(Response::json(&order).status(StatusCode::CREATED))
}
```

> **Note:** There is no `Queue` facade in RustForge. To dispatch jobs use the sync free
> functions `rf_jobs::dispatch(&qm, job)` / `dispatch_to(&qm, job, "queue")` /
> `dispatch_later(&qm, job, delay)`, or the methods on `QueueManager` / `SyncQueueManager`.
> The `Job` trait signature is `async fn handle(&self, ctx: JobContext) -> JobResult`
> (`JobContext` is taken by value), and `Job::backoff()` / `Job::timeout()` both return
> `std::time::Duration`. Retry count comes from `Job::max_attempts() -> u32`.

### Routing jobs to queues with `JobRouter`

Laravel-13-style queue routing lets the application centrally assign a job *type* to a
specific queue at boot, instead of each job hard-coding its queue via `Job::queue`. A route
registered for a type wins over that job's own `queue()` when dispatched:

```rust
use rf_jobs::JobRouter;

// During application boot:
fn register_queue_routes() {
    // Every ProcessOrderJob goes to the "orders" queue
    JobRouter::route::<ProcessOrderJob>("orders");

    // Route to a queue on a specific connection
    JobRouter::route_to::<SendFeedbackRequestJob>("emails", "redis");
}
```

---

## Email System

Mailable classes for reusable email templates.

The `Mailable` trait has a single required method, `fn build(&self) -> MailBuilder`.
`MailBuilder` is a fluent builder (`.to`, `.from`, `.subject`, `.html`/`.text`/`.markdown`,
`.attach`). To send, hand the mailable to the `Mail` facade — `Mail::send(mailable)` and
`Mail::to(addr).send(mailable)` are both synchronous and return `MailResult<()>`.

```rust
use rf_mail::{Address, MailBuilder, Mailable};
use rf::Mail;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct OrderConfirmation {
    pub order: Order,
    pub customer: User,
}

impl Mailable for OrderConfirmation {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .from(Address::new("orders@example.com"))
            .to(Address::new(&self.customer.email))
            .subject(format!("Order Confirmation #{}", self.order.id))
            .markdown(format!(
                "# Thanks, {name}!\n\nYour order **#{id}** totalling ${total:.2} is confirmed.",
                name = self.customer.name,
                id = self.order.id,
                total = self.order.total,
            ))
    }
}

// Usage (inside an `#[auto_await]` fn/impl/mod — `find_or_fail` is awaited for you).
let order = Order::find_or_fail(123)?;
let customer = User::find_or_fail(order.user_id)?;

let email = OrderConfirmation { order, customer };

// Send synchronously (no `.await` on the facade call).
Mail::send(email.clone())?;

// Or address it explicitly:
Mail::to(&email.customer.email).send(email)?;
```

---

## Caching Strategy

Implement cache-aside pattern with Laravel-style Cache facade.

```rust
use rf::Cache;
use std::time::Duration;

pub struct PostRepository;

// `#[auto_await]` once on the impl: `get`, `find`, `all`, `remember`, `forget`,
// and `flush` are all in the auto-await set, so the bodies stay await-free.
#[auto_await]
impl PostRepository {
    pub async fn find(id: i32) -> Result<Option<Post>> {
        let cache_key = format!("post:{}", id);

        // `get` is in the auto-await set.
        if let Some(post) = Cache::get::<Post>(&cache_key)? {
            return Ok(Some(post));
        }

        // Query database — `find` is in the auto-await set.
        let post = Post::find(id)?;

        // Cache result with tags. `TaggedCache::set` is NOT in the auto-await set,
        // so it keeps its `.await`.
        if let Some(ref post) = post {
            let tagged = Cache::tags(&["posts"]);
            tagged.set(&cache_key, post, Duration::from_secs(3600)).await?;
        }

        Ok(post)
    }

    pub async fn all_published() -> Result<Vec<Post>> {
        // `remember` is in the auto-await set; the closure body uses `get` (also in the set).
        let posts = Cache::remember("posts:published", Duration::from_secs(300), || async {
            Ok(Post::query()
                .r#where("published", true)
                .order_by_desc("created_at")
                .get()?)
        })?;

        Ok(posts)
    }

    pub async fn update(id: i32, data: UpdatePost) -> Result<Post> {
        // `update_by_id` is in the auto-await set.
        let post = Post::update_by_id(id, data)?;

        // Invalidate cache — `forget` and `flush` are in the auto-await set.
        Cache::forget(&format!("post:{}", id))?;
        Cache::tags(&["posts"]).flush()?;

        Ok(post)
    }
}
```

> **Note:** Under `#[auto_await]` the `Cache` facade calls (`get`, `remember`, `forget`,
> `flush`) and model lookups (`find`) are awaited for you, so the code reads Laravel-style.
> The one exception here is `TaggedCache::set`, which is not in the auto-await set and keeps
> its explicit `.await`.

### Extending TTL with `Cache::touch`

Extend a key's expiration to `now + ttl` without re-reading or rewriting its value.
Accepts seconds (integer) or a `Duration`, and returns `true` if the key existed:

```rust
use rf::Cache;
use std::time::Duration;

// Refresh a session's TTL on each request (sliding expiration)
fn keep_session_alive(session_id: &str) -> rf_cache::CacheResult<()> {
    let key = format!("session:{session_id}");

    // Laravel style - pass seconds directly...
    let touched = Cache::touch(&key, 3600)?;

    // ...or a Duration
    if !touched {
        Cache::touch(&key, Duration::from_secs(3600))?;
    }

    Ok(())
}
```

---

## GraphQL API

Complete GraphQL API with queries, mutations, and subscriptions.

```rust
use rf_graphql::{Schema, Object, Subscription, Context, ID};
use async_graphql::SimpleObject;

#[derive(SimpleObject)]
struct Post {
    id: ID,
    title: String,
    content: String,
    author: User,
}

struct Query;

// `#[auto_await]` on the resolver impl: `all`, `find`, and `create` are awaited for you.
#[Object]
#[auto_await]
impl Query {
    async fn posts(&self, _ctx: &Context<'_>) -> Result<Vec<Post>> {
        Ok(Post::all()?)
    }

    async fn post(&self, _ctx: &Context<'_>, id: ID) -> Result<Option<Post>> {
        let id: i32 = id.parse()?;
        Ok(Post::find(id)?)
    }
}

struct Mutation;

#[Object]
#[auto_await]
impl Mutation {
    async fn create_post(
        &self,
        ctx: &Context<'_>,
        title: String,
        content: String,
    ) -> Result<Post> {
        let auth = ctx.data::<AuthGuard>()?;

        // `create` is in the auto-await set.
        let post = Post::create(json!({
            "user_id": auth.user_id(),
            "title": title,
            "content": content,
        }))?;

        // Publish subscription event
        SimpleBroker::publish(PostCreated { post: post.clone() });

        Ok(post)
    }
}

struct Subscription;

#[Subscription]
impl Subscription {
    async fn post_created(&self) -> impl Stream<Item = Post> {
        SimpleBroker::<PostCreated>::subscribe().map(|event| event.post)
    }
}

// Schema
let schema = Schema::build(Query, Mutation, Subscription)
    .data(db)
    .finish();

// Route
router.post("/graphql", async move |req: GraphQLRequest| {
    schema.execute(req.into_inner()).await.into()
});
```

---

## Testing

Comprehensive test examples.

`rf_testing` provides `HttpTester` (driven from an axum `Router`) plus `TestResponse`,
`TestClient`, and `TestDatabase`. `HttpTester::get`/`post`/`put`/`delete` are async,
status assertions take a `StatusCode` (not an integer), and `TestResponse::json::<T>()`
is async and borrows `&mut self`.

```rust
use rf_testing::{HttpTester, TestDatabase};
use axum::http::StatusCode;
use serde_json::json;

async fn make_tester() -> HttpTester {
    // Build the application's axum Router however your app exposes it.
    HttpTester::new(crate::build_router())
}

#[tokio::test]
async fn test_post_crud() {
    // Spin up an isolated test database (migrations + cleanup handled by TestDatabase).
    let db = TestDatabase::new().await.unwrap();
    db.migrate().await.unwrap();

    let tester = make_tester().await;

    // Create post
    let mut post_response = tester
        .post(
            "/posts",
            json!({
                "title": "Test Post",
                "content": "This is a test post with enough content to pass validation.",
                "published": true
            }),
        )
        .await
        .assert_status(StatusCode::CREATED);

    // `json::<T>()` is async and borrows the response.
    let post: Post = post_response.json().await;

    // Get post
    tester
        .get(&format!("/posts/{}", post.id))
        .await
        .assert_status(StatusCode::OK)
        .assert_json(json!({ "title": "Test Post" }))
        .await;

    // Update post
    tester
        .put(&format!("/posts/{}", post.id), json!({ "title": "Updated Title" }))
        .await
        .assert_status(StatusCode::OK);

    // Delete post
    tester
        .delete(&format!("/posts/{}", post.id))
        .await
        .assert_status(StatusCode::NO_CONTENT);

    // Verify deleted
    tester
        .get(&format!("/posts/{}", post.id))
        .await
        .assert_status(StatusCode::NOT_FOUND);

    db.cleanup().await.unwrap();
}

#[tokio::test]
async fn test_caching() {
    let tester = make_tester().await;

    // First request (database query)
    let start = Instant::now();
    tester.get("/posts").await.assert_status(StatusCode::OK);
    let duration1 = start.elapsed();

    // Second request (served from cache)
    let start = Instant::now();
    tester.get("/posts").await.assert_status(StatusCode::OK);
    let duration2 = start.elapsed();

    // Cache should be faster
    assert!(duration2 < duration1);
}
```

---

## Next Steps

- **[API Documentation](API-Documentation)** - Detailed API reference
- **[Features](Features)** - Explore all features
- **[Quick Start](Quick-Start)** - Build your first app
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

---

*More examples coming soon! Contribute on [GitHub](https://github.com/Chregu12/RustForge).*
