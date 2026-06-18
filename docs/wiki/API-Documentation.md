# API Documentation

Comprehensive API documentation for RustForge v1.0.0.

## Core Modules

### rf-core & rf-application

`rf-core` provides runtime/context primitives (environment, request context, error
types). Application bootstrapping lives in `rf-application` via `FoundryApp`.

```rust
use rf_application::FoundryApp;

// Build the application from a config value plus the artifact/migration/seed ports.
let app = FoundryApp::builder(config, artifacts, migrations, seeds)
    .with_storage_port(storage)
    .with_cache_port(cache)
    .build()?; // Result<FoundryApp, ApplicationError>

// Dispatch a console command through the registry.
let result = app
    .dispatch("migrate", vec![], ResponseFormat::Text, ExecutionOptions::default())
    .await?;
```

**Key Types:**
- `FoundryApp` - Main application container (`rf_application::FoundryApp`); built via `FoundryApp::builder(config, artifacts, migrations, seeds)` with `with_*_port(...)` methods and `.build() -> Result<FoundryApp, ApplicationError>`
- `Environment` - Environment enum (`rf_core::Environment`: `Development`, `Staging`, `Production`)
- `Config` - Configuration management (lives in `rf-config`, not `rf-core`)
- `AppError` - Core error type (`rf_core::AppError`)

---

### rf-orm

Database ORM based on SeaORM with additional Laravel-like features.

#### Defining Models

```rust
use rf_orm::prelude::*;
use serde::{Serialize, Deserialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub email: String,

    pub name: String,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post::Entity")]
    Posts,
}

impl ActiveModelBehavior for ActiveModel {}
```

#### Query Builder

```rust
// Find all
let users = User::find().all(&db).await?;

// Find by primary key
let user = User::find_by_id(1).one(&db).await?;

// Filtering
let active_users = User::find()
    .filter(User::Column::Active.eq(true))
    .order_by_asc(User::Column::Name)
    .all(&db)
    .await?;

// Pagination
let page = User::find()
    .paginate(&db, 20)
    .fetch_page(0)
    .await?;

// Relationships
let user_with_posts = User::find_by_id(1)
    .find_with_related(Post::Entity)
    .all(&db)
    .await?;
```

#### Creating & Updating

```rust
// Create
let user = User::ActiveModel {
    name: Set("John".to_string()),
    email: Set("john@example.com".to_string()),
    ..Default::default()
};
let user = user.insert(&db).await?;

// Update
let mut user: User::ActiveModel = user.into();
user.name = Set("Jane".to_string());
let user = user.update(&db).await?;

// Delete
user.delete(&db).await?;
```

**Key Types:**
- `Entity` - Database entity
- `Model` - Data model
- `ActiveModel` - Mutable model for insert/update
- `Column` - Table columns enum
- `Relation` - Model relationships

---

### rf-http

HTTP routing and request/response handling.

#### Router (Laravel-style Route Facade)

```rust
use rf::prelude::*;

// Basic routes with Laravel-style facade
Route::get("/", home);
Route::post("/users", create_user);
Route::put("/users/:id", update_user);
Route::delete("/users/:id", delete_user);

// Route parameters
Route::get("/users/:id", |Path(id): Path<i32>| async move {
    // Use id
});

// Route groups with middleware
Route::middleware(&["auth"]).group(|| {
    Route::get("/profile", get_profile);
    Route::post("/logout", logout);
});

// Prefix groups
Route::prefix("/api/v1").group(|| {
    Route::resource("/posts", PostController);
});

// Named routes
Route::get("/dashboard", dashboard).name("dashboard");

// Multiple middleware
Route::middleware(&["auth", "verified", "admin"]).group(|| {
    Route::get("/admin", admin_panel);
});
```

#### Request Handling

