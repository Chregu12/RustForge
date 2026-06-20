# Features

RustForge targets Laravel feature parity, bringing the elegant Laravel developer experience to Rust, and includes recent Laravel 13 additions (`Cache::touch`, queue routing by job class, JSON:API resources, a provider-agnostic AI SDK, and vector/semantic search). This page provides a comprehensive overview of all features.

## Table of Contents

- [ORM & Database](#orm--database)
- [Authentication & Authorization](#authentication--authorization)
- [HTTP & Routing](#http--routing)
- [Validation](#validation)
- [Form Requests](#form-requests)
- [Exception Handling](#exception-handling)
- [Blade Templates](#blade-templates)
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
- [Browser Testing (Dusk)](#browser-testing-dusk)
- [Broadcasting Client (Echo)](#broadcasting-client-echo)
- [SSH Deployment (Envoy)](#ssh-deployment-envoy)
- [Docker Environment (Sail)](#docker-environment-sail)
- [SaaS Billing (Spark)](#saas-billing-spark)
- [AI SDK (rf-ai)](#ai-sdk-rf-ai)
- [Vector & Semantic Search (rf-vector)](#vector--semantic-search-rf-vector)
- [JSON:API Resources](#jsonapi-resources)
- [Simplified Imports (rf)](#simplified-imports-rf)

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

With `#[auto_await]` on the surrounding `fn`/`impl`/`mod`, the query-builder terminals
(`get`, `find`, `create`, `update`, `delete`, `paginate`, `begin_transaction`, `commit`)
are awaited for you, so DB code reads exactly like Laravel — no `.await`:

```rust
use rf::DB;

#[auto_await]
async fn examples() -> Result<()> {
    // Query
    let users = DB::table("users")
        .r#where("active", true)
        .order_by("name", "asc")
        .limit(10)
        .get()?;

    // Find by ID
    let user = DB::table("users").find(1)?;

    // Create (returns the record)
    let user = DB::table("users").create(json!({
        "name": "John",
        "email": "john@example.com"
    }))?;

    // Update
    DB::table("users")
        .r#where("id", 1)
        .update(json!({"active": true}))?;

    // Delete
    DB::table("users")
        .r#where("id", 1)
        .delete()?;

    // Advanced queries
    let users = DB::table("users")
        .r#where("active", true)
        .where_op("age", ">=", 18)
        .where_in("role", vec!["admin", "mod"])
        .where_like("name", "John%")
        .order_by_desc("created_at")
        .paginate(15, 1)?;

    // Raw queries — `DB::select` is a raw passthrough (not in the auto-await
    // set), so it keeps its `.await`.
    let users = DB::select("SELECT * FROM users WHERE active = ?", &[true.into()]).await?;

    // Transactions — `begin_transaction` and `commit` are in the auto-await set.
    DB::begin_transaction()?;
    // ... operations
    DB::commit()?;

    Ok(())
}
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

With `#[auto_await]`, the `Auth` and model calls (`create`, `login`, `attempt`, `check`,
`user`, `id`, `logout`) are awaited for you — Laravel-style, await-free:

```rust
use rf::{Auth, Hash};

#[auto_await]
async fn auth_examples() -> Result<()> {
    // Register user. `Hash::make` returns a String directly (no Result, no `?`).
    let password_hash = Hash::make("password123");
    let user = User::create(json!({
        "email": "user@example.com",
        "name": "John",
        "password": password_hash,
    }))?;

    // Login with Laravel-style Auth facade.
    Auth::login(user)?;

    // Or attempt login with credentials (like Laravel's Auth::attempt).
    let credentials = json!({
        "email": "user@example.com",
        "password": "secret"
    });
    if Auth::attempt(credentials)? {
        println!("Login successful!");
    }

    // Check authentication.
    if Auth::check() {
        println!("User is authenticated");
    }

    // Get current user.
    if let Some(user) = Auth::user::<User>() {
        println!("Welcome, {}", user.name);
    }

    // Get user ID.
    if let Some(id) = Auth::id() {
        println!("User ID: {}", id);
    }

    // Logout.
    Auth::logout();

    Ok(())
}

// Protect routes with middleware (named middleware via the Route facade)
Route::middleware(&["auth"]).group(|| {
    Route::get("/profile", "get_profile");
});

// Check authorization (sync helper, not in the auto-await set)
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
use rf::prelude::*;

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
use rf_validation::Validate;
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

## Form Requests

Laravel-style form requests with automatic validation and authorization.

### Features

- **Automatic Validation**: Validate on extraction
- **Authorization Checks**: Control access to actions
- **Custom Messages**: User-friendly error messages
- **Validation Rules**: 40+ built-in rules
- **Field-Level Rules**: Attribute-based validation

### Example

```rust
use rustforge::*;

// Define a form request
form_request! {
    pub struct CreateUserRequest {
        #[required, email, unique("users", "email")]
        email: String,

        #[required, min(8)]
        password: String,

        #[required, min(2), max(100)]
        name: String,
    }

    fn authorize(&self) -> bool {
        auth!(check)  // Only authenticated users
    }

    fn messages() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("email.required", "Email is required"),
            ("email.email", "Please provide a valid email"),
            ("password.min", "Password must be at least 8 characters"),
        ])
    }
}

// Use in handler - automatic validation! `#[auto_await]` awaits `create` for you.
#[auto_await]
async fn store(Validated(req): Validated<CreateUserRequest>) -> Response {
    let user = User::create(json!({
        "email": req.email,
        "password": bcrypt!(req.password),
        "name": req.name,
    }));
    Response::json(&user).status(StatusCode::CREATED)
}
```

### Simple Attribute Syntax

```rust
#[validated]
struct CreatePostRequest {
    #[validate(required, min_length(5))]
    title: String,

    #[validate(required)]
    body: String,

    #[validate(required, exists("categories", "id"))]
    category_id: i64,
}
```

### Available Validation Rules

| Category | Rules |
|----------|-------|
| Basic | `required`, `nullable`, `string`, `integer`, `numeric`, `boolean`, `array` |
| String | `email`, `url`, `ip`, `uuid`, `alpha`, `alpha_num`, `lowercase`, `uppercase`, `regex("pattern")` |
| Length | `min(n)`, `max(n)`, `between(min, max)`, `min_length(n)`, `max_length(n)`, `size(n)` |
| Date | `date`, `date_format("fmt")`, `before("date")`, `after("date")` |
| Database | `unique("table", "column")`, `exists("table", "column")` |
| Compare | `same("field")`, `different("field")`, `confirmed` |
| Conditional | `required_if`, `required_unless`, `required_with`, `required_without` |

---

## Exception Handling

Laravel-style global exception handler for consistent error responses.

### Features

- **Global Handler**: Centralized error handling
- **Custom Rendering**: Format errors for JSON/HTML
- **Selective Reporting**: Control what gets logged
- **Helper Macros**: `abort_if!`, `abort_unless!`, `rescue!`
- **Don't Flash**: Protect sensitive form fields

### Example

```rust
use rustforge::*;

// Define global exception handler
exception_handler! {
    // Exceptions that should not be logged
    dont_report: [
        ValidationException,
        AuthenticationException,
    ];

    // Form fields not flashed to session
    dont_flash: [
        "password",
        "password_confirmation",
    ];

    // Custom exception rendering
    fn render(error: &AppError, request: &Request) -> Response {
        match error {
            AppError::NotFound { .. } => {
                if request.wants_json() {
                    Response::json(&json!({ "error": "Not found" }))
                        .status(StatusCode::NOT_FOUND)
                } else {
                    view!("errors.404").status(StatusCode::NOT_FOUND)
                }
            }
            _ => Response::text("Server Error").status(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }

    // Custom exception reporting
    fn report(error: &AppError) {
        logger!(error: "Application error: {:?}", error);
    }
}
```

### Helper Macros

```rust
#[auto_await]
async fn handler() -> Result<()> {
    // Abort if condition is true
    abort_if!(user.is_banned(), 403, "Account banned");

    // Abort unless condition is true
    abort_unless!(user.can_edit(&post), 403, "Not authorized");

    // Rescue with fallback value (`find` is awaited for you — no `.await`)
    let user = rescue!(User::find(id), User::default());

    // Report without throwing
    report!(error);

    Ok(())
}
```

---

## Blade Templates

Laravel Blade-like templating with familiar directives.

### Features

- **Blade Directives**: `@if`, `@foreach`, `@auth`, `@guest`
- **Output Escaping**: `{{ }}` escaped, `{!! !!}` raw
- **Form Helpers**: `@csrf`, `@method`
- **Sections/Stacks**: Template inheritance
- **Rust Integration**: `@rust` for inline code

### Example

```rust
use rustforge::*;

let html = blade! {
    <div class="container">
        @if let Some(user) = user {
            <h1>Welcome, {{ user.name }}!</h1>

            @if user.is_admin {
                <span class="badge">Admin</span>
            } @else {
                <span class="badge">User</span>
            }

            <ul>
            @foreach post in posts {
                <li>{{ post.title }}</li>
            }
            </ul>
        } @else {
            <p>Please log in</p>
        }

        @auth {
            <a href="/logout">Logout</a>
        }

        @guest {
            <a href="/login">Login</a>
        }

        <form method="POST">
            @csrf
            @method("PUT")
            <button type="submit">Submit</button>
        </form>
    </div>
};
```

### Available Directives

| Category | Directive | Description |
|----------|-----------|-------------|
| Control | `@if`, `@else`, `@foreach`, `@for`, `@while`, `@match` | Control flow |
| Auth | `@auth`, `@guest` | Authentication checks |
| Forms | `@csrf`, `@method("PUT")` | Form helpers |
| Output | `{{ expr }}`, `{!! expr !!}`, `@json(data)` | Output with escaping |
| Include | `@include("partial")` | Include templates |
| Utility | `@isset`, `@empty`, `@env`, `@rust`, `@class` | Utilities |

### Additional Macros

```rust
// Simple HTML template
let html = html! { <div>Hello, {name}!</div> };

// Template sections
section!("content") { <h1>Content</h1> }

// Push to stack
push!("scripts") { <script src="/app.js"></script> }

// Render stack
let scripts = stack!("scripts");
```

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
- **Touch (Laravel 13)**: Reset an entry's TTL without rewriting its value via `Cache::touch(key, ttl)`

### Example

With `#[auto_await]` on the surrounding `fn`/`impl`/`mod`, every `Cache` operation in the
auto-await set (`put`, `get`, `has`, `remember`, `forever`, `remember_forever`, `pull`,
`add`, `increment`, `decrement`, `tags`, `forget`, `flush`) reads await-free, Laravel-style:

```rust
use rf::Cache;
use std::time::Duration;

#[auto_await]
async fn cache_examples() -> Result<()> {
    // Simple caching with Laravel-style facade.
    Cache::put("key", "value", Duration::from_secs(3600))?;
    let value: Option<String> = Cache::get("key")?;

    // Check if key exists.
    if Cache::has("key")? {
        println!("Key exists");
    }

    // Cache with closure (like Laravel's Cache::remember). `remember` and the
    // `all` inside the closure are both in the auto-await set.
    let users = Cache::remember("users:all", Duration::from_secs(3600), || async {
        Ok(User::all()?)
    })?;

    // Store forever.
    Cache::forever("config", "value")?;

    // Remember forever. `load_settings()` is your own async fn — not in the
    // auto-await set — so it keeps its `.await`.
    let settings = Cache::remember_forever("settings", || async {
        Ok(load_settings().await?)
    })?;

    // Pull: get and delete.
    let value: Option<String> = Cache::pull("temp_key")?;

    // Add only if doesn't exist.
    let added = Cache::add("unique_key", "value", Duration::from_secs(60))?;

    // Increment/decrement.
    Cache::increment("counter", 1)?;
    Cache::decrement("counter", 1)?;

    // Cache tags. `tags` and `flush` are in the auto-await set; `TaggedCache::set`
    // is NOT, so it keeps its explicit `.await`.
    let tagged = Cache::tags(&["users", "posts"]);
    tagged.set("key", &"value", Duration::from_secs(3600)).await?;

    // Flush tagged cache.
    Cache::tags(&["users"]).flush()?;

    // Remove single key.
    Cache::forget("key")?;

    // Flush all cache.
    Cache::flush()?;

    Ok(())
}
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
- **Queue Routing by Class (Laravel 13)**: Route a job type to a specific queue via `JobRouter::route::<MyJob>("queue-name")` (in `rf-jobs`); a registered route takes precedence over a job's default queue

### Example

```rust
use rf_jobs::{dispatch, dispatch_later, Job, JobContext, JobResult, QueueManager};
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub to: String,
    pub subject: String,
    pub body: String,
}

impl Job for SendEmailJob {
    async fn handle(&self, _ctx: JobContext) -> JobResult {
        // Send email (build a Mailable, then send it)
        // Mail::to(&self.to).send(my_mailable)?;
        Ok(())
    }

    // Optional overrides:
    fn queue(&self) -> &str { "emails" }
    fn max_attempts(&self) -> u32 { 5 }
    fn backoff(&self) -> Duration { Duration::from_secs(30) }
}

// Build the queue manager once (e.g. during bootstrap)
let queue_manager = QueueManager::new("redis://localhost:6379").await?;

// Dispatch job (synchronous API - takes a &QueueManager)
dispatch(&queue_manager, SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
    body: "Welcome to RustForge!".to_string(),
})?;

// Delayed job
dispatch_later(&queue_manager, SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
    body: "Welcome to RustForge!".to_string(),
}, Duration::from_secs(60))?;
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
use rf::Event;
use serde::{Serialize, Deserialize};

// Define event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRegistered {
    pub user_id: i32,
    pub email: String,
}

// The `Event` facade is fully synchronous — like `Cache`/`Auth`, you never
// write `.await` on it (and `#[auto_await]` resolves the `dispatch`/`send`
// calls transparently).
#[auto_await]
async fn register_events() {
    Event::listen("user.registered", |event: UserRegistered| {
        // Send a welcome email (a Mailable). `send` resolves under #[auto_await].
        Mail::to(&event.email).send(WelcomeEmail::new(&event.email))?;
        Ok(())
    });

    Event::listen("user.registered", |event: UserRegistered| {
        // Log the registration — `Log::info` is synchronous.
        Log::info(&format!("New user registered: {}", event.email));
        Ok(())
    });

    // Dispatch an event.
    Event::dispatch("user.registered", UserRegistered {
        user_id: 1,
        email: "user@example.com".to_string(),
    })?;

    // Check if an event has listeners.
    if Event::has_listeners("user.registered") {
        println!("Event has listeners");
    }

    // Dispatch several events.
    Event::dispatch("user.created", json!({ "id": 1 }))?;
    Event::dispatch("notification.send", json!({ "type": "welcome" }))?;
}
```

---

## Mail System

Email sending with multiple drivers, templates, and Laravel-style Mailables.

### Features

- **Multiple Drivers**: SMTP, Mailgun, SendGrid, SES
- **Mailable Classes**: Structured email definitions
- **Email Templates**: HTML and text templates
- **Attachments**: Send files with emails
- **Queue Support**: Queue emails for async sending
- **Markdown Emails**: Write emails in Markdown
- **Notifications**: Multi-channel notifications
- **Testing**: Email testing utilities

### Mailable Classes (Laravel-style)

```rust
use rustforge::*;

// Define a mailable
mailable! {
    pub struct WelcomeEmail {
        user: User,
        activation_url: String,
    }

    fn envelope(&self) -> Envelope {
        Envelope::new()
            .subject("Welcome to RustForge!")
            .from("hello@rustforge.dev")
            .reply_to("support@rustforge.dev")
    }

    fn content(&self) -> Content {
        Content::view("emails.welcome")
            .with("user", &self.user)
            .with("url", &self.activation_url)
    }

    fn attachments(&self) -> Vec<Attachment> {
        vec![
            Attachment::from_path("/docs/guide.pdf")
                .as_("Getting Started.pdf"),
        ]
    }
}

// Send a Mailable to a recipient. No `.await` needed under `#[auto_await]`.
Mail::to("user@example.com")
    .send(WelcomeEmail { user, activation_url })?;

// To send in the background, dispatch a job that sends the mail (see Queue & Jobs).
dispatch(&queue, SendWelcomeEmailJob { email: "user@example.com".to_string() })?;
```

### Simple Attribute Syntax

```rust
#[mail(subject = "Welcome!", view = "emails.welcome")]
pub struct WelcomeEmail {
    pub user: User,
}
```

### Notifications

```rust
notification! {
    pub struct OrderShipped {
        order: Order,
    }

    fn via(&self) -> Vec<Channel> {
        vec![Channel::Mail, Channel::Database, Channel::Slack]
    }

    fn to_mail(&self) -> Mailable {
        Mailable::new()
            .subject("Your order has shipped!")
            .view("emails.order_shipped")
    }

    fn to_database(&self) -> Value {
        json!({
            "type": "order_shipped",
            "order_id": self.order.id,
        })
    }
}

// Send notification (`notify` is not in the auto-await set, so it keeps `.await`)
user.notify(OrderShipped { order }).await?;
```

### Simple Email API

```rust
use rf::Mail;
use rf_mail::{Mailable, MailBuilder};

// A Mailable describes the message by building a MailBuilder.
struct WelcomeEmail;

impl Mailable for WelcomeEmail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .subject("Welcome!")
            .text("Welcome to RustForge!")
    }
}

// Send to a recipient (Mail::to is synchronous; send returns MailResult<()>)
Mail::to("user@example.com").send(WelcomeEmail)?;

// Or send without specifying a recipient on the facade (the Mailable can
// carry its own `to`/`from` via the builder)
Mail::send(WelcomeEmail)?;

// Build with an HTML body or attachments using the same builder:
struct InvoiceEmail { path: String }

impl Mailable for InvoiceEmail {
    fn build(&self) -> MailBuilder {
        MailBuilder::new()
            .subject("Invoice")
            .html("<p>Your invoice is attached.</p>")
            .attach(&self.path)
            .expect("attachment exists")
    }
}
```

### Markdown Emails

```rust
let content = markdown! {
    # Welcome {{ user.name }}!

    Thanks for joining us.

    - Create projects
    - Invite team members
    - Start building

    @component("button", url: "https://app.rustforge.dev")
        Get Started
    @endcomponent
};
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
use rf::Storage;

// Store file with Laravel-style facade (synchronous - no .await).
// `put` takes the file contents as a Vec<u8>.
Storage::put("uploads/photo.jpg", file_contents)?;

// Get file (returns Vec<u8>; use get_string for text)
let contents = Storage::get("uploads/photo.jpg")?;

// Delete file
Storage::delete("uploads/photo.jpg")?;

// Check if file exists (returns bool - no ?)
if Storage::exists("uploads/photo.jpg") {
    println!("File exists");
}

// Select the active disk for subsequent operations
Storage::disk("s3");

// List files (returns Vec<String> - no ?)
let files = Storage::files();
let files_in_uploads = Storage::files_in("uploads/");

// List directories
let dirs = Storage::directories();

// Copy and move
Storage::copy("old/path.jpg", "new/path.jpg")?;
Storage::move_file("old/path.jpg", "new/path.jpg")?;

// Get file size
let size = Storage::size("uploads/photo.jpg")?;
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
use rf_broadcast::{broadcast, subscribe, Channel, SimpleEvent};

#[auto_await]
async fn broadcasting_examples() -> Result<()> {
    // Broadcast to a channel — `send` is in the auto-await set.
    Channel::public("chat.1")
        .send(json!({
            "message": "Hello, World!",
            "user": "John"
        }))?;

    // Private channel.
    Channel::private("user.1")
        .send(json!({
            "notification": "New message"
        }))?;

    // Presence channel — `join` is not in the auto-await set, so it keeps `.await`.
    Channel::presence("chat.1")
        .join(user_id)
        .await?;

    Ok(())
}
```

> **Note:** See the [Real-time Chat example](Examples#real-time-chat) for the concrete
> `rf_broadcast` API (`broadcast`/`subscribe` helpers, `Channel`, `SimpleEvent`).

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
use rf_ratelimit::{MemoryRateLimiter, RateLimitConfig, RateLimiter};
use rf::Route;

#[auto_await]
async fn rate_limit_example() -> Result<()> {
    // Configure an in-memory rate limiter (e.g. 60 requests per minute)
    let config = RateLimitConfig::per_minute(60);
    let limiter = MemoryRateLimiter::new(config);

    // Check a limit for a key — `check` is in the auto-await set.
    let result = limiter.check("user:1")?;
    if !result.allowed {
        // Reject the request with a 429
    }

    Ok(())
}

// Apply rate limiting to routes via named middleware on the Route facade
Route::middleware(&["throttle"]).group(|| {
    Route::get("/api/data", "get_data");
});
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

#[auto_await]
async fn i18n_examples() -> Result<()> {
    // Simple translation — `get` is in the auto-await set.
    let message = Trans::get("welcome.message")?;

    // With parameters.
    let message = Trans::get("welcome.user", json!({
        "name": "John"
    }))?;

    // Pluralization (`choice` is not in the auto-await set, so it keeps `.await`).
    let message = Trans::choice("messages.count", 5).await?;

    // Change locale (`set_locale` is not in the auto-await set).
    Trans::set_locale("de").await?;

    Ok(())
}
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

// `#[auto_await]` on the resolver impl: `all`, `find`, and `create` are awaited for you.
#[Object]
#[auto_await]
impl Query {
    async fn users(&self, _ctx: &Context<'_>) -> Result<Vec<User>> {
        Ok(User::all()?)
    }

    async fn user(&self, _ctx: &Context<'_>, id: i32) -> Result<Option<User>> {
        Ok(User::find(id)?)
    }
}

#[Object]
#[auto_await]
impl Mutation {
    async fn create_user(
        &self,
        _ctx: &Context<'_>,
        name: String,
        email: String,
    ) -> Result<User> {
        let user = User::create(json!({ "name": name, "email": email }))?;
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

#[auto_await]
async fn audit_examples() -> Result<()> {
    // Custom audit log — `save` is in the auto-await set.
    AuditLog::create()
        .user(user_id)
        .event("payment.processed")
        .metadata(json!({
            "amount": 99.99,
            "currency": "USD"
        }))
        .save()?;

    // Query logs — `get` is in the auto-await set.
    let logs = AuditLog::for_user(user_id)
        .event_type("payment.processed")
        .get()?;

    Ok(())
}
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
use rf_testing::{HttpTester, TestDatabase};
use axum::http::StatusCode;
use serde_json::json;

#[tokio::test]
async fn test_user_registration() {
    // Spin up a test database (async)
    let _db = TestDatabase::new().await.unwrap();

    // `app` is your axum Router
    let tester = HttpTester::new(app);

    let response = tester
        .post("/auth/register", json!({
            "email": "test@example.com",
            "name": "Test User",
            "password": "password123"
        }))
        .await;

    // assert_status takes a StatusCode (chainable)
    response
        .assert_status(StatusCode::CREATED)
        .assert_json(json!({
            "user": {
                "email": "test@example.com"
            }
        }))
        .await;
}
```

---

## Browser Testing (Dusk)

Automated browser testing with WebDriver, inspired by Laravel Dusk.

### Features

- **WebDriver Integration**: Chrome, Firefox, Safari support via fantoccini
- **Page Object Pattern**: Reusable page components
- **Element Interactions**: Click, type, select, submit
- **Assertions**: Assert text, URL, element visibility
- **Screenshots**: Capture screenshots on failure
- **Wait Helpers**: Wait for elements, conditions, JavaScript

### Example

```rust
use rf_dusk::{Browser, DuskTestCase};

#[tokio::test]
async fn test_user_login() {
    let browser = Browser::new().await.unwrap();

    browser
        .visit("http://localhost:8000/login")
        .await
        .type_text("#email", "user@example.com")
        .await
        .type_text("#password", "secret")
        .await
        .click("button[type='submit']")
        .await
        .assert_path_is("/dashboard")
        .await
        .assert_see("Welcome back!")
        .await;
}

// Page Object Pattern
struct LoginPage;

impl LoginPage {
    async fn login(browser: &Browser, email: &str, password: &str) {
        browser
            .visit("http://localhost:8000/login").await
            .type_text("#email", email).await
            .type_text("#password", password).await
            .click("button[type='submit']").await;
    }
}
```

---

## Broadcasting Client (Echo)

WebSocket broadcasting client compatible with Pusher and Soketi, inspired by Laravel Echo.

### Features

- **Pusher Protocol**: Compatible with Pusher, Soketi, Ably
- **Channel Types**: Public, Private, Presence channels
- **Authentication**: Automatic channel authentication
- **Presence Tracking**: Track online users in presence channels
- **Event Handling**: Subscribe to and handle events

### Example

```rust
use rf_echo::{Echo, Channel};

// Connect to broadcasting server
let echo = Echo::new()
    .host("ws://localhost:6001")
    .app_key("your-app-key")
    .connect()
    .await?;

// Subscribe to public channel
echo.channel("chat-room")
    .listen("MessageSent", |event| {
        println!("New message: {:?}", event.data);
    })
    .await?;

// Subscribe to private channel
echo.private("user.1")
    .listen("NotificationReceived", |event| {
        println!("Notification: {:?}", event.data);
    })
    .await?;

// Subscribe to presence channel
echo.join("chat.1")
    .here(|users| println!("Users online: {:?}", users))
    .joining(|user| println!("{} joined", user.name))
    .leaving(|user| println!("{} left", user.name))
    .listen("MessageSent", |event| {
        println!("Message: {:?}", event.data);
    })
    .await?;
```

---

## SSH Deployment (Envoy)

SSH task runner for deployment automation, inspired by Laravel Envoy.

### Features

- **Task Definition**: Define deployment tasks
- **Multi-Server**: Execute tasks on multiple servers
- **Stories**: Group tasks into stories
- **Parallel Execution**: Run tasks in parallel
- **Notifications**: Slack, Discord notifications on completion
- **Variables**: Template variable substitution

### Example

```rust
use rf_envoy::{Envoy, Server, Task};

let envoy = Envoy::new()
    .server(Server::new("production")
        .host("192.168.1.100")
        .user("deploy")
        .identity_file("~/.ssh/id_rsa"))
    .server(Server::new("staging")
        .host("192.168.1.101")
        .user("deploy"));

// Define tasks
envoy
    .task("deploy")
    .on(&["production"])
    .run(r#"
        cd /var/www/app
        git pull origin main
        cargo build --release
        sudo systemctl restart app
    "#);

envoy
    .task("rollback")
    .on(&["production"])
    .run(r#"
        cd /var/www/app
        git checkout HEAD~1
        cargo build --release
        sudo systemctl restart app
    "#);

// Define story (multiple tasks)
envoy
    .story("full-deploy", &["deploy", "cache:clear", "migrate"]);

// Run tasks
envoy.run("deploy").await?;
```

---

## Docker Environment (Sail)

Docker development environment management, inspired by Laravel Sail.

### Features

- **Service Management**: MySQL, PostgreSQL, Redis, Mailhog, etc.
- **Docker Compose**: Auto-generated docker-compose.yml
- **Container Commands**: Up, down, shell, exec
- **File Watching**: Auto-rebuild on file changes
- **Volume Management**: Persistent data volumes

### Example

```rust
use rf_sail::{Sail, Service};

let sail = Sail::new()
    .service(Service::Postgres)
    .service(Service::Redis)
    .service(Service::Mailhog)
    .service(Service::Minio);

// Start all services
sail.up().await?;

// Execute command in container
sail.exec("cargo test").await?;

// Open shell
sail.shell().await?;

// Stop all services
sail.down().await?;

// Available services
// - Service::Postgres
// - Service::Mysql
// - Service::Redis
// - Service::Memcached
// - Service::Mailhog
// - Service::Minio
// - Service::MeiliSearch
// - Service::Selenium
```

---

## SaaS Billing (Spark)

Stripe-based SaaS billing system, inspired by Laravel Spark/Cashier.

### Features

- **Stripe Integration**: Full Stripe API support
- **Subscriptions**: Create, update, cancel subscriptions
- **Payment Methods**: Add, remove, update payment methods
- **Invoices**: Generate and manage invoices
- **Webhooks**: Handle Stripe webhook events
- **Customer Management**: Stripe customer sync

### Example

```rust
use rf_spark::{Spark, Billable};

#[auto_await]
async fn billing_examples() -> Result<()> {
    // Initialize Spark
    let spark = Spark::new()
        .stripe_key("sk_test_...")
        .stripe_secret("sk_secret_...");

    // Subscribe user to plan — the `create` terminal is in the auto-await set;
    // `subscribe`/`trial_days` are builder steps.
    let subscription = spark
        .subscribe(&user, "pro-monthly")
        .trial_days(14)
        .create()?;

    // Check subscription status (sync helper)
    if user.subscribed("pro-monthly") {
        // User has active subscription
    }

    // Cancel subscription (`cancel` is not in the auto-await set)
    user.subscription("pro-monthly")
        .cancel()
        .await?;

    // Update payment method (`update_payment_method` is not in the auto-await set)
    user.update_payment_method("pm_...")
        .await?;

    // Get invoices (`invoices` is not in the auto-await set)
    let invoices = user.invoices().await?;

    // Webhook handler
    let handler = spark.webhook_handler()
        .on("invoice.paid", |event| {
            println!("Invoice paid: {:?}", event);
        })
        .on("customer.subscription.deleted", |event| {
            println!("Subscription cancelled: {:?}", event);
        });

    Ok(())
}
```

---

## AI SDK (rf-ai)

A provider-agnostic AI toolkit (`rf-ai`) for text generation, embeddings, and tool-calling agents, with an Anthropic provider built in. Application code targets the `ChatProvider` / `EmbeddingProvider` traits, so a live `AnthropicProvider` can be swapped for a `MockChatProvider` in tests.

### Features

- **Provider-Agnostic Traits**: `ChatProvider` and `EmbeddingProvider`
- **Anthropic Provider**: Speaks the Anthropic Messages API directly over `reqwest` (no official Rust SDK required); default model `claude-opus-4-8`
- **Tool-Calling Agents**: An `Agent` loop runs tools over any `ChatProvider`, with a configurable max-turns guard
- **Mock Providers**: Deterministic `MockChatProvider` / `MockEmbeddingProvider` for offline tests

### Example

```rust
use rf_ai::prelude::*;

let provider = AnthropicProvider::new(std::env::var("ANTHROPIC_API_KEY").unwrap());

let request = ChatRequest::default_model()
    .max_tokens(256)
    .system("You are a terse assistant.")
    .message(Message::user("Name the capital of France."));

let response = provider.chat(&request).await?;
println!("{}", response.text());
```

---

## Vector & Semantic Search (rf-vector)

Vector and semantic search primitives (`rf-vector`): dense embedding vectors, similarity/distance metrics, an in-memory brute-force store, and pure SQL helpers for the Postgres `pgvector` extension.

### Features

- **`Vector`**: Dense `f32` embeddings with `dot`, `cosine_similarity`, `euclidean_distance`, `magnitude`, `normalized`, and checked `try_*` variants
- **`DistanceMetric`**: `Cosine`, `Euclidean`, `DotProduct`, with a unified "higher score = more similar" scoring helper
- **`InMemoryVectorStore`**: Brute-force k-nearest-neighbour search with JSON metadata
- **`pgvector` helpers**: String builders (`to_literal`, `operator`, `order_by_nearest`, `nearest_neighbor_sql`) for use with rf-orm raw query fragments (no database dependency required)

### Example

```rust
use rf_vector::*;
use serde_json::json;

let mut store = InMemoryVectorStore::new();
store.add("doc:cat", Vector::new(vec![1.0, 0.0, 0.0]), json!({"title": "cats"}));
store.add("doc:dog", Vector::new(vec![0.0, 1.0, 0.0]), json!({"title": "dogs"}));

let query = Vector::new(vec![0.9, 0.1, 0.0]);
let hits = store.search(&query, 1, DistanceMetric::Cosine);
assert_eq!(hits[0].id, "doc:cat");
```

---

## JSON:API Resources

The `rf-api-resources` crate includes a `jsonapi` module for building JSON:API-compliant responses (a Laravel 13 addition).

### Features

- **`JsonApiResource`**: Render a model as a JSON:API resource object
- **`JsonApiDocument`**: Top-level document with primary data, relationships, and links
- **Relationships**: Model relationships as JSON:API `Relationship` objects
- **Collections**: `document_from_collection` to build a document from a resource collection

---

## Simplified Imports (rf)

The `rf` crate provides simplified imports for the entire RustForge framework.

### Features

- **Direct Imports**: Import common types directly
- **Prelude**: One import for all common items
- **5 Main Modules**: Organized into logical groups
- **Laravel-Style**: Familiar naming conventions

### Usage

```rust
// Option 1: Direct imports (most common)
use rf::{Route, Auth, DB, Hash, Collection, Response};

// Option 2: Prelude for everything
use rf::prelude::*;

// Option 3: Specific modules
use rf::web::*;        // HTTP, Views, API
use rf::data::*;       // DB, Cache, Validation
use rf::background::*; // Jobs, Events, Broadcast
use rf::services::*;   // Storage, Mail, Auth
use rf::helpers::*;    // Helper functions

// Available at root level:
// - Facades: Route, Auth, DB, Cache, Event, Storage, Log, Mail, Session, Config, View
// - Helpers: Hash, redirect, csrf_token
// - Collections: Collection, collect
// - Macros: rules, route, controller, Model
// - Validation: Validate
// - Errors: RustForgeError, Result
```

### Module Overview

| Module | Description | Key Exports |
|--------|-------------|-------------|
| `rf::prelude` | Common imports | All facades, helpers, macros |
| `rf::web` | HTTP & Views | Request, Response, Blade, Inertia |
| `rf::data` | Database | ORM, Eloquent, Cache, Validation |
| `rf::background` | Background | Jobs, Events, Notifications, Broadcast |
| `rf::services` | Infrastructure | Storage, Mail, Auth, Logging |
| `rf::helpers` | Helpers | String, array, URL helpers |
| `rf::facades` | All Facades | Route, Auth, DB, Cache, etc. |

---

## Next Steps

- **[API Documentation](API-Documentation)** - Detailed API reference
- **[Examples](Examples)** - Code examples for each feature
- **[Quick Start](Quick-Start)** - Build your first app
- **[Migration Guide](Migration-Guide)** - Migrate from other frameworks

---

*Have a feature request? Open an issue on [GitHub](https://github.com/Chregu12/RustForge/issues).*
