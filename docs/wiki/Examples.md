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
use std::time::Duration;
use serde::Deserialize;
use crate::requests::post_request::*;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    page: Option<usize>,
    per_page: Option<usize>,
    published: Option<bool>,
}

pub async fn index(Query(params): Query<ListQuery>) -> Result<Response, Error> {
    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(15);

    let cache_key = format!("posts:page:{}:{}:{}", page, per_page, params.published.unwrap_or(true));

    let posts = Cache::remember(&cache_key, Duration::from_secs(300), || async {
        let mut query = DB::table("posts");

        if let Some(published) = params.published {
            query = query.r#where("published", published);
        }

        Ok(query
            .order_by_desc("created_at")
            .paginate(per_page, page)
            .await?)
    }).await?;

    Ok(Response::json(posts))
}

pub async fn show(Path(id): Path<i32>) -> Result<Response, Error> {
    let post = DB::table("posts")
        .find(id).await?
        .ok_or_else(|| Error::NotFound("Post not found".into()))?;

    Queue::push(IncrementViewsJob { post_id: id }).await?;

    Ok(Response::json(post))
}

pub async fn store(Json(payload): Json<CreatePostRequest>) -> Result<Response, Error> {
    payload.validate()?;

    let user_id = Auth::id().await.ok_or(Error::Unauthorized("Not logged in".into()))?;

    let post = DB::table("posts").create(json!({
        "user_id": user_id,
        "title": payload.title,
        "slug": generate_slug(&payload.title),
        "content": payload.content,
        "published": payload.published.unwrap_or(false)
    })).await?;

    // Clear cache
    Cache::tags(&["posts"]).await.flush().await?;

    // Dispatch event
    Event::dispatch("post.created", json!({"post_id": id})).await?;

    Ok(Response::json(post).status(201))
}

pub async fn update(Path(id): Path<i32>, Json(payload): Json<UpdatePostRequest>) -> Result<Response, Error> {
    payload.validate()?;

    let user_id = Auth::id().await.ok_or(Error::Unauthorized("Not logged in".into()))?;

    let post = DB::table("posts").find(id).await?
        .ok_or_else(|| Error::NotFound("Post not found".into()))?;

    if post["user_id"] != user_id {
        return Err(Error::Forbidden("Not authorized".into()));
    }

    let mut update_data = json!({});
    if let Some(title) = payload.title {
        update_data["title"] = json!(title);
        update_data["slug"] = json!(generate_slug(&title));
    }
    if let Some(content) = payload.content {
        update_data["content"] = json!(content);
    }
    if let Some(published) = payload.published {
        update_data["published"] = json!(published);
    }

    DB::table("posts")
        .r#where("id", id)
        .update(update_data).await?;

    let post = DB::table("posts").find(id).await?;

    Cache::tags(&["posts"]).await.flush().await?;

    Ok(Response::json(post))
}

pub async fn destroy(Path(id): Path<i32>) -> Result<Response, Error> {
    let user_id = Auth::id().await.ok_or(Error::Unauthorized("Not logged in".into()))?;

    let post = DB::table("posts").find(id).await?
        .ok_or_else(|| Error::NotFound("Post not found".into()))?;

    if post["user_id"] != user_id {
        return Err(Error::Forbidden("Not authorized".into()));
    }

    DB::table("posts")
        .r#where("id", id)
        .delete().await?;

    Cache::tags(&["posts"]).await.flush().await?;

    Ok(Response::no_content())
}
```

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
) -> Result<Response, Error> {
    payload.validate()?;

    // Check if email exists
    let existing = User::find()
        .filter(User::Column::Email.eq(&payload.email))
        .one(&db)
        .await?;

    if existing.is_some() {
        return Err(Error::BadRequest("Email already registered".into()));
    }

    // Hash password
    let password_hash = Hash::make(&payload.password)?;

    // Create user
    let user = User::ActiveModel {
        email: Set(payload.email.clone()),
        name: Set(payload.name),
        password: Set(password_hash),
        email_verified_at: Set(None),
        ..Default::default()
    };

    let user = user.insert(&db).await?;

    // Send verification email using Mail facade
    Mail::to(&user.email)
        .subject("Verify your email")
        .view("emails.verify", json!({ "user": &user }))
        .queue()
        .await?;

    // Login using Auth facade (like Laravel!)
    Auth::login(user.clone()).await?;

    Ok(Response::json(AuthResponse {
        message: "Registration successful".to_string(),
        user
    }).status(201))
}

pub async fn login(
    Json(payload): Json<LoginRequest>,
    db: Database,
) -> Result<Response, Error> {
    payload.validate()?;

    let user = User::find()
        .filter(User::Column::Email.eq(&payload.email))
        .one(&db)
        .await?
        .ok_or_else(|| Error::Unauthorized("Invalid credentials".into()))?;

    if !Hash::check(&payload.password, &user.password)? {
        return Err(Error::Unauthorized("Invalid credentials".into()));
    }

    // Login using Laravel-style Auth facade
    Auth::login(user.clone()).await?;

    Ok(Response::json(AuthResponse {
        message: "Login successful".to_string(),
        user
    }))
}

pub async fn logout() -> Result<Response, Error> {
    // Logout using Auth facade
    Auth::logout().await;
    Ok(Response::json(json!({ "message": "Logged out successfully" })))
}

pub async fn me() -> Result<Response, Error> {
    // Get current user using Auth facade
    if let Some(user) = Auth::user::<User>().await {
        Ok(Response::json(user))
    } else {
        Err(Error::Unauthorized("Not authenticated".into()))
    }
}

pub async fn forgot_password(
    Json(payload): Json<ForgotPasswordRequest>,
    db: Database,
) -> Result<Response, Error> {
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

        // Send email
        Mail::to(&user.email)
            .subject("Password Reset")
            .view("emails.reset_password", json!({
                "user": &user,
                "token": &token
            }))
            .send()
            .await?;
    }

    Ok(Response::json(json!({
        "message": "If the email exists, a reset link will be sent"
    })))
}
```

