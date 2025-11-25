# Features

RustForge provides 100% Laravel 12 feature parity, bringing the elegant Laravel developer experience to Rust. This page provides a comprehensive overview of all features.

## Table of Contents

- [ORM & Database](#orm--database)
- [Authentication & Authorization](#authentication--authorization)
- [HTTP & Routing](#http--routing)
- [Validation](#validation)
- [Caching](#caching)
- [Queue & Jobs](#queue--jobs)
- [Events & Listeners](#events--listeners)
- [Mail System](#mail-system)
- [File Storage](#file-storage)
- [Broadcasting](#broadcasting)
- [Rate Limiting](#rate-limiting)
- [Internationalization](#internationalization-i18n)
- [GraphQL](#graphql)
- [Health Checks](#health-checks)
- [Audit Logging](#audit-logging)
- [CLI Commands](#cli-commands)
- [Service Container](#service-container)
- [Testing](#testing)

---

## ORM & Database

RustForge includes a powerful Laravel-style DB facade and query builder.

### Features

- **DB Facade**: Laravel-style `DB::table()` query builder
- **Query Builder**: Fluent, chainable query methods
- **Migrations**: Version control for your database schema
- **Seeders**: Populate database with test data
- **Multiple Databases**: PostgreSQL, MySQL, SQLite support
- **Transactions**: `DB::transaction()` support
- **Raw Queries**: `DB::select()`, `DB::insert()`, `DB::update()`, `DB::delete()`
- **Pagination**: Built-in pagination support

### Example

```rust
use rf_db_facade::DB;

// Laravel-style Query Builder
let users = DB::table("users")
    .where_clause("active", "=", true.into())
    .order_by("name", "asc")
    .limit(10)
    .get().await?;

// Get single record
let user = DB::table("users")
    .where_clause("id", "=", 1.into())
    .first().await?;

// Insert
let id = DB::table("users").insert(json!({
    "name": "John",
    "email": "john@example.com"
})).await?;

// Update
DB::table("users")
    .where_clause("id", "=", 1.into())
    .update(json!({"active": true})).await?;

// Delete
DB::table("users")
    .where_clause("id", "=", 1.into())
    .delete().await?;

// Raw queries
let users = DB::select("SELECT * FROM users WHERE active = ?", &[true.into()]).await?;

// Transactions
DB::begin_transaction().await?;
// ... operations
DB::commit().await?;
```

### Supported Databases

| Database | Version | Status |
|----------|---------|--------|
| PostgreSQL | 12+ | ✅ Full Support |
| MySQL | 8+ | ✅ Full Support |
| SQLite | 3.35+ | ✅ Full Support |

---

## Authentication & Authorization

Complete authentication system with multiple strategies.

### Features

- **Multiple Auth Strategies**: JWT, Session, OAuth
- **Password Hashing**: Bcrypt with configurable cost
- **User Registration & Login**: Built-in endpoints
- **Password Reset**: Email-based password recovery
- **Email Verification**: Verify user emails
- **Remember Me**: Persistent login sessions
- **Two-Factor Auth**: TOTP-based 2FA
- **API Tokens**: Personal access tokens
- **Policies**: Resource-based authorization
- **Middleware**: Route protection

### Example

```rust
use rf_auth_facade::Auth;
use rf_auth::Hash;

// Register user
let password_hash = Hash::make("password123")?;
let user = User::create(email, name, password_hash).await?;

// Login with Laravel-style Auth facade
Auth::login(user).await?;

// Or attempt login with credentials (like Laravel's Auth::attempt)
let credentials = json!({
    "email": "user@example.com",
    "password": "secret"
});
if Auth::attempt(credentials).await? {
    println!("Login successful!");
}

// Check authentication
if Auth::check().await {
    println!("User is authenticated");
}

// Get current user
if let Some(user) = Auth::user::<User>().await {
    println!("Welcome, {}", user.name);
}

// Get user ID
if let Some(id) = Auth::id().await {
    println!("User ID: {}", id);
}

// Logout
Auth::logout().await;

// Protect routes with middleware
router.group(middleware::auth(), |router| {
    router.get("/profile", get_profile);
});

// Check authorization
if !user.can("edit-post", &post) {
    return Err(Error::Forbidden);
}
```

### Supported Auth Methods

- JWT (JSON Web Tokens)
- Session-based authentication
- OAuth 2.0 (Google, GitHub, Facebook)
- API tokens
- Basic HTTP authentication

---

## HTTP & Routing

Expressive routing system with middleware support.

### Features

- **RESTful Routing**: Resource-based routes
- **Route Parameters**: Dynamic URL segments
- **Route Groups**: Organize routes with common attributes
- **Middleware**: Request/response pipeline
- **CORS Support**: Cross-origin resource sharing
- **Request Validation**: Built-in validation
- **Response Formatting**: JSON, XML, HTML
- **File Uploads**: Multipart form data handling
- **Streaming**: Large file streaming
- **WebSockets**: Real-time communication

### Example

```rust
use rf_route_facade::Route;
use rf_http::{Request, Response, middleware};

// Laravel-style Route facade
Route::get("/", home);
Route::post("/users", create_user);
Route::put("/users/:id", update_user);
Route::delete("/users/:id", delete_user);

// Route groups with middleware
Route::middleware(&["auth"]).group(|| {
    Route::prefix("/api/v1").group(|| {
        Route::get("/posts", list_posts);
        Route::post("/posts", create_post);
        Route::get("/posts/:id", show_post);
    });
});

// Resource routes (like Laravel)
Route::resource("/users", UserController);

// Named routes
Route::get("/profile", profile).name("profile");

// Route with multiple middleware
Route::middleware(&["auth", "verified"]).group(|| {
    Route::get("/dashboard", dashboard);
});

// Apply global middleware
Route::use_middleware(middleware::cors());
Route::use_middleware(middleware::rate_limit(60, 60));
```

### Available Middleware

- Authentication
- CORS
- Rate Limiting
- Logging
- CSRF Protection
- Compression (gzip, brotli)
- Timeout
- Request ID

---

## Validation

Powerful validation framework with custom rules.

### Features

- **Built-in Rules**: 40+ validation rules
- **Custom Rules**: Define your own validators
- **Nested Validation**: Validate nested structures
- **Array Validation**: Validate array elements
- **Conditional Validation**: Rules based on conditions
- **Custom Messages**: Localized error messages
- **Automatic Validation**: Via derive macros

### Example

```rust
use rf_validation::{Validate, ValidationRule};
use serde::Deserialize;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 3, max = 50))]
    pub name: String,

    #[validate(length(min = 8), regex = "^(?=.*[A-Z])(?=.*[0-9]).*$")]
    pub password: String,

    #[validate(range(min = 18, max = 120))]
    pub age: u8,

    #[validate(url)]
    pub website: Option<String>,
}

// In handler
pub async fn create_user(
    Json(payload): Json<CreateUserRequest>
) -> Result<Response, Error> {
    payload.validate()?; // Auto validates
    // ... create user
}
```

### Available Rules

- `required`, `optional`
- `email`, `url`, `ip`
- `length(min, max)`, `range(min, max)`
- `regex(pattern)`
- `in(values)`, `not_in(values)`
- `unique(table, column)`
- `exists(table, column)`
- `date`, `after(date)`, `before(date)`
- `confirmed` (password confirmation)
- `alpha`, `alpha_numeric`, `numeric`
- Custom rules

---

## Caching

High-performance caching layer with multiple drivers.

### Features

- **Multiple Drivers**: Redis, Memcached, File, In-Memory
- **Cache Tags**: Group related cache items
- **Cache Events**: Listen to cache operations
- **Atomic Locks**: Prevent race conditions
- **Remember**: Cache query results
- **Cache Aside**: Automatic cache population
- **TTL Support**: Time-to-live for cache items

### Example

```rust
use rf_cache_facade::Cache;
use std::time::Duration;

// Simple caching with Laravel-style facade
Cache::put("key", &"value", Duration::from_secs(3600)).await?;
let value: Option<String> = Cache::get("key").await?;

// Check if key exists
if Cache::has("key").await? {
    println!("Key exists");
}

// Cache with closure (like Laravel's Cache::remember)
let users = Cache::remember("users:all", Duration::from_secs(3600), || async {
    Ok(User::find().all(&db).await?)
}).await?;

// Store forever
Cache::forever("config", &"value").await?;

// Remember forever
let settings = Cache::remember_forever("settings", || async {
    Ok(load_settings().await?)
}).await?;

// Pull: get and delete
let value: Option<String> = Cache::pull("temp_key").await?;

// Add only if doesn't exist
let added = Cache::add("unique_key", &"value", Duration::from_secs(60)).await?;

// Increment/decrement
Cache::increment("counter", 1).await?;
Cache::decrement("counter", 1).await?;

// Cache tags
let tagged = Cache::tags(&["users", "posts"]).await;
tagged.set("key", &"value", Duration::from_secs(3600)).await?;

// Flush tagged cache
Cache::tags(&["users"]).await.flush().await?;

// Remove single key
Cache::forget("key").await?;

// Flush all cache
Cache::flush().await?;
```

### Supported Cache Drivers

| Driver | Best For | Persistent |
|--------|----------|------------|
| Redis | Production, distributed | ✅ Yes |
| Memcached | Production, simple | ✅ Yes |
| File | Development, small apps | ✅ Yes |
| Memory | Testing, temporary | ❌ No |

---

## Queue & Jobs

Background job processing with multiple queue drivers.

### Features

- **Multiple Drivers**: Redis, Database, In-Memory
- **Job Retry**: Automatic retry with exponential backoff
- **Job Priority**: High/normal/low priority queues
- **Delayed Jobs**: Schedule jobs for future execution
- **Job Chaining**: Execute jobs in sequence
- **Job Batching**: Group related jobs
- **Failed Job Handling**: Dedicated failed jobs table
- **Queue Workers**: Multiple concurrent workers
- **Job Monitoring**: Track job progress

### Example

```rust
use rf_queue::{Queue, Job};
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), Error> {
        // Send email
        Mail::to(&self.to)
            .subject(&self.subject)
            .body(&self.body)
            .send()
            .await?;
        Ok(())
    }
}

// Dispatch job
Queue::push(SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
    body: "Welcome to RustForge!".to_string(),
}).await?;

// Delayed job
Queue::later(60, SendEmailJob { /* ... */ }).await?;

// Job chain
Queue::chain(vec![
    Box::new(ProcessOrder { /* ... */ }),
    Box::new(SendConfirmation { /* ... */ }),
    Box::new(UpdateInventory { /* ... */ }),
]).await?;
```

### Running Workers

```bash
# Start queue worker
forge queue:work

# Specific queue
forge queue:work --queue=emails

# Multiple workers
forge queue:work --workers=4
```

---

## Events & Listeners

Event-driven architecture with synchronous and asynchronous listeners.

### Features

- **Event Dispatching**: Dispatch custom events
- **Event Listeners**: Subscribe to events
- **Event Discovery**: Automatic listener registration
- **Async Listeners**: Non-blocking event handling
- **Event Broadcasting**: Broadcast to WebSocket clients
- **Event Replay**: Replay events for debugging

### Example

```rust
use rf_event_facade::Event;
use serde::{Serialize, Deserialize};

// Define event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistered {
    pub user_id: i32,
    pub email: String,
}

// Register listeners (typically in bootstrap)
Event::listen("user.registered", |event: UserRegistered| async move {
    // Send welcome email
    Mail::to(&event.email)
        .subject("Welcome!")
        .send()
        .await?;
    Ok(())
}).await;

Event::listen("user.registered", |event: UserRegistered| async move {
    // Log the registration
    Log::info(&format!("New user registered: {}", event.email)).await;
    Ok(())
}).await;

// Dispatch event (like Laravel's Event::dispatch)
Event::dispatch("user.registered", UserRegistered {
    user_id: 1,
    email: "user@example.com".to_string(),
}).await?;

// Check if event has listeners
if Event::has_listeners("user.registered").await {
    println!("Event has listeners");
}

// Dispatch multiple events
Event::dispatch_many(vec![
    ("user.created", json!({"id": 1})),
    ("notification.send", json!({"type": "welcome"})),
]).await?;
```

---

## Mail System

Email sending with multiple drivers and templates.

### Features

- **Multiple Drivers**: SMTP, Mailgun, SendGrid, SES
- **Email Templates**: HTML and text templates
- **Attachments**: Send files with emails
- **Queue Support**: Queue emails for async sending
- **Markdown Emails**: Write emails in Markdown
- **Testing**: Email testing utilities

### Example

```rust
use rf_mail_facade::Mail;

// Simple email with Laravel-style facade
Mail::to("user@example.com")
    .subject("Welcome!")
    .body("Welcome to RustForge!")
    .send()
    .await?;

// With CC and BCC
Mail::to("user@example.com")
    .cc("manager@example.com")
    .bcc("admin@example.com")
    .subject("Weekly Report")
    .body("...")
    .send()
    .await?;

// With template
Mail::to("user@example.com")
    .subject("Order Confirmation")
    .view("emails.order_confirmation", json!({
        "order_number": "12345",
        "total": 99.99
    }))
    .send()
    .await?;

// With attachment
Mail::to("user@example.com")
    .subject("Invoice")
    .attach("/path/to/invoice.pdf")
    .send()
    .await?;

// Queue email for async sending
Mail::to("user@example.com")
    .subject("Newsletter")
    .body("...")
    .queue()
    .await?;

// Send to multiple recipients
Mail::to(&["user1@example.com", "user2@example.com"])
    .subject("Announcement")
    .body("...")
    .send()
    .await?;
```

### Supported Mail Drivers

- SMTP
- Mailgun
- SendGrid
- Amazon SES
- Postmark
- Log (for testing)

---

## File Storage

Unified file storage interface for local and cloud storage.

### Features

- **Multiple Drivers**: Local, S3, FTP
- **Streaming**: Stream large files
- **Visibility**: Public/private file access
- **Temporary URLs**: Signed URLs for private files
- **File Operations**: Copy, move, delete, exists
- **Directory Operations**: List, create, delete directories

### Example

```rust
use rf_storage_facade::Storage;

// Store file with Laravel-style facade
Storage::put("uploads/photo.jpg", file_contents).await?;

// Get file
let contents = Storage::get("uploads/photo.jpg").await?;

// Delete file
Storage::delete("uploads/photo.jpg").await?;

// Check if file exists
if Storage::exists("uploads/photo.jpg").await? {
    println!("File exists");
}

// Use specific disk
Storage::disk("s3").put("uploads/photo.jpg", file_contents).await?;
let contents = Storage::disk("s3").get("uploads/photo.jpg").await?;

// Generate temporary URL (1 hour)
let url = Storage::disk("s3")
    .temporary_url("uploads/photo.jpg", 3600)
    .await?;

// List files in directory
let files = Storage::files("uploads/").await?;
let all_files = Storage::all_files("uploads/").await?; // recursive

// List directories
let dirs = Storage::directories("uploads/").await?;

// Copy and move
Storage::copy("old/path.jpg", "new/path.jpg").await?;
Storage::move_file("old/path.jpg", "new/path.jpg").await?;

// Get file info
let size = Storage::size("uploads/photo.jpg").await?;
let modified = Storage::last_modified("uploads/photo.jpg").await?;
```

### Supported Storage Drivers

- Local filesystem
- Amazon S3
- DigitalOcean Spaces
- Wasabi
- Any S3-compatible storage

---

## Broadcasting

Real-time event broadcasting via WebSockets.

### Features

- **WebSocket Support**: Pusher, Socket.io compatible
- **Private Channels**: Authenticated channels
- **Presence Channels**: Track online users
- **Client Events**: Client-to-client messaging
- **Broadcasting Events**: Broadcast events to channels

### Example

```rust
use rf_broadcast::{Broadcast, Channel};

// Broadcast to channel
Broadcast::channel("chat.1")
    .send(json!({
        "message": "Hello, World!",
        "user": "John"
    }))
    .await?;

// Private channel
Broadcast::private("user.1")
    .send(json!({
        "notification": "New message"
    }))
    .await?;

// Presence channel
Broadcast::presence("chat.1")
    .join(user_id)
    .await?;
```

---

## Rate Limiting

Protect your API from abuse with rate limiting.

### Features

- **Multiple Strategies**: Fixed window, sliding window
- **Multiple Stores**: Redis, in-memory
- **Per-User Limits**: Different limits per user
- **Custom Keys**: Rate limit by IP, user, API key
- **Headers**: X-RateLimit headers in responses

### Example

```rust
use rf_http::middleware;

// Global rate limit (60 requests per minute)
router.use_middleware(middleware::rate_limit(60, 60));

// Per-route rate limit
router.get("/api/data", middleware::rate_limit(10, 60).apply(handler));

// Custom rate limiter
let limiter = RateLimiter::for_user()
    .max_attempts(100)
    .decay_minutes(1)
    .by(|req| req.user_id());

router.use_middleware(limiter);
```

---

## Internationalization (i18n)

Multi-language support with translation files.

### Features

- **Translation Files**: JSON-based translations
- **Pluralization**: Language-specific pluralization
- **Parameter Replacement**: Dynamic values in translations
- **Locale Detection**: Auto-detect user locale
- **Fallback Locales**: Fallback to default language

### Example

```rust
use rf_i18n::Trans;

// Simple translation
let message = Trans::get("welcome.message").await?;

// With parameters
let message = Trans::get("welcome.user", json!({
    "name": "John"
})).await?;

// Pluralization
let message = Trans::choice("messages.count", 5).await?;

// Change locale
Trans::set_locale("de").await?;
```

---

## GraphQL

GraphQL API support with schema generation.

### Features

- **Schema Generation**: Generate GraphQL schema from Rust types
- **Queries & Mutations**: Full GraphQL support
- **Subscriptions**: Real-time updates via WebSocket
- **DataLoader**: Batch and cache database queries
- **Playground**: Built-in GraphQL playground

### Example

```rust
use rf_graphql::{Schema, Object, Context};

#[Object]
impl Query {
    async fn users(&self, ctx: &Context<'_>) -> Result<Vec<User>> {
        let db = ctx.data::<Database>()?;
        Ok(User::find().all(db).await?)
    }

    async fn user(&self, ctx: &Context<'_>, id: i32) -> Result<Option<User>> {
        let db = ctx.data::<Database>()?;
        Ok(User::find_by_id(id).one(db).await?)
    }
}

#[Object]
impl Mutation {
    async fn create_user(
        &self,
        ctx: &Context<'_>,
        name: String,
        email: String,
    ) -> Result<User> {
        let db = ctx.data::<Database>()?;
        let user = User::create(name, email).insert(db).await?;
        Ok(user)
    }
}
```

---

## Health Checks

Monitor application health and dependencies.

### Features

- **Custom Checks**: Define custom health checks
- **Database Check**: Verify database connectivity
- **Cache Check**: Verify cache connectivity
- **Disk Space**: Monitor disk usage
- **Response Format**: JSON health status

### Example

```rust
use rf_health::{HealthCheck, Check};

let health = HealthCheck::new()
    .register(Check::database())
    .register(Check::cache())
    .register(Check::custom("api", || async {
        // Custom health check logic
        Ok(())
    }));

// GET /health
router.get("/health", health.handler());
```

---

## Audit Logging

Track user actions and system events.

### Features

- **Automatic Tracking**: Track model changes
- **Custom Events**: Log custom events
- **User Tracking**: Associate events with users
- **Metadata**: Store additional context
- **Query Logs**: Search audit logs

### Example

```rust
use rf_audit::{Audit, AuditLog};

// Automatic tracking (via trait)
#[derive(Model, Auditable)]
pub struct Post {
    // Model fields
}

// Custom audit log
AuditLog::create()
    .user(user_id)
    .event("payment.processed")
    .metadata(json!({
        "amount": 99.99,
        "currency": "USD"
    }))
    .save()
    .await?;

// Query logs
let logs = AuditLog::for_user(user_id)
    .event_type("payment.processed")
    .get()
    .await?;
```

---

## CLI Commands

50+ built-in CLI commands for development.

### Features

- **Code Generation**: Generate models, controllers, migrations
- **Database Operations**: Migrate, seed, rollback
- **Cache Operations**: Clear, flush cache
- **Queue Operations**: Run workers, retry failed jobs
- **Development Server**: Built-in development server

### Available Commands

```bash
# Code Generation
forge make:model User --migration
forge make:controller UserController
forge make:migration create_users_table
forge make:seeder UserSeeder
forge make:factory UserFactory
forge make:request StoreUserRequest
forge make:policy UserPolicy
forge make:job SendEmailJob
forge make:event UserRegistered
forge make:listener SendWelcomeEmail
forge make:mail WelcomeEmail
forge make:notification OrderShipped
forge make:middleware CheckRole

# Database
forge migrate                    # Run migrations
forge migrate:rollback          # Rollback last migration
forge migrate:reset             # Reset all migrations
forge migrate:refresh           # Reset and re-run all migrations
forge migrate:fresh             # Drop all tables and re-migrate
forge migrate:status            # Show migration status
forge db:seed                   # Seed database
forge db:wipe                   # Drop all tables

# Cache
forge cache:clear               # Clear application cache
forge cache:forget <key>        # Remove specific cache entry
forge config:cache              # Cache configuration
forge config:clear              # Clear config cache
forge route:cache               # Cache routes
forge route:clear               # Clear route cache

# Queue
forge queue:work                # Start queue worker
forge queue:retry <id>          # Retry failed job
forge queue:flush               # Delete all failed jobs
forge queue:restart             # Restart queue workers

# Development
forge serve                     # Start development server
forge tinker                    # Interactive REPL
forge route:list                # List all routes
forge schedule:run              # Run scheduled tasks
forge schedule:list             # List scheduled tasks

# Testing
forge test                      # Run tests
forge test --coverage           # Run tests with coverage
```

---

## Service Container

Dependency injection container for managing services.

### Features

- **Auto-wiring**: Automatic dependency resolution
- **Singletons**: Singleton service registration
- **Interfaces**: Bind interfaces to implementations
- **Contextual Binding**: Different bindings per context
- **Service Providers**: Organize service registration

### Example

```rust
use rf_container::Container;

// Register service
let container = Container::new();
container.singleton(|| Database::connect().await);
container.bind(|| EmailService::new());

// Resolve service
let db: Arc<Database> = container.resolve()?;

// Bind interface to implementation
container.bind_interface::<dyn Cache, RedisCache>();

// Service provider
pub struct AppServiceProvider;

impl ServiceProvider for AppServiceProvider {
    fn register(&self, container: &Container) {
        container.singleton(|| Database::connect().await);
        container.bind(|| Cache::redis());
    }

    fn boot(&self) {
        // Bootstrap services
    }
}
```

---

## Testing

Comprehensive testing utilities.

### Features

- **Unit Tests**: Test individual components
- **Integration Tests**: Test feature workflows
- **HTTP Tests**: Test API endpoints
- **Database Tests**: Test with test database
- **Mocking**: Mock dependencies
- **Factories**: Generate test data
- **Assertions**: Rich assertion library

### Example

```rust
use rf_testing::{TestCase, DatabaseTestCase};

#[tokio::test]
async fn test_user_registration() {
    let mut test = TestCase::new().await;

    let response = test.post("/auth/register", json!({
        "email": "test@example.com",
        "name": "Test User",
        "password": "password123"
    }))
    .await;

    response.assert_status(201);
    response.assert_json_contains(json!({
        "user": {
            "email": "test@example.com"
        }
    }));

    // Verify in database
    let user = User::find()
        .filter(User::Column::Email.eq("test@example.com"))
        .one(&test.db())
        .await?
        .unwrap();

    assert_eq!(user.name, "Test User");
}
```

---

## Next Steps

- **[API Documentation](API-Documentation)** - Detailed API reference
- **[Examples](Examples)** - Code examples for each feature
- **[Quick Start](Quick-Start)** - Build your first app
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

---

*Have a feature request? Open an issue on [GitHub](https://github.com/Chregu12/RustForge/issues).*