```rust
// `Request` comes from rf-request; the Json/Query/Path/Form extractors are
// re-exported from axum (RustForge has no `rf-http` crate).
use rf_request::Request;
use axum::extract::{Json, Query, Path};
use axum::Form;

// JSON body
async fn create_user(Json(payload): Json<CreateUserRequest>) -> Result<Response> {
    // payload is deserialized
}

// Query parameters
async fn list_users(Query(params): Query<ListParams>) -> Result<Response> {
    // params.page, params.limit
}

// Path parameters
async fn get_user(Path(id): Path<i32>) -> Result<Response> {
    // id from URL
}

// Form data
async fn submit_form(Form(data): Form<FormData>) -> Result<Response> {
    // data from form
}

// Request headers
async fn handler(req: Request) -> Result<Response> {
    let auth_header = req.headers().get("Authorization");
}
```

#### Response Building

```rust
use rf_response::Response;
use axum::http::StatusCode;

// JSON response (takes a reference)
Response::json(&data)

// With status code (StatusCode, not an int)
Response::json(&data).status(StatusCode::CREATED)

// Plain text
Response::text("Hello, World!")

// Redirect
Response::redirect("/login")

// No content
Response::no_content()

// Custom headers
Response::json(&data)
    .header("X-Custom", "value")
    .status(StatusCode::OK)
```

**Key Types:**
- `Route` - Laravel-style route facade (`rf_routing::RouteFacade`, re-exported as `rf::Route`)
- `Request` - HTTP request (`rf_request::Request`)
- `Response` - HTTP response (`rf_response::Response`)
- `Json<T>` - JSON extractor (`axum::extract::Json`)
- `Query<T>` - Query param extractor (`axum::extract::Query`)
- `Path<T>` - Path param extractor (`axum::extract::Path`)
- `Form<T>` - Form data extractor (`axum::Form`)

---

### rf-auth

Authentication and authorization with Laravel-style Auth facade.

#### Auth Facade (Laravel-style)

```rust
use rf::{Auth, Hash};

// Login user (like Laravel's Auth::login) - synchronous, no .await
Auth::login(user)?;

// Attempt login with credentials (like Laravel's Auth::attempt)
let credentials = json!({
    "email": "user@example.com",
    "password": "secret"
});
if Auth::attempt(credentials)? {
    println!("Login successful!");
}

// Check if authenticated
if Auth::check() {
    println!("User is logged in");
}

// Check if guest
if Auth::guest() {
    println!("User is not logged in");
}

// Get current user
if let Some(user) = Auth::user::<User>() {
    println!("Welcome, {}", user.name);
}

// Get user ID
if let Some(id) = Auth::id() {
    println!("User ID: {}", id);
}

// Login with remember me
Auth::login_using_id(user_id, true)?;

// Check if via remember
if Auth::via_remember() {
    println!("Logged in via remember token");
}

// Logout (returns unit)
Auth::logout();

// Use specific guard
let api_guard = Auth::guard("api");
if api_guard.check() {
    println!("Authenticated on API guard");
}

// Role checks
if Auth::has_role("admin") {
    println!("User is admin");
}

if Auth::has_any_role(&["admin", "moderator"]) {
    println!("User has elevated privileges");
}
```

#### Password Hashing

```rust
// `Hash` lives in rf-global-helpers and is re-exported as `rf::Hash`.
use rf::Hash;

// Hash password (returns String, not Result)
let hash = Hash::make("password123");

// Verify password (returns bool, not Result)
let is_valid = Hash::check("password123", &hash);
```

#### Protect Routes

```rust
use rf::Route;

// Protect routes with auth middleware
Route::middleware(&["auth"]).group(|| {
    Route::get("/profile", get_profile);
    Route::post("/logout", logout);
});
```

**Key Types:**
- `Auth` - Laravel-style authentication facade
- `Hash` - Password hashing
- `Guard` - Authentication guard

---

### rf-validation

Input validation framework.

#### Validation Rules