---

## File Upload

Handle file uploads with validation and storage.

```rust
// NOTE: there is no `rf_http` crate. `Request` lives in `rf_request`, `Response` in
// `rf_response` (both re-exported via `rf::web::*`). A `Multipart` extractor is not yet
// exported by any rf-* crate — this upload example is illustrative/aspirational.
use rf_request::Request;
use rf_response::Response;
use rf_storage::StorageFacade as Storage;
use rf_validation::Validate;

#[derive(Debug, Validate)]
pub struct UploadRequest {
    #[validate(file(mime = "image/*", max_size = 5242880))] // 5MB
    pub image: File,

    pub title: String,
}

pub async fn upload_image(
    auth: AuthGuard,
    mut multipart: Multipart,
) -> Result<Response, Error> {
    let mut title = None;
    let mut file = None;

    while let Some(field) = multipart.next_field().await? {
        let name = field.name().unwrap_or_default();

        match name {
            "title" => {
                title = Some(field.text().await?);
            }
            "image" => {
                let filename = field.file_name()
                    .ok_or_else(|| Error::BadRequest("No filename".into()))?
                    .to_string();

                let data = field.bytes().await?;

                // Validate file type
                if !filename.ends_with(".jpg") &&
                   !filename.ends_with(".png") {
                    return Err(Error::BadRequest("Only JPG/PNG allowed".into()));
                }

                // Generate unique filename
                let unique_name = format!(
                    "{}/{}/{}",
                    auth.user_id(),
                    Utc::now().timestamp(),
                    filename
                );

                // Store file
                Storage::disk("s3")
                    .put(&format!("uploads/{}", unique_name), data)
                    .await?;

                file = Some(unique_name);
            }
            _ => {}
        }
    }

    let file_path = file.ok_or_else(|| Error::BadRequest("No file uploaded".into()))?;
    let title = title.ok_or_else(|| Error::BadRequest("No title provided".into()))?;

    // Save to database
    let image = Image::ActiveModel {
        user_id: Set(auth.user_id()),
        title: Set(title),
        path: Set(file_path.clone()),
        ..Default::default()
    };

    let image = image.insert(&db).await?;

    // Generate temporary URL
    let url = Storage::disk("s3")
        .temporary_url(&file_path, 3600)
        .await?;

    Ok(Response::json(json!({
        "image": image,
        "url": url
    })).status(201))
}
```

---

## Real-time Chat

WebSocket chat application with broadcasting.

