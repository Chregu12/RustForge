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

#### Router

```rust
use rf_http::{Router, Request, Response, Json};

let mut router = Router::new();

// Basic routes
router.get("/", home);
router.post("/users", create_user);
router.put("/users/:id", update_user);
router.delete("/users/:id", delete_user);

// Route parameters
router.get("/users/:id", |Path(id): Path<i32>| async move {
    // Use id
});

// Route groups
router.group(middleware::auth(), |router| {
    router.get("/profile", get_profile);
    router.post("/logout", logout);
});

// Prefix
router.prefix("/api/v1", |router| {
    router.resource("/posts", PostController);
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

Authentication and authorization.

#### JWT Authentication

```rust
use rf_auth::{JwtAuth, Hash, AuthGuard};

// Generate token
let token = JwtAuth::generate_token(user.id)?;

// Verify token
let claims = JwtAuth::verify_token(&token)?;

// Hash password
let hash = Hash::make("password123")?;

// Verify password
let is_valid = Hash::check("password123", &hash)?;

// Protect routes
router.get("/profile", |auth: AuthGuard| async move {
    let user_id = auth.user_id();
    // ...
});
```

#### Session Authentication

```rust
use rf_auth::SessionAuth;

// Login
SessionAuth::login(&session, user.id).await?;

// Logout
SessionAuth::logout(&session).await?;

// Get current user
let user_id = SessionAuth::user(&session).await?;
```

**Key Types:**
- `JwtAuth` - JWT token management
- `SessionAuth` - Session-based auth
- `Hash` - Password hashing
- `AuthGuard` - Authentication middleware
- `Policy<T>` - Authorization policies

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

Caching layer with multiple drivers.

#### Basic Usage

```rust
use rf_cache::{Cache, CacheManager};

// Put value in cache (TTL in seconds)
Cache::put("key", "value", 3600).await?;

// Get value from cache
let value: Option<String> = Cache::get("key").await?;

// Remember (cache with closure)
let users = Cache::remember("users:all", 3600, || async {
    User::find().all(&db).await
}).await?;

// Forever (no expiration)
Cache::forever("key", "value").await?;

// Forget (delete)
Cache::forget("key").await?;

// Flush all
Cache::flush().await?;
```

#### Cache Tags

```rust
use rf_cache::Cache;

// Tag caches
Cache::tags(&["users", "posts"])
    .put("key", "value", 3600)
    .await?;

// Get with tags
let value = Cache::tags(&["users"])
    .get("key")
    .await?;

// Flush by tag
Cache::tags(&["users"]).flush().await?;
```

#### Atomic Locks

```rust
use rf_cache::Cache;

// Acquire lock
let lock = Cache::lock("process:payment", 10).await?;

if let Some(lock) = lock {
    // Process payment
    // Lock automatically released when dropped
}
```

**Key Types:**
- `Cache` - Cache facade
- `CacheManager` - Cache manager
- `CacheLock` - Atomic lock
- `CacheDriver` - Cache driver trait

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

File storage.

#### File Operations

```rust
use rf_storage::Storage;

// Put file
Storage::disk("s3")
    .put("path/to/file.txt", contents)
    .await?;

// Get file
let contents = Storage::disk("s3")
    .get("path/to/file.txt")
    .await?;

// Delete file
Storage::disk("s3")
    .delete("path/to/file.txt")
    .await?;

// Check existence
let exists = Storage::disk("s3")
    .exists("path/to/file.txt")
    .await?;

// Copy file
Storage::disk("s3")
    .copy("old.txt", "new.txt")
    .await?;

// Move file
Storage::disk("s3")
    .move("old.txt", "new.txt")
    .await?;

// File size
let size = Storage::disk("s3")
    .size("path/to/file.txt")
    .await?;

// Last modified
let modified = Storage::disk("s3")
    .last_modified("path/to/file.txt")
    .await?;
```

#### Directory Operations

```rust
// List files
let files = Storage::disk("s3")
    .files("directory/")
    .await?;

// List all files (recursive)
let all_files = Storage::disk("s3")
    .all_files("directory/")
    .await?;

// List directories
let dirs = Storage::disk("s3")
    .directories("directory/")
    .await?;

// Create directory
Storage::disk("s3")
    .make_directory("new/directory")
    .await?;

// Delete directory
Storage::disk("s3")
    .delete_directory("directory/")
    .await?;
```

#### Temporary URLs

```rust
// Generate signed URL (1 hour)
let url = Storage::disk("s3")
    .temporary_url("private/file.pdf", 3600)
    .await?;
```

**Key Types:**
- `Storage` - Storage facade
- `StorageManager` - Storage manager
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

## Next Steps

- **[Features](Features)** - Explore all features
- **[Examples](Examples)** - Code examples
- **[Quick Start](Quick-Start)** - Build your first app
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

---

*For detailed documentation on specific modules, see the inline documentation in the source code.*