```rust
use rf::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 3, max = 50))]
    pub name: String,

    #[validate(length(min = 8), regex = "^(?=.*[A-Z])(?=.*[0-9]).*$")]
    pub password: String,

    #[validate(confirmed)]
    pub password_confirmation: String,

    #[validate(range(min = 18, max = 120))]
    pub age: u8,
}

// Validate
let result = request.validate();
match result {
    Ok(_) => { /* Valid */ }
    Err(errors) => { /* Handle errors */ }
}
```

#### Custom Validators

```rust
use rf_validation::{Rule, RuleResult};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

pub struct UniqueEmail;

#[async_trait]
impl Rule for UniqueEmail {
    fn name(&self) -> &str {
        "unique_email"
    }

    async fn validate(&self, value: &Value, _data: &HashMap<String, Value>) -> RuleResult {
        let email = value.as_str().unwrap_or_default();
        let exists = User::find()
            .filter(User::Column::Email.eq(email))
            .count(&db)
            .await? > 0;

        if exists {
            return Err("Email already registered".to_string());
        }

        Ok(())
    }

    fn message(&self) -> String {
        "Email already registered".to_string()
    }
}
```

**Key Types:**
- `Validate` - Validation derive (`rf_validation_derive::Validate`, re-exported as `rf::Validate`)
- `Rule` - Custom validation rule trait (`rf_validation::Rule`) — named `Rule`, not `Validator`
- `ValidationErrors` - Validation errors collection (`rf_validation::ValidationErrors`)
- `RuleResult` - Rule result (`rf_validation::RuleResult` = `Result<(), String>`)

---

### rf-cache

Caching layer with Laravel-style Cache facade.

#### Basic Usage (Laravel-style)

```rust
use rf::Cache;
use std::time::Duration;

// Put value in cache (synchronous facade - no .await).
// TTL accepts Duration or i64/u64 seconds.
Cache::put("key", "value", Duration::from_secs(3600))?;
Cache::put("key", "value", 3600)?;

// Get value from cache (turbofish the deserialized type)
let value: Option<String> = Cache::get::<String>("key")?;

// Check if key exists (returns bool)
if Cache::has("key")? {
    println!("Key exists");
}

// Remember (cache with closure) - like Laravel's Cache::remember.
// The facade call is synchronous; the closure returns a future.
let users = Cache::remember("users:all", 3600, || async {
    Ok(User::find().all(&db).await?)
})?;

// Remember forever
let settings = Cache::remember_forever("settings", || async {
    Ok(load_settings().await?)
})?;

// Store forever (no expiration)
Cache::forever("key", "value")?;

// Add only if doesn't exist (returns bool)
let added = Cache::add("unique_key", "value", 60)?;

// Pull: get and delete
let value: Option<String> = Cache::pull::<String>("temp_key")?;

// Touch: extend an existing entry's TTL without rewriting its value (returns bool)
let touched = Cache::touch("key", 3600)?;

// Forget (delete single key)
Cache::forget("key")?;

// Flush all cache
Cache::flush()?;

// Increment/decrement counters
Cache::increment("counter", 1)?;
Cache::decrement("counter", 1)?;
```

#### Cache Tags

```rust
use rf::Cache;
use std::time::Duration;

// Create tagged cache (Cache::tags is synchronous; TaggedCache ops are async).
let tagged = Cache::tags(&["users", "posts"]);
tagged.set("key", &"value", Duration::from_secs(3600)).await?;

// Flush all entries with specific tag
Cache::tags(&["users"]).flush().await?;
```

**Key Types:**
- `Cache` - Laravel-style cache facade
- `TaggedCache` - Tagged cache operations

---

### rf-queue & rf-jobs

Background job processing.

#### Defining Jobs

```rust
use rf_jobs::{Job, JobContext, JobResult};
use serde::{Serialize, Deserialize};
use async_trait::async_trait;
use std::time::Duration;

// Jobs must be Clone in addition to Serialize/Deserialize.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Sending email to {}", self.to));
        // Send email via the synchronous Mail facade (no .await).
        Mail::to(&self.to).send(/* a Mailable */)?;

        Ok(())
    }

    fn queue(&self) -> &str {
        "emails"
    }

    fn max_attempts(&self) -> u32 {
        3
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(30)
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }
}
```