```rust
// NOTE: this example is aspirational and does not match the current API.
// `rf_broadcast` exports `Broadcaster` and `Channel` (with `Channel::presence`),
// not a `Broadcast` facade. There is no `rf_http` crate; `rf_broadcasting` provides
// `WebSocketServer`/`WebSocketConfig` rather than a `WebSocket` extractor + `Message` type.
use rf_broadcast::{Broadcaster, Channel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatMessage {
    pub user_id: i32,
    pub username: String,
    pub message: String,
    pub timestamp: DateTime<Utc>,
}

pub async fn ws_handler(
    ws: WebSocket,
    auth: AuthGuard,
    Path(room_id): Path<String>,
) -> Result<(), Error> {
    let (mut sender, mut receiver) = ws.split();

    // Join room
    let channel = format!("chat.{}", room_id);
    Broadcast::presence(&channel)
        .join(auth.user_id())
        .await?;

    // Listen for messages
    let receiver_task = tokio::spawn(async move {
        while let Some(msg) = receiver.next().await {
            if let Ok(Message::Text(text)) = msg {
                let chat_msg = ChatMessage {
                    user_id: auth.user_id(),
                    username: auth.user().name.clone(),
                    message: text,
                    timestamp: Utc::now(),
                };

                // Broadcast to room
                Broadcast::channel(&channel)
                    .send(&chat_msg)
                    .await?;

                // Save to database
                ChatHistory::ActiveModel {
                    room_id: Set(room_id.clone()),
                    user_id: Set(auth.user_id()),
                    message: Set(chat_msg.message),
                    ..Default::default()
                }
                .insert(&db)
                .await?;
            }
        }
        Ok::<_, Error>(())
    });

    // Send messages to client
    let mut subscriber = Broadcast::subscribe(&channel).await?;
    let sender_task = tokio::spawn(async move {
        while let Ok(msg) = subscriber.recv().await {
            let text = serde_json::to_string(&msg)?;
            sender.send(Message::Text(text)).await?;
        }
        Ok::<_, Error>(())
    });

    tokio::try_join!(receiver_task, sender_task)?;

    // Leave room
    Broadcast::presence(&channel)
        .leave(auth.user_id())
        .await?;

    Ok(())
}
```

---

## Job Queue

Background job processing with email notifications.

```rust
use rf_jobs::{Job, JobContext};
use rf_mail::Mail;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessOrderJob {
    pub order_id: i32,
}

#[async_trait]
impl Job for ProcessOrderJob {
    async fn handle(&self, ctx: &JobContext) -> Result<(), Error> {
        let db = ctx.db();

        // Get order
        let order = Order::find_by_id(self.order_id)
            .one(db)
            .await?
            .ok_or_else(|| Error::NotFound("Order not found".into()))?;

        // Process payment
        let payment_result = PaymentGateway::charge(
            order.amount,
            order.payment_method
        ).await?;

        // Update order status
        let mut order: Order::ActiveModel = order.into();
        order.status = Set("paid".to_string());
        order.payment_id = Set(Some(payment_result.id));
        let order = order.update(db).await?;

        // Update inventory
        for item in order.items {
            Product::Entity::update_many()
                .filter(Product::Column::Id.eq(item.product_id))
                .col_expr(
                    Product::Column::Stock,
                    Expr::col(Product::Column::Stock).sub(item.quantity)
                )
                .exec(db)
                .await?;
        }

        // Send confirmation email
        Mail::to(&order.customer_email)
            .subject("Order Confirmation")
            .view("emails.order_confirmation", json!({
                "order": &order
            }))
            .send()
            .await?;

        // Dispatch follow-up jobs
        Queue::later(
            86400, // 24 hours
            SendFeedbackRequestJob {
                order_id: order.id
            }
        ).await?;

        Ok(())
    }

    fn max_tries(&self) -> u32 {
        3
    }

    fn timeout(&self) -> u64 {
        300 // 5 minutes
    }

    fn backoff(&self) -> Vec<u64> {
        vec![60, 300, 900] // 1min, 5min, 15min
    }
}

// Dispatch job
pub async fn checkout(
    auth: AuthGuard,
    Json(payload): Json<CheckoutRequest>,
    db: Database,
) -> Result<Response, Error> {
    // Create order
    let order = Order::create(auth.user_id(), payload).await?;

    // Queue processing
    Queue::push(ProcessOrderJob {
        order_id: order.id
    }).await?;

    Ok(Response::json(order).status(201))
}
```

> **Note:** There is no `Queue` facade in RustForge. To dispatch jobs use the free functions
> `rf_jobs::dispatch(job)` / `rf_jobs::dispatch_later(delay, job)`, a `SyncQueueManager`, or
> `rf_queue::QueueFacade` (`push` / `push_later`). Also note the `Job` trait signature is
> `async fn handle(&self, ctx: JobContext) -> JobResult` (`JobContext` is taken by value), and
> `Job::backoff()` / `Job::timeout()` both return `std::time::Duration` (not `Vec<u64>` / `u64`).

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

