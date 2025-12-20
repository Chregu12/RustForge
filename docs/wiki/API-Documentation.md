# API Documentation

Comprehensive API documentation for RustForge v1.0.0.

## Core Modules

### rf-core

The core framework module providing application bootstrapping and lifecycle management.

```rust
use rf_core::Application;

// Create application
let app = Application::new();

// Configure application
let app = Application::builder()
    .env_file(".env")
    .log_level(LogLevel::Info)
    .build()?;

// Run application
app.run().await?;
```

**Key Types:**
- `Application` - Main application container
- `Config` - Configuration management
- `Environment` - Environment variable access

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
use rf_route_facade::Route;
use rf_http::{Request, Response, Json};

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
use rf_http::{Request, Json, Query, Path, Form};

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
use rf_http::Response;

// JSON response
Response::json(data)

// With status code
Response::json(data).status(201)

// Plain text
Response::text("Hello, World!")

// HTML
Response::html("<h1>Hello</h1>")

// Redirect
Response::redirect("/login")

// No content
Response::no_content()

// Custom headers
Response::json(data)
    .header("X-Custom", "value")
    .status(200)
```

**Key Types:**
- `Router` - Route definition
- `Request` - HTTP request
- `Response` - HTTP response
- `Json<T>` - JSON extractor
- `Query<T>` - Query param extractor
- `Path<T>` - Path param extractor
- `Form<T>` - Form data extractor

---

### rf-auth

Authentication and authorization with Laravel-style Auth facade.

#### Auth Facade (Laravel-style)

```rust
use rf_auth_facade::Auth;
use rf_auth::Hash;

// Login user (like Laravel's Auth::login)
Auth::login(user).await?;

// Attempt login with credentials (like Laravel's Auth::attempt)
let credentials = json!({
    "email": "user@example.com",
    "password": "secret"
});
if Auth::attempt(credentials).await? {
    println!("Login successful!");
}

// Check if authenticated
if Auth::check().await {
    println!("User is logged in");
}

// Check if guest
if Auth::guest().await {
    println!("User is not logged in");
}

// Get current user
if let Some(user) = Auth::user::<User>().await {
    println!("Welcome, {}", user.name);
}

// Get user ID
if let Some(id) = Auth::id().await {
    println!("User ID: {}", id);
}

// Login with remember me
Auth::login_using_id(user_id, true).await?;

// Check if via remember
if Auth::via_remember().await {
    println!("Logged in via remember token");
}

// Logout
Auth::logout().await;

// Use specific guard
let api_guard = Auth::guard("api").await;
if api_guard.check().await {
    println!("Authenticated on API guard");
}

// Role checks
if Auth::has_role("admin").await {
    println!("User is admin");
}

if Auth::has_any_role(&["admin", "moderator"]).await {
    println!("User has elevated privileges");
}
```

#### Password Hashing

```rust
use rf_auth::Hash;

// Hash password
let hash = Hash::make("password123")?;

// Verify password
let is_valid = Hash::check("password123", &hash)?;
```

#### Protect Routes

```rust
use rf_route_facade::Route;

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
use rf_validation::{Validate, ValidationRule};

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
use rf_validation::{Validator, ValidationError};

pub struct UniqueEmail;

impl Validator for UniqueEmail {
    async fn validate(&self, value: &str, _context: &Context) -> Result<(), ValidationError> {
        let exists = User::find()
            .filter(User::Column::Email.eq(value))
            .count(&db)
            .await? > 0;

        if exists {
            return Err(ValidationError::new("email_exists", "Email already registered"));
        }

        Ok(())
    }
}
```

**Key Types:**
- `Validate` - Validation trait
- `Validator` - Custom validator trait
- `ValidationError` - Validation error
- `ValidationResult` - Validation result

---

### rf-cache

Caching layer with Laravel-style Cache facade.

#### Basic Usage (Laravel-style)

```rust
use rf_cache_facade::Cache;
use std::time::Duration;