#### Dispatching Jobs

```rust
// Dispatch jobs with the synchronous free functions in `rf_jobs`.
// Each takes a `&QueueManager` and returns `Result<Uuid, QueueError>`.
use rf_jobs::{dispatch, dispatch_to, dispatch_later};
use std::time::Duration;

let job = SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Welcome".to_string(),
    body: "Hello!".to_string(),
};

// Dispatch to the job's default queue (Job::queue()).
let id = dispatch(&queue_manager, job.clone())?;

// Dispatch to a specific queue.
let id = dispatch_to(&queue_manager, job.clone(), "emails")?;

// Delayed dispatch.
let id = dispatch_later(&queue_manager, job, Duration::from_secs(300))?;
```

> Per-call queue selection is done via `Job::queue()`, `dispatch_to(..)`, or
> central routing with `JobRouter` (see below). Lower-level
> `rf_queue::QueueFacade::{push, push_later}` is also available when you hold an
> `Arc<dyn Queue>` directly.

**Key Types:**
- `Job` - Job trait (`rf_jobs::Job`); key methods: `handle(&self, ctx: JobContext) -> JobResult`, `queue() -> &str`, `max_attempts() -> u32`, `backoff() -> Duration`, `timeout() -> Duration`, `failed`
- `dispatch` / `dispatch_to` / `dispatch_later` / `dispatch_with_priority` / `dispatch_on` - Synchronous dispatch free functions (`rf_jobs`), each `Result<Uuid, QueueError>`
- `QueueFacade` - Lower-level dispatch facade over `Arc<dyn Queue>` (`rf_queue::QueueFacade`): `push`, `push_later`
- `JobContext` - Job execution context (`rf_jobs::JobContext`)
- `JobRouter` - Routes job classes to queues/connections (`rf_jobs::JobRouter`): `route::<J>(queue)`, `route_to::<J>(queue, connection)`, `resolve(type_name)`

---

### rf-mail

Email sending.

#### Sending Emails

Emails are modelled as `Mailable` types. Each builds a `MailBuilder` describing
the message; the `Mail` facade then sends it synchronously (no `.await`).

```rust
use rf::Mail;
use rf_mail::{Mailable, MailBuilder, Address};

// Define a reusable Mailable.
pub struct WelcomeEmail {
    pub to: String,
    pub name: String,
}

impl Mailable for WelcomeEmail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new(&self.to))
            .subject("Welcome!")
            .text(format!("Welcome to RustForge, {}!", self.name))
    }
}

// Send via the synchronous facade.
Mail::send(WelcomeEmail {
    to: "user@example.com".into(),
    name: "Jane".into(),
})?;

// Or address the recipient first, then hand it a Mailable.
Mail::to("user@example.com").send(WelcomeEmail {
    to: "user@example.com".into(),
    name: "Jane".into(),
})?;
```

**Key Types:**
- `Mail` - Mail facade (`rf_mail::facade::Mail`, re-exported as `rf::Mail`): `Mail::send(mailable)`, `Mail::to(addr) -> Mailer`
- `Mailable` - Mailable trait (`rf_mail::Mailable`); implement `build(&self) -> MailBuilder`
- `MailBuilder` - Chainable builder (`.from`, `.to`, `.subject`, `.text`, `.html`, `.attach`, ...) returned from `Mailable::build`
- `Mailer` - Facade recipient handle (`rf_mail::facade::Mailer`): `Mailer::send(mailable)`

---

### rf-storage

File storage with Laravel-style Storage facade.

#### File Operations (Laravel-style)

