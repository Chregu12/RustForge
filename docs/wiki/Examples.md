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

pub async fn index(
    Query(params): Query<ListQuery>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Response> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(15);

    let cache_key = format!(
        "posts:page:{}:{}:{}",
        page, per_page, params.published.unwrap_or(true)
    );

    // `Cache::remember` is SYNC (the closure is async, the call is not).
    let posts: Vec<Post> = Cache::remember(&cache_key, Duration::from_secs(300), || async {
        let mut query = Post::find();

        if let Some(published) = params.published {
            query = query.filter(post::Column::Published.eq(published));
        }

        Ok(query
            .order_by_desc(post::Column::CreatedAt)
            .all(&db)
            .await?)
    })?;

    Ok(Response::json(&posts))
}

pub async fn show(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Extension(queue): Extension<QueueManager>,
) -> Result<Response> {
    let post = Post::find_by_id(id)
        .one(&db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    // Dispatch a job (sync free fn, returns the job's Uuid).
    dispatch(&queue, IncrementViewsJob { post_id: id })?;

    Ok(Response::json(&post))
}

pub async fn store(
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<Response> {
    payload.validate()?;

    // `Auth::id()` is SYNC and returns Option<u64>.
    let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let post = post::ActiveModel {
        user_id: Set(user_id as i32),
        title: Set(payload.title.clone()),
        slug: Set(Model::generate_slug(&payload.title)),
        content: Set(payload.content),
        published: Set(payload.published.unwrap_or(false)),
        ..Default::default()
    }
    .insert(&db)
    .await?;

    // Clear tagged cache. `Cache::tags(...)` is sync; the returned TaggedCache is async.
    Cache::tags(&["posts"]).flush().await?;

    Ok(Response::json(&post).status(StatusCode::CREATED))
}

pub async fn update(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
    Json(payload): Json<UpdatePostRequest>,
) -> Result<Response> {
    payload.validate()?;

    let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let post = Post::find_by_id(id)
        .one(&db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    if post.user_id != user_id as i32 {
        return Err(anyhow::anyhow!("Not authorized").into());
    }

    let mut active: post::ActiveModel = post.into();
    if let Some(title) = payload.title {
        active.slug = Set(Model::generate_slug(&title));
        active.title = Set(title);
    }
    if let Some(content) = payload.content {
        active.content = Set(content);
    }
    if let Some(published) = payload.published {
        active.published = Set(published);
    }

    let post = active.update(&db).await?;

    Cache::tags(&["posts"]).flush().await?;

    Ok(Response::json(&post))
}

pub async fn destroy(
    Path(id): Path<i32>,
    Extension(db): Extension<DatabaseConnection>,
) -> Result<Response> {
    let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let post = Post::find_by_id(id)
        .one(&db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Post not found"))?;

    if post.user_id != user_id as i32 {
        return Err(anyhow::anyhow!("Not authorized").into());
    }

    post.delete(&db).await?;

    Cache::tags(&["posts"]).flush().await?;

    Ok(Response::no_content())
}
```

> **Note:** `Cache::remember`, `Cache::tags`, and `Auth::id` are all **synchronous** — there is
> no `.await` on the facade call itself (`TaggedCache::flush` returned by `Cache::tags` *is*
> async). `Response::json` takes a reference (`&post`) and `.status(...)` takes a
> `StatusCode`, not an integer.

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
    let db = Database::connect(&std::env::var("DATABASE_URL")?).await?;

    // Public routes using Laravel-style Route facade
    Route::get("/posts", controllers::post_controller::index);
    Route::get("/posts/:id", controllers::post_controller::show);

    // Protected routes
    Route::middleware(&["auth"]).group(|| {
        Route::post("/posts", controllers::post_controller::store);
        Route::put("/posts/:id", controllers::post_controller::update);
        Route::delete("/posts/:id", controllers::post_controller::destroy);
    });

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

pub async fn register(
    Json(payload): Json<RegisterRequest>,
    db: Database,
) -> Result<Response> {
    payload.validate()?;

    // Check if email exists
    let existing = User::find()
        .filter(User::Column::Email.eq(&payload.email))
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(anyhow::anyhow!("Email already registered").into());
    }

    // Hash password — `Hash::make` returns a String directly (no Result, no `?`).
    let password_hash = Hash::make(&payload.password);

    // Create user
    let user = User::ActiveModel {
        email: Set(payload.email.clone()),
        name: Set(payload.name),
        password: Set(password_hash),
        email_verified_at: Set(None),
        ..Default::default()
    };

    let user = user.insert(&db).await?;

    // Send verification email using a Mailable (see the Email System section).
    Mail::send(VerifyEmail { user: user.clone() })?;

    // Login using the Auth facade. `Auth::login` is SYNC and returns Result<(), String>.
    Auth::login(user.clone()).map_err(|e| anyhow::anyhow!(e))?;

    Ok(Response::json(&AuthResponse {
        message: "Registration successful".to_string(),
        user
    }).status(StatusCode::CREATED))
}

pub async fn login(
    Json(payload): Json<LoginRequest>,
    db: Database,
) -> Result<Response> {
    payload.validate()?;

    let user = User::find()
        .filter(User::Column::Email.eq(&payload.email))
        .one(&db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Invalid credentials"))?;

    // `Hash::check` returns a bool directly (no Result, no `?`).
    if !Hash::check(&payload.password, &user.password) {
        return Err(anyhow::anyhow!("Invalid credentials").into());
    }

    // Login using the Auth facade (sync, returns Result<(), String>).
    Auth::login(user.clone()).map_err(|e| anyhow::anyhow!(e))?;

    Ok(Response::json(&AuthResponse {
        message: "Login successful".to_string(),
        user
    }))
}

pub async fn logout() -> Result<Response> {
    // `Auth::logout` is SYNC and returns unit — no `.await`, no `?`.
    Auth::logout();
    Ok(Response::json(&json!({ "message": "Logged out successfully" })))
}

pub async fn me() -> Result<Response> {
    // `Auth::user::<T>()` is SYNC and returns Option<T>.
    if let Some(user) = Auth::user::<User>() {
        Ok(Response::json(&user))
    } else {
        Err(anyhow::anyhow!("Not authenticated").into())
    }
}

pub async fn forgot_password(
    Json(payload): Json<ForgotPasswordRequest>,
    db: Database,
) -> Result<Response> {
    payload.validate()?;

    let user = User::find()
        .filter(User::Column::Email.eq(&payload.email))
        .one(&db)
        .await?;

    if let Some(user) = user {
        // Generate reset token
        let token = uuid::Uuid::new_v4().to_string();

        // Store token
        PasswordReset::ActiveModel {
            email: Set(user.email.clone()),
            token: Set(token.clone()),
            created_at: Set(Utc::now()),
            ..Default::default()
        }
        .insert(&db)
        .await?;

        // Send email via a Mailable. `Mail::to(addr).send(mailable)` is sync
        // (returns MailResult<()>); the closure inside a Mailable does the work.
        Mail::to(&user.email)
            .send(PasswordResetMail { user: user.clone(), token: token.clone() })?;
    }

    Ok(Response::json(&json!({
        "message": "If the email exists, a reset link will be sent"
    })))
}
```

---

## File Upload

Handle file uploads with validation and storage.

RustForge uses axum's `Multipart` extractor for uploads, and the `Storage` facade
(`rf::Storage` = `rf_storage::StorageFacade`) to persist the bytes. Every `Storage`
facade call is **synchronous** — no `.await`. `Storage::exists` returns a bool, and
`Storage::put` takes the path plus a `Vec<u8>` and returns `Result<(), String>`.

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
impl Job for ProcessOrderJob {
    // `handle` takes `JobContext` BY VALUE and returns `JobResult`.
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Processing order {}", self.order_id));

        // Load order, process payment, update inventory... (app-specific)
        let order = load_order(self.order_id).await?;

        // Send confirmation email via a Mailable (sync facade call).
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

// Dispatch jobs. `dispatch`/`dispatch_later` are sync free functions that take a
// `&QueueManager` and return the job's `Uuid`. There is NO `Queue::push` facade.
pub async fn checkout(
    Extension(queue): Extension<QueueManager>,
    Json(payload): Json<CheckoutRequest>,
) -> Result<Response> {
    let user_id = Auth::id().ok_or_else(|| anyhow::anyhow!("Not logged in"))?;

    let order = Order::create(user_id, payload).await?;

    // Queue immediate processing.
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

// Usage
let order = Order::find_by_id(123).one(&db).await?.unwrap();
let customer = User::find_by_id(order.user_id).one(&db).await?.unwrap();

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

impl PostRepository {
    pub async fn find_by_id(id: i32, db: &Database) -> Result<Option<Post>> {
        let cache_key = format!("post:{}", id);

        // `Cache::get` is SYNC — no `.await` on the facade call.
        if let Some(post) = Cache::get::<Post>(&cache_key)? {
            return Ok(Some(post));
        }

        // Query database
        let post = Post::find_by_id(id).one(db).await?;

        // Cache result with tags. `Cache::tags(...)` is sync; the returned
        // `TaggedCache::set` IS async.
        if let Some(ref post) = post {
            let tagged = Cache::tags(&["posts"]);
            tagged.set(&cache_key, post, Duration::from_secs(3600)).await?;
        }

        Ok(post)
    }

    pub async fn all_published(db: &Database) -> Result<Vec<Post>> {
        // `Cache::remember` is SYNC (only the closure is async). No trailing `.await`.
        let posts = Cache::remember("posts:published", Duration::from_secs(300), || async {
            Ok(Post::find()
                .filter(Post::Column::Published.eq(true))
                .order_by_desc(Post::Column::CreatedAt)
                .all(db)
                .await?)
        })?;

        Ok(posts)
    }

    pub async fn update(id: i32, data: UpdatePost, db: &Database) -> Result<Post> {
        let post = /* update logic */;

        // Invalidate cache. `Cache::forget` and `Cache::tags` are sync;
        // `TaggedCache::flush` is async.
        Cache::forget(&format!("post:{}", id))?;
        Cache::tags(&["posts"]).flush().await?;

        Ok(post)
    }
}
```

> **Note:** The `Cache` facade (`rf::Cache` / `rf_cache::CacheFacade`) is **synchronous** — its
> methods return `CacheResult<T>` directly and are NOT `async`. Use `Cache::get("k")?` rather
> than `Cache::get("k").await?`. (`TaggedCache` returned by `Cache::tags(...)` is async internally
> but `Cache::tags(...)` itself is synchronous.)

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

#[Object]
impl Query {
    async fn posts(&self, ctx: &Context<'_>) -> Result<Vec<Post>> {
        let db = ctx.data::<Database>()?;
        Ok(Post::find().all(db).await?)
    }

    async fn post(&self, ctx: &Context<'_>, id: ID) -> Result<Option<Post>> {
        let db = ctx.data::<Database>()?;
        let id: i32 = id.parse()?;
        Ok(Post::find_by_id(id).one(db).await?)
    }
}

struct Mutation;

#[Object]
impl Mutation {
    async fn create_post(
        &self,
        ctx: &Context<'_>,
        title: String,
        content: String,
    ) -> Result<Post> {
        let db = ctx.data::<Database>()?;
        let auth = ctx.data::<AuthGuard>()?;

        let post = Post::ActiveModel {
            user_id: Set(auth.user_id()),
            title: Set(title),
            content: Set(content),
            ..Default::default()
        };

        let post = post.insert(db).await?;

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
