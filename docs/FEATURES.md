# RustForge Features Overview

> **Complete Feature List with Examples and Usage Guides**

This document provides a comprehensive overview of all RustForge features organized by tier and category.

---

## Table of Contents

- [Core Features](#core-features)
- [Tier 1: Essential Features](#tier-1-essential-features)
- [Tier 2: Enterprise Features](#tier-2-enterprise-features)
- [Tier 3: Nice-to-Have Features](#tier-3-nice-to-have-features)
- [Advanced Features](#advanced-features)

---

## Core Features

### CLI & Code Generation

**Code Scaffolding**
- `make:model` - Generate models with optional migrations
- `make:controller` - Create HTTP controllers (standard or API)
- `make:migration` - Database migration files
- `make:seeder` - Database seeders
- `make:factory` - Test data factories
- `make:job` - Background job classes
- `make:event` - Event classes
- `make:listener` - Event listeners
- `make:command` - Custom CLI commands
- `make:middleware` - HTTP middleware
- `make:request` - Form request validation

**Database Management**
- `database:create` - Interactive database setup wizard
- `migrate` - Run pending migrations
- `migrate:rollback` - Rollback migrations
- `migrate:fresh` - Drop all tables and re-run migrations
- `db:seed` - Seed the database
- `db:show` - Show database information

**Interactive REPL (Tinker)**
- `tinker` - Start interactive database console
- CRUD operations: find, list, create, update, delete
- Raw SQL execution
- Command history and autocompletion

### HTTP & API

**REST API**
- Axum-based HTTP server
- Middleware support (Auth, CORS, Rate Limiting)
- Request validation
- Response transformation
- Error handling

**WebSocket**
- Real-time bidirectional communication
- Channel-based broadcasting
- Presence tracking
- Message queuing

**GraphQL**
- async-graphql integration
- Type-safe resolvers
- Subscriptions support
- Schema generation

### Authentication & Authorization

**Authentication Methods**
- JWT token-based auth
- Session-based auth
- OAuth/SSO (Google, GitHub, Facebook)

**Authorization**
- Role-Based Access Control (RBAC)
- Permission system
- Middleware guards
- Policy-based authorization

---

## Tier 1: Essential Features

### Mail System

**Features**:
- SMTP integration
- Template engine support
- Queue integration
- Multiple drivers (SMTP, Sendmail)
- Attachments support
- HTML/Plain text emails

**Usage**:
```bash
# Create mail class
foundry make:mail WelcomeEmail

# Send email
use foundry_mail::Mail;
Mail::to("user@example.com")
    .template("welcome")
    .send().await?;
```

### Notifications

**Channels**:
- Email
- SMS (Twilio, Nexmo)
- Slack
- Push notifications
- Database notifications

**Features**:
- Multi-channel delivery
- Notification preferences
- Read/unread tracking
- Notification templates

**Usage**:
```bash
foundry make:notification OrderShipped

# Send notification
user.notify(OrderShipped::new(order)).await?;
```

### Task Scheduling

**Features**:
- Cron-based scheduling
- Timezone support
- Job chaining
- Failure handling
- Schedule management

**Usage**:
```rust
schedule.hourly(|| {
    cleanup_old_records().await
});

schedule.cron("0 0 * * *", || {
    send_daily_report().await
});
```

### Caching Layer

**Drivers**:
- Redis (Production)
- File-based
- In-memory (Development)

**Features**:
- TTL support
- Cache tags
- Cache events
- Atomic operations
- Connection pooling (Redis)

**Usage**:
```rust
// Using cache
cache.put("user:1", &user, Duration::hours(1)).await?;
let user = cache.remember("user:1", Duration::hours(1), || {
    fetch_user_from_db(1)
}).await?;

// Configuration via .env
CACHE_DRIVER=redis
REDIS_URL=redis://127.0.0.1:6379
CACHE_PREFIX=app_cache:
```

### Job Queue System

**Backends**:
- Redis (Production) - Connection pooling, atomic operations
- In-memory (Development/Testing)

**Features**:
- Delayed job execution
- Job priority support
- Automatic retry with configurable attempts
- Worker process with graceful shutdown
- Multiple queue support
- Failed job tracking
- Custom job handlers

**Configuration**:
```bash
# Environment variables
QUEUE_DRIVER=redis
REDIS_URL=redis://127.0.0.1:6379
QUEUE_PREFIX=queue:
QUEUE_TIMEOUT=300
```

**Dispatching Jobs**:
```rust
use foundry_queue::prelude::*;
use serde_json::json;

// Simple job dispatch
let job = Job::new("send_email")
    .with_payload(json!({
        "to": "user@example.com",
        "subject": "Welcome!"
    }));

queue.dispatch(job).await?;

// Delayed job
let job = Job::new("cleanup")
    .with_delay(Duration::from_secs(3600));
queue.dispatch(job).await?;

// Priority job
let job = Job::new("urgent_task")
    .with_priority(10);
queue.dispatch(job).await?;
```

**Worker Process**:
```rust
use foundry_queue::prelude::*;

// Create worker
let mut worker = Worker::new(queue);

// Register custom job handler
worker.register_handler("send_email", EmailHandler);

// Run worker (processes jobs until stopped)
let stats = worker.run().await?;
println!("Processed: {}, Failed: {}", stats.processed, stats.failed);
```

**Custom Job Handlers**:
```rust
use async_trait::async_trait;
use foundry_queue::worker::JobHandler;

struct EmailHandler;

#[async_trait]
impl JobHandler for EmailHandler {
    async fn handle(&self, job: &Job) -> QueueResult<Option<Value>> {
        // Extract data from job.payload
        let to = job.payload["to"].as_str().unwrap();

        // Perform work
        send_email(to).await?;

        Ok(Some(json!({"sent": true})))
    }
}
```

**Commands**:
```bash
# Start queue worker
foundry queue:work

# View queue status
foundry queue:status

# Retry failed jobs
foundry queue:retry

# Clear failed jobs
foundry queue:flush-failed
```

### Multi-Tenancy

**Features**:
- Database-level isolation
- Domain-based routing
- Tenant configuration
- Cross-tenant queries (admin)

**Usage**:
```rust
// Automatic tenant scoping
let users = User::all().await?;  // Only current tenant's users

// Cross-tenant access (admin)
let all_users = User::without_tenant_scope().all().await?;
```

---

## Tier 2: Enterprise Features

### API Resources & Transformers

Transform models for API responses with pagination and filtering.

```rust
use foundry_resources::Resource;

#[derive(Resource)]
struct UserResource {
    id: i64,
    name: String,
    email: String,
}

// With pagination
let users = User::paginate(15).await?;
UserResource::collection(users).to_json()
```

### Soft Deletes

Logical deletion with restore capabilities.

```rust
use foundry_soft_deletes::SoftDeletes;

#[derive(Model, SoftDeletes)]
struct Post {
    id: i64,
    title: String,
    deleted_at: Option<DateTime>,
}

// Soft delete
post.delete().await?;

// Include trashed
let all_posts = Post::with_trashed().all().await?;

// Restore
post.restore().await?;
```

### Audit Logging

Complete change tracking for all models.

```rust
use foundry_audit::Auditable;

#[derive(Model, Auditable)]
struct User {
    id: i64,
    name: String,
}

// View audit log
let audits = user.audits().await?;
for audit in audits {
    println!("{} changed {} from {:?} to {:?}",
        audit.user, audit.field, audit.old_value, audit.new_value);
}
```

### Full-Text Search

**Drivers**:
- Database (PostgreSQL, MySQL)
- Elasticsearch

```rust
use foundry_search::Searchable;

#[derive(Model, Searchable)]
struct Article {
    id: i64,
    title: String,
    content: String,
}

// Search
let results = Article::search("rust framework").await?;

// Index management
foundry search:index Article
foundry search:reindex --force
```

### Broadcasting & WebSocket

Real-time event broadcasting across multiple channels.

```rust
use foundry_broadcast::Broadcast;

// Broadcast event
Broadcast::channel("chat.room.1")
    .send(MessageSent::new(message))
    .await?;

// Listen in JavaScript/TypeScript client
const channel = socket.channel("chat.room.1");
channel.on("MessageSent", (data) => {
    console.log(data);
});
```

### OAuth / SSO

Third-party authentication integration.

**Providers**:
- Google
- GitHub
- Facebook
- Microsoft
- Custom OAuth providers

```bash
# Configure OAuth
foundry oauth:list-providers
foundry oauth:test google

# Use in code
use foundry_oauth::OAuth;

let auth_url = OAuth::provider("google").redirect_url();
let user = OAuth::provider("google").user_from_code(code).await?;
```

### Configuration Management

Dynamic configuration with environment-specific overrides.

```rust
use foundry_config::Config;

let config = Config::load()?;
let db_url = config.get::<String>("database.url")?;
let max_conn = config.get::<u32>("database.max_connections")?;

// Environment-specific
let app_env = config.env(); // development, staging, production
```

### Rate Limiting

Request and user-based rate limiting.

```rust
use foundry_ratelimit::RateLimit;

// Middleware
app.route("/api/*", get(handler))
    .layer(RateLimit::per_minute(60));

// Manual check
if RateLimit::attempt("user:1", 10, Duration::minutes(1))? {
    // Allow request
} else {
    // Rate limit exceeded
}
```

### Localization / i18n

Multi-language support with translation management.

```bash
# Create translation file
foundry make:translation auth

# Use in code
trans!("auth.welcome", name => "John")
// Hello, John!

# Language switching
set_locale("de");
trans!("auth.welcome", name => "John")
// Hallo, John!
```

---

## Tier 3: Nice-to-Have Features

### Admin Panel

Filament/Nova-style admin dashboard with automatic CRUD generation.

```bash
# Generate admin resource
foundry make:admin-resource User

# Publish config
foundry admin:publish

# Access at http://localhost:8000/admin
```

### PDF/Excel Export

Data export and report generation.

```bash
# Export commands
foundry export:pdf users output.pdf
foundry export:excel posts output.xlsx
foundry export:csv orders output.csv

# In code
use foundry_export::{PdfExport, ExcelExport};

PdfExport::from_query(User::query())
    .template("users_report")
    .save("users.pdf").await?;
```

### Form Builder

HTML form generation with validation and themes.

```rust
use foundry_forms::Form;

let form = Form::for_model::<User>()
    .text("name", "Name")
    .email("email", "Email Address")
    .password("password", "Password")
    .submit("Register")
    .render()?;
```

### HTTP Client

Guzzle-style HTTP client with retry logic and authentication.

```rust
use foundry_http::Client;

let client = Client::new()
    .base_url("https://api.example.com")
    .bearer_token("secret")
    .retry(3);

let response = client.get("/users/1").send().await?;
let user: User = response.json().await?;
```

---

## Advanced Features

### Programmatic Command Execution

Execute commands programmatically from code.

```rust
use foundry_api::Artisan;

let artisan = Artisan::new(app);

// Single command
let result = artisan.call("migrate").dispatch().await?;

// With arguments
let result = artisan
    .call("make:model")
    .with_args(vec!["Post"])
    .dispatch().await?;

// Command chaining
let results = artisan.chain()
    .add("migrate")
    .add("db:seed")
    .add("cache:clear")
    .dispatch().await?;
```

### Verbosity Levels

Control output detail with verbosity flags.

```bash
foundry migrate -q      # Quiet
foundry migrate -v      # Verbose
foundry migrate -vv     # Very verbose
foundry migrate -vvv    # Debug
```

### Advanced Input Handling

Flexible argument parsing and validation.

```rust
use foundry_api::input::InputParser;

let parser = InputParser::from_args(&args);
let name = parser.option("name").required()?;
let age = parser.option("age").parse::<u32>()?;
let tags = parser.option_array("tag");
```

### Stub Customization

Customize code generation templates.

```bash
# Publish stubs
foundry vendor:publish --tag=stubs

# Customize in stubs/ directory
# Available variables: {{name}}, {{namespace}}, {{table}}, etc.
```

### Isolatable Commands

Prevent concurrent command execution.

```rust
use foundry_api::isolatable::CommandIsolation;

let isolation = CommandIsolation::new("migrate");
let _guard = isolation.lock()?;

// Migration runs exclusively
// Lock is released when guard is dropped
```

### Queued Commands

Dispatch commands to queue for async execution.

```rust
use foundry_api::queued_commands::{QueuedCommand, CommandQueue};

let queue = CommandQueue::default();

let cmd = QueuedCommand::new("import:data")
    .with_args(vec!["users.csv".to_string()])
    .with_delay(Duration::from_secs(60))
    .with_max_attempts(3);

let job_id = queue.dispatch(cmd).await?;
```

---

## Feature Comparison Matrix

| Feature | Tier | LOC | Status |
|---------|------|-----|--------|
| CLI & Code Gen | Core | 2,500+ | ✅ Complete |
| Database Management | Core | 1,800+ | ✅ Complete |
| Tinker REPL | Core | 1,200+ | ✅ Complete |
| Mail System | 1 | 1,809 | ✅ Complete |
| Notifications | 1 | 2,234 | ✅ Complete |
| Task Scheduling | 1 | 1,567 | ✅ Complete |
| Caching | 1 | 1,892 | ✅ Complete |
| Multi-Tenancy | 1 | 5,078 | ✅ Complete |
| API Resources | 2 | 892 | ✅ Complete |
| Soft Deletes | 2 | 456 | ✅ Complete |
| Audit Logging | 2 | 1,234 | ✅ Complete |
| Full-Text Search | 2 | 987 | ✅ Complete |
| Broadcasting | 2 | 1,456 | ✅ Complete |
| OAuth/SSO | 2 | 1,678 | ✅ Complete |
| Configuration | 2 | 567 | ✅ Complete |
| Rate Limiting | 2 | 678 | ✅ Complete |
| i18n | 2 | 789 | ✅ Complete |
| GraphQL | 2 | 1,123 | ✅ Complete |
| Testing Utils | 2 | 891 | ✅ Complete |
| Admin Panel | 3 | 2,345 | ✅ Complete |
| PDF/Excel Export | 3 | 1,234 | ✅ Complete |
| Form Builder | 3 | 890 | ✅ Complete |
| HTTP Client | 3 | 678 | ✅ Complete |

---

## Getting Started

For detailed usage of each feature, refer to:

- [Architecture Guide](ARCHITECTURE.md)
- [Command Reference](COMMANDS.md)
- [Tier System](TIER_SYSTEM.md)
- [Main README](../README.md)

---

*Last Updated: 2025-11-06*
*RustForge v0.2.0*