```rust
use rf_mail::{Mailable, Mail};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct OrderConfirmation {
    pub order: Order,
    pub customer: User,
}

impl Mailable for OrderConfirmation {
    fn to(&self) -> Vec<String> {
        vec![self.customer.email.clone()]
    }

    fn subject(&self) -> String {
        format!("Order Confirmation #{}", self.order.id)
    }

    fn view(&self) -> String {
        "emails.order_confirmation".to_string()
    }

    fn data(&self) -> serde_json::Value {
        json!({
            "order": &self.order,
            "customer": &self.customer,
            "items": &self.order.items,
            "total": &self.order.total
        })
    }

    fn attachments(&self) -> Vec<String> {
        vec![
            format!("/tmp/invoices/order_{}.pdf", self.order.id)
        ]
    }
}

// Usage
let order = Order::find_by_id(123).one(&db).await?.unwrap();
let customer = User::find_by_id(order.user_id).one(&db).await?.unwrap();

let email = OrderConfirmation { order, customer };
email.send().await?;

// Or queue it
email.queue().await?;
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

        // Try cache first using Laravel-style facade
        if let Some(post) = Cache::get::<Post>(&cache_key).await? {
            return Ok(Some(post));
        }

        // Query database
        let post = Post::find_by_id(id).one(db).await?;

        // Cache result with tags
        if let Some(ref post) = post {
            let tagged = Cache::tags(&["posts"]).await;
            tagged.set(&cache_key, post, Duration::from_secs(3600)).await?;
        }

        Ok(post)
    }

    pub async fn all_published(db: &Database) -> Result<Vec<Post>> {
        // Use Cache::remember like Laravel
        Cache::remember("posts:published", Duration::from_secs(300), || async {
            Ok(Post::find()
                .filter(Post::Column::Published.eq(true))
                .order_by_desc(Post::Column::CreatedAt)
                .all(db)
                .await?)
        }).await
    }

    pub async fn update(id: i32, data: UpdatePost, db: &Database) -> Result<Post> {
        let post = /* update logic */;

        // Invalidate cache
        Cache::forget(&format!("post:{}", id)).await?;
        Cache::tags(&["posts"]).await.flush().await?;

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

> **Note:** `rf_testing` does not export `TestCase` / `DatabaseTestCase`. The real test
> harness types are `HttpTester`, `TestClient`, `TestResponse`, and `TestDatabase`
> (plus `refresh_database`, `Factory`, `Seeder`). The example below is illustrative; adapt
> the entry points to those types.

```rust
use rf_testing::{HttpTester, TestClient, TestResponse, TestDatabase};

#[tokio::test]
async fn test_post_crud() {
    let mut test = DatabaseTestCase::new().await;

    // Register user
    let user_response = test.post("/auth/register", json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "Password123",
        "password_confirmation": "Password123"
    })).await;

    user_response.assert_status(201);
    let token = user_response.json::<AuthResponse>().token;

    // Create post
    let post_response = test
        .post("/posts", json!({
            "title": "Test Post",
            "content": "This is a test post with enough content to pass validation.",
            "published": true
        }))
        .header("Authorization", format!("Bearer {}", token))
        .await;

    post_response.assert_status(201);
    let post = post_response.json::<Post>();

    // Get post
    let get_response = test.get(&format!("/posts/{}", post.id)).await;
    get_response.assert_status(200);
    get_response.assert_json_contains(json!({
        "title": "Test Post"
    }));

    // Update post
    let update_response = test
        .put(&format!("/posts/{}", post.id), json!({
            "title": "Updated Title"
        }))
        .header("Authorization", format!("Bearer {}", token))
        .await;

    update_response.assert_status(200);

    // Delete post
    let delete_response = test
        .delete(&format!("/posts/{}", post.id))
        .header("Authorization", format!("Bearer {}", token))
        .await;

    delete_response.assert_status(204);

    // Verify deleted
    let verify_response = test.get(&format!("/posts/{}", post.id)).await;
    verify_response.assert_status(404);
}

#[tokio::test]
async fn test_caching() {
    let test = TestCase::new().await;

    // First request (database query)
    let start = Instant::now();
    let response1 = test.get("/posts").await;
    let duration1 = start.elapsed();

    response1.assert_status(200);

    // Second request (from cache)
    let start = Instant::now();
    let response2 = test.get("/posts").await;
    let duration2 = start.elapsed();

    response2.assert_status(200);

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