// Put value in cache
Cache::put("key", &"value", Duration::from_secs(3600)).await?;

// Get value from cache
let value: Option<String> = Cache::get("key").await?;

// Check if key exists
if Cache::has("key").await? {
    println!("Key exists");
}

// Remember (cache with closure) - like Laravel's Cache::remember
let users = Cache::remember("users:all", Duration::from_secs(3600), || async {
    Ok(User::find().all(&db).await?)
}).await?;

// Remember forever
let settings = Cache::remember_forever("settings", || async {
    Ok(load_settings().await?)
}).await?;

// Store forever (no expiration)
Cache::forever("key", &"value").await?;

// Add only if doesn't exist
let added = Cache::add("unique_key", &"value", Duration::from_secs(60)).await?;

// Pull: get and delete
let value: Option<String> = Cache::pull("temp_key").await?;

// Forget (delete single key)
Cache::forget("key").await?;

// Flush all cache
Cache::flush().await?;

// Increment/decrement counters
Cache::increment("counter", 1).await?;
Cache::decrement("counter", 1).await?;
```

#### Cache Tags

```rust
use rf_cache_facade::Cache;
use std::time::Duration;

// Create tagged cache
let tagged = Cache::tags(&["users", "posts"]).await;
tagged.set("key", &"value", Duration::from_secs(3600)).await?;

// Flush all entries with specific tag
Cache::tags(&["users"]).await.flush().await?;
```

**Key Types:**
- `Cache` - Laravel-style cache facade
- `TaggedCache` - Tagged cache operations

---

### rf-queue & rf-jobs

Background job processing.

#### Defining Jobs

```rust
use rf_jobs::{Job, JobContext};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, _ctx: &JobContext) -> Result<(), Error> {
        // Send email
        Mail::to(&self.to)
            .subject(&self.subject)
            .body(&self.body)
            .send()
            .await?;

        Ok(())
    }

    fn max_tries(&self) -> u32 {
        3
    }

    fn timeout(&self) -> u64 {
        60
    }
}
```

#### Dispatching Jobs

```rust
use rf_queue::Queue;

// Dispatch immediately
Queue::push(SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Hello".to_string(),
    body: "Welcome!".to_string(),
}).await?;

// Delay (in seconds)
Queue::later(60, job).await?;

// Specific queue
Queue::on("emails").push(job).await?;

// Chain jobs
Queue::chain(vec![
    Box::new(job1),
    Box::new(job2),
    Box::new(job3),
]).await?;
```

**Key Types:**
- `Job` - Job trait
- `Queue` - Queue facade
- `QueueManager` - Queue manager
- `JobContext` - Job execution context

---

### rf-mail

Email sending.

#### Sending Emails

```rust
use rf_mail::Mail;

// Simple email
Mail::to("user@example.com")
    .subject("Welcome!")
    .body("Welcome to RustForge!")
    .send()
    .await?;

// With template
Mail::to("user@example.com")
    .subject("Order Confirmation")
    .view("emails.order", json!({
        "order_id": 12345
    }))
    .send()
    .await?;

// With attachment
Mail::to("user@example.com")
    .subject("Invoice")
    .attach("/path/to/invoice.pdf")
    .send()
    .await?;

// Multiple recipients
Mail::to("user1@example.com")
    .cc("user2@example.com")
    .bcc("admin@example.com")
    .subject("Newsletter")
    .send()
    .await?;

// Queue email
Mail::to("user@example.com")
    .subject("Welcome")
    .queue()
    .await?;
```

**Key Types:**
- `Mail` - Mail facade
- `Mailable` - Mailable trait
- `MailDriver` - Mail driver trait

---

### rf-storage

File storage with Laravel-style Storage facade.

#### File Operations (Laravel-style)

```rust
use rf_storage_facade::Storage;

// Put file (uses default disk)
Storage::put("path/to/file.txt", contents).await?;