```rust
use rf::Storage;

// Put file (contents is a Vec<u8>; synchronous facade - no .await).
Storage::put("path/to/file.txt", contents)?;

// Get file (returns Result<Vec<u8>, String>)
let contents = Storage::get("path/to/file.txt")?;

// Get file as a UTF-8 string
let text = Storage::get_string("path/to/file.txt")?;

// Delete file
Storage::delete("path/to/file.txt")?;

// Check existence (returns bool)
if Storage::exists("path/to/file.txt") {
    println!("File exists");
}

// Select the active disk (subsequent operations use it).
Storage::disk("s3");
Storage::put("uploads/photo.jpg", contents)?;
let contents = Storage::get("uploads/photo.jpg")?;

// Copy file
Storage::copy("old.txt", "new.txt")?;

// Move file
Storage::move_file("old.txt", "new.txt")?;

// File info
let size = Storage::size("path/to/file.txt")?;
```

#### Directory Operations

```rust
use rf::Storage;

// List all files (returns Vec<String>, no .await)
let files = Storage::files();

// List files in a specific directory
let dir_files = Storage::files_in("directory/");

// List directories
let dirs = Storage::directories();
```

**Key Types:**
- `Storage` - Laravel-style storage facade (`rf_storage::StorageFacade`, re-exported as `rf::Storage`); all methods are synchronous
- `disk(name)` - Selects the active disk for subsequent operations

---

## Middleware

### Available Middleware

Named middleware (`"auth"`, `"verified"`, etc.) is applied via the route facade —
`Route::middleware(&["auth"])`. Tower-layer style middleware lives in `rf-web`:
`rf_web::cors_layer(CorsConfig)` and `rf_web::compression_layer()`. CSRF is
token-based via `rf_web::{CsrfToken, csrf_token, csrf_field}`.

```rust
use rf_web::{cors_layer, CorsConfig, compression_layer};

// CORS (Tower layer)
let cors = cors_layer(CorsConfig::default());

// Compression (Tower layer)
let compress = compression_layer();

// Named middleware via the Route facade:
Route::middleware(&["auth"]).group(|| {
    // protected routes
});
```

### Custom Middleware

Custom middleware uses axum's function-style pattern (`async fn(Request, Next) -> Response`),
registered with `axum::middleware::from_fn`. Named middleware can also be registered via
`rf_routing::middleware_pipeline`.

```rust
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;

// A function-style middleware.
async fn custom_middleware(req: Request, next: Next) -> Response {
    // Before request
    println!("Before: {}", req.uri());

    let response = next.run(req).await;

    // After request
    println!("After: {}", response.status());

    response
}

// Apply it to a router:
// let app = router.layer(axum::middleware::from_fn(custom_middleware));
```

---

## Error Handling

### Error Types

```rust
// The core error type is `AppError` (not `Error`). Variants use struct-style
// fields, e.g. `NotFound { resource }`, `BadRequest { message }`.
use rf_core::AppError;

// Real enum (rf-core/src/error/app_error.rs):
pub enum AppError {
    Validation(/* validator::ValidationErrors, feature-gated */),
    NotFound { resource: String },
    Unauthorized,
    Forbidden { reason: String },
    BadRequest { message: String },
    Conflict { message: String },
    RateLimitExceeded,
    Internal(anyhow::Error),
    ServiceUnavailable { service: String },
    // Note: there is no `DatabaseError` variant.
}

// Convert to HTTP response
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound { resource } => Response::json(&json!({
                "error": resource
            })).status(StatusCode::NOT_FOUND),
            AppError::BadRequest { message } => Response::json(&json!({
                "error": message
            })).status(StatusCode::BAD_REQUEST),
            // ...
        }
    }
}
```

---

## Configuration

### Environment Variables