// Get file
let contents = Storage::get("path/to/file.txt").await?;

// Delete file
Storage::delete("path/to/file.txt").await?;

// Check existence
if Storage::exists("path/to/file.txt").await? {
    println!("File exists");
}

// Use specific disk
Storage::disk("s3").put("uploads/photo.jpg", contents).await?;
let contents = Storage::disk("s3").get("uploads/photo.jpg").await?;

// Copy file
Storage::copy("old.txt", "new.txt").await?;

// Move file
Storage::move_file("old.txt", "new.txt").await?;

// File info
let size = Storage::size("path/to/file.txt").await?;
let modified = Storage::last_modified("path/to/file.txt").await?;
```

#### Directory Operations

```rust
use rf_storage_facade::Storage;

// List files in directory
let files = Storage::files("directory/").await?;

// List all files (recursive)
let all_files = Storage::all_files("directory/").await?;

// List directories
let dirs = Storage::directories("directory/").await?;
```

#### Temporary URLs

```rust
// Generate signed URL (1 hour)
let url = Storage::disk("s3")
    .temporary_url("private/file.pdf", 3600)
    .await?;
```

**Key Types:**
- `Storage` - Laravel-style storage facade
- `Disk` - Storage disk interface

---

## Middleware

### Available Middleware

```rust
use rf_http::middleware;

// Authentication
router.use_middleware(middleware::auth());

// CORS
router.use_middleware(middleware::cors());

// Rate limiting
router.use_middleware(middleware::rate_limit(60, 60)); // 60 req/min

// Logging
router.use_middleware(middleware::logger());

// CSRF protection
router.use_middleware(middleware::csrf());

// Compression
router.use_middleware(middleware::compress());
```

### Custom Middleware

```rust
use rf_http::{Middleware, Request, Response, Next};

pub struct CustomMiddleware;

#[async_trait]
impl Middleware for CustomMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Result<Response> {
        // Before request
        println!("Before: {}", req.uri());

        let response = next.run(req).await?;

        // After request
        println!("After: {}", response.status());

        Ok(response)
    }
}
```

---

## Error Handling

### Error Types

```rust
use rf_core::Error;

// Application errors
pub enum Error {
    NotFound(String),
    BadRequest(String),
    Unauthorized(String),
    Forbidden(String),
    InternalError(String),
    ValidationError(ValidationErrors),
    DatabaseError(DbErr),
}

// Convert to HTTP response
impl Into Response> for Error {
    fn into_response(self) -> Response {
        match self {
            Error::NotFound(msg) => Response::json(json!({
                "error": msg
            })).status(404),
            Error::BadRequest(msg) => Response::json(json!({
                "error": msg
            })).status(400),
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
use rf_core::Config;

let app_name = Config::get("app.name")?;
let database_url = Config::get("database.url")?;
let debug = Config::get("app.debug").unwrap_or(false);
```

---

## Testing

### HTTP Testing

```rust
use rf_testing::TestCase;

#[tokio::test]
async fn test_user_registration() {
    let test = TestCase::new().await;

    let response = test.post("/auth/register", json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "password123"
    })).await;

    response.assert_status(201);
    response.assert_json(json!({
        "email": "test@example.com"
    }));
}
```

### Database Testing

```rust
use rf_testing::DatabaseTestCase;

#[tokio::test]
async fn test_user_creation() {
    let test = DatabaseTestCase::new().await;

    let user = User::factory()
        .create(&test.db())
        .await?;

    assert_eq!(user.email, "test@example.com");
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
    .service(Service::Postgres)
    .service(Service::Redis)
    .service(Service::Mailhog);

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

## Next Steps

- **[Features](Features)** - Explore all features
- **[Examples](Examples)** - Code examples
- **[Quick Start](Quick-Start)** - Build your first app
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

---

*For detailed documentation on specific modules, see the inline documentation in the source code.*