```env
# Application
APP_NAME=MyApp
APP_ENV=production
APP_DEBUG=false
APP_URL=https://example.com

# Database
DATABASE_URL=postgres://user:pass@localhost/db

# Cache
CACHE_DRIVER=redis
REDIS_URL=redis://localhost:6379

# Queue
QUEUE_DRIVER=redis

# Mail
MAIL_DRIVER=smtp
MAIL_HOST=smtp.mailtrap.io
MAIL_PORT=2525
MAIL_USERNAME=username
MAIL_PASSWORD=password

# Storage
FILESYSTEM_DRIVER=s3
AWS_ACCESS_KEY_ID=key
AWS_SECRET_ACCESS_KEY=secret
AWS_DEFAULT_REGION=us-east-1
AWS_BUCKET=bucket
```

### Accessing Config

```rust
// Config lives in rf-config (re-exported as `rf::Config`).
use rf_config::Config;

let app_name = Config::get("app.name");                 // Option<String>
let database_url = Config::get("database.url");          // Option<String>
let debug = Config::get_or("app.debug", "false");        // String with default
```

---

## Testing

### HTTP Testing

```rust
// The HTTP test entrypoint is `HttpTester` (returns `TestResponse`).
use rf_testing::HttpTester;
use axum::http::StatusCode;

#[tokio::test]
async fn test_user_registration() {
    // HttpTester wraps an axum Router.
    let test = HttpTester::new(app);

    let response = test.post("/auth/register", json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "password123"
    })).await;

    // assert_status takes a StatusCode; assert_json is async.
    response
        .assert_status(StatusCode::CREATED)
        .assert_json(json!({
            "email": "test@example.com"
        }))
        .await;
}
```

### Database Testing

Database assertions are provided as async macros
(`assert_database_has!`, `assert_database_missing!`, `assert_database_count!`,
`assert_database_empty!`). Each expands to an async call, so add `.await`.

```rust
use rf_testing::{TestDatabase, assert_database_has, assert_database_count};

#[tokio::test]
async fn test_user_creation() -> Result<(), Box<dyn std::error::Error>> {
    let test_db = TestDatabase::new().await?;
    let user = User::factory().create(&db).await?;

    assert_database_has!("users", { "email" => "test@example.com" }).await?;
    assert_database_count!("users", 1).await?;
    assert_eq!(user.email, "test@example.com");
    Ok(())
}
```

---

## rf (Simplified Imports)

Unified import crate for the entire RustForge framework.

### Usage

```rust
// Direct imports for common items
use rf::{Route, Auth, DB, Hash, Collection, Response};

// Prelude for all common imports
use rf::prelude::*;

// Specific modules
use rf::web::*;        // HTTP, Views, API
use rf::data::*;       // DB, Cache, Validation
use rf::background::*; // Jobs, Events, Broadcast
use rf::services::*;   // Storage, Mail, Auth
use rf::helpers::*;    // Helper functions
```

### Available Modules

| Module | Description |
|--------|-------------|
| `rf::prelude` | All common imports |
| `rf::web` | Request, Response, Routing, Blade |
| `rf::data` | ORM, Eloquent, Cache, Validation |
| `rf::background` | Jobs, Events, Notifications |
| `rf::services` | Storage, Mail, Auth, Logging |
| `rf::facades` | All Laravel-style facades |
| `rf::helpers` | String, array, URL helpers |

---

## rf-dusk (Browser Testing)

Browser testing with WebDriver.

### Basic Usage

```rust
use rf_dusk::{Browser, DuskTestCase};

#[tokio::test]
async fn test_login_page() {
    let browser = Browser::new().await.unwrap();

    browser
        .visit("http://localhost:8000/login").await
        .type_text("#email", "user@example.com").await
        .type_text("#password", "secret").await
        .click("button[type='submit']").await
        .assert_path_is("/dashboard").await
        .assert_see("Welcome").await;
}
```

**Key Types:**
- `Browser` - Browser automation
- `DuskTestCase` - Test case trait
- `Element` - DOM element wrapper

---

## rf-echo (Broadcasting Client)

WebSocket client for real-time broadcasting.

### Basic Usage

```rust
use rf_echo::{Echo, Channel};

let echo = Echo::new()
    .host("ws://localhost:6001")
    .app_key("your-key")
    .connect()
    .await?;

// Subscribe to channel
echo.channel("chat-room")
    .listen("MessageSent", |event| {
        println!("Message: {:?}", event.data);
    })
    .await?;

// Presence channel
echo.join("room.1")
    .here(|users| println!("Users: {:?}", users))
    .joining(|user| println!("{} joined", user.name))
    .leaving(|user| println!("{} left", user.name))
    .await?;
```

**Key Types:**
- `Echo` - Broadcasting client
- `Channel` - Channel subscription
- `PresenceChannel` - Presence channel

---

## rf-envoy (SSH Deployment)

SSH task runner for deployment.

### Basic Usage

```rust
use rf_envoy::{Envoy, Server, Task};

let envoy = Envoy::new()
    .server(Server::new("production")
        .host("192.168.1.100")
        .user("deploy"));

envoy.task("deploy")
    .on(&["production"])
    .run(r#"
        cd /var/www/app
        git pull origin main
        cargo build --release
    "#);

envoy.run("deploy").await?;
```

**Key Types:**
- `Envoy` - Task runner
- `Server` - Server configuration
- `Task` - Task definition

---

## rf-sail (Docker Environment)

Docker development environment management.

### Basic Usage

```rust
use rf_sail::{Sail, Service};

let sail = Sail::new()
    .with_service(Service::Postgres)
    .with_service(Service::Redis)
    .with_service(Service::Mailhog);

// Start services
sail.up().await?;

// Execute command
sail.exec("cargo test").await?;

// Stop services
sail.down().await?;
```

**Key Types:**
- `Sail` - Docker manager
- `Service` - Service enum (Postgres, Redis, etc.)

---

## rf-spark (SaaS Billing)

Stripe-based SaaS billing.

> Note: `rf-spark` exists, but the primary Stripe billing implementation is
> `rf-cashier` (re-exported as `rf::Cashier`), which provides `Billable`,
> `Subscription`, `CheckoutSession`, `Invoice`, and `WebhookEvent`. The
> `Spark` API below is approximate (`Spark` exposes `new()` and `user()`).

### Basic Usage

```rust
use rf_spark::{Spark, Billable};

let spark = Spark::new()
    .stripe_key("sk_test_...");

// Subscribe user
spark.subscribe(&user, "pro-monthly")
    .trial_days(14)
    .create()
    .await?;

// Check subscription
if user.subscribed("pro-monthly") {
    // Has subscription
}

// Cancel
user.subscription("pro-monthly")
    .cancel()
    .await?;
```

**Key Types:**
- `Spark` - Billing manager
- `Billable` - Billable trait
- `Subscription` - Subscription model

---

## rf-ai (AI SDK)

LLM integration with provider abstraction and agents.

**Key Types:**
- `ChatProvider` / `EmbeddingProvider` - provider traits (`rf_ai`)
- `AnthropicProvider` - Anthropic Claude provider (`rf_ai::provider`)
- `Agent` - tool-using agent loop
- `ChatRequest`, `Tool`, `MockChatProvider`

---

## rf-vector (Vector Search)

Vector storage and similarity search.

**Key Types:**
- `Vector` - vector value type (`rf_vector`)
- `DistanceMetric` - distance metric enum
- `InMemoryVectorStore` - in-memory vector store
- `pgvector` - pgvector/Postgres helper module

---

## rf-api-resources (JSON:API)

API resource transformation, including a JSON:API module.

**Key Types (`rf_api_resources::jsonapi`):**
- `JsonApiDocument` - top-level JSON:API document
- `ResourceObject`, `ResourceIdentifier`
- `Relationship`, `RelationshipData`, `RelationshipMap`
- `PrimaryData`

---

## Next Steps

- **[Features](Features)** - Explore all features
- **[Examples](Examples)** - Code examples
- **[Quick Start](Quick-Start)** - Build your first app
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

---

*For detailed documentation on specific modules, see the inline documentation in the source code.*
