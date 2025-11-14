# Migration Guide: v0.2.0 to v1.0.0

This guide will help you migrate your RustForge application from v0.2.0 (Beta) to v1.0.0 (Production Ready).

## Overview

RustForge v1.0.0 is a major release that introduces production-ready backends and many new features. While we've maintained API compatibility where possible, there are some breaking changes you need to be aware of.

**Key Changes:**
- In-memory Queue → Redis Queue (REQUIRED)
- In-memory Cache → Redis Cache (REQUIRED)
- New authentication features (Email Verification, Password Reset, Remember Me)
- Advanced ORM features (Scopes, Collections, Polymorphic Relations)
- Enhanced testing utilities
- Multi-channel Notifications
- Real-time Broadcasting with WebSockets
- AWS S3 Storage integration

**Estimated Migration Time:** 2-4 hours for a typical application

---

## Prerequisites

Before starting the migration, ensure you have:

1. **Rust 1.75+** - Check with `rustc --version`
2. **Redis 6.0+** - Required for queue and cache
3. **Backup** - Create a backup of your application
4. **Test Suite** - Ensure your tests are passing on v0.2.0

---

## Step 1: Update Dependencies

### Update Cargo.toml

Replace your old dependencies with the new v1.0.0 versions:

```toml
[dependencies]
# Core Framework
rf-core = "1.0"
rf-web = "1.0"
rf-config = "1.0"
rf-container = "1.0"

# Database & ORM
rf-orm = "1.0"
sea-orm = { version = "0.12", features = ["runtime-tokio-rustls", "sqlx-sqlite", "sqlx-postgres"] }

# Authentication & Security
rf-auth = "1.0"
rf-validation = "1.0"

# Infrastructure
foundry-queue = "1.0"
foundry-cache = "1.0"

# Background Jobs & Events
rf-jobs = "1.0"
rf-events = "1.0"

# Communication
rf-mail = "1.0"
rf-notifications = "1.0"

# Real-time
rf-broadcast = "1.0"
rf-broadcasting = "1.0"

# Storage
rf-storage = "1.0"

# Testing
rf-testing = "1.0"

# Async Runtime
tokio = { version = "1.37", features = ["macros", "rt-multi-thread", "signal"] }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Error Handling
anyhow = "1.0"
thiserror = "1.0"

# Redis (NEW - REQUIRED)
redis = { version = "0.24", features = ["aio", "tokio-comp", "connection-manager"] }
deadpool-redis = "0.14"

# AWS S3 (Optional)
aws-sdk-s3 = "1.0"  # If using S3 storage
```

### Run Cargo Update

```bash
cargo update
cargo build
```

Fix any compilation errors that arise. Most will be related to:
- Queue/Cache backend changes
- New async signatures
- Import path changes

---

## Step 2: Install and Configure Redis

### Install Redis

#### macOS
```bash
brew install redis
brew services start redis

# Verify installation
redis-cli ping  # Should return PONG
```

#### Ubuntu/Debian
```bash
sudo apt update
sudo apt install redis-server

# Start Redis
sudo systemctl start redis-server
sudo systemctl enable redis-server

# Verify installation
redis-cli ping  # Should return PONG
```

#### Docker
```bash
docker run -d \
  --name rustforge-redis \
  -p 6379:6379 \
  -v rustforge-redis-data:/data \
  redis:latest

# Verify installation
docker exec rustforge-redis redis-cli ping  # Should return PONG
```

### Configure Redis in .env

Add these environment variables to your `.env` file:

```env
# Redis Configuration (REQUIRED)
REDIS_URL=redis://localhost:6379
REDIS_PASSWORD=  # Leave empty for local development
REDIS_DB=0  # Default database

# Queue Configuration
QUEUE_DRIVER=redis  # Changed from "memory"
QUEUE_CONNECTION_POOL_SIZE=10
QUEUE_RETRY_ATTEMPTS=3
QUEUE_RETRY_DELAY=5  # seconds

# Cache Configuration
CACHE_DRIVER=redis  # Changed from "memory"
CACHE_CONNECTION_POOL_SIZE=20
CACHE_PREFIX=app_cache
CACHE_DEFAULT_TTL=3600  # 1 hour in seconds
```

---

## Step 3: Migrate Queue Backend

### Before (v0.2.0) - In-Memory Queue

```rust
use foundry_queue::QueueManager;

// Old way - in-memory queue (NOT PRODUCTION READY)
let queue = QueueManager::memory();

// Dispatch job
queue.dispatch(job).await?;
```

### After (v1.0.0) - Redis Queue

```rust
use foundry_queue::QueueManager;
use std::env;

// New way - Redis queue (PRODUCTION READY)
let redis_url = env::var("REDIS_URL")?;
let queue = QueueManager::redis(&redis_url).await?;

// Dispatch job (same API)
queue.dispatch(job).await?;

// New: Delayed jobs
queue.dispatch_delayed(job, Duration::from_secs(300)).await?;

// New: Priority queues
queue.dispatch_with_priority(job, Priority::High).await?;
```

### Configuration-Based Initialization

For cleaner code, use configuration-based initialization:

```rust
use foundry_queue::QueueManager;
use rf_config::AppConfig;

let config = AppConfig::from_env()?;
let queue = QueueManager::from_config(&config.queue).await?;
```

### Worker Changes

If you have custom workers:

```rust
// Before (v0.2.0)
queue.process("default", |job| async move {
    // Handle job
    Ok(())
}).await?;

// After (v1.0.0) - More features
let worker = queue.worker("default")
    .max_jobs(100)  // Process up to 100 jobs before restarting
    .timeout(Duration::from_secs(60))  // Job timeout
    .build();

worker.run(|job, ctx| async move {
    // ctx provides: job_id, attempt, is_final_attempt()
    ctx.log(&format!("Processing job: {}", job.name));

    // Your job logic here
    process_job(&job).await?;

    Ok(())
}).await?;
```

---

## Step 4: Migrate Cache Backend

### Before (v0.2.0) - In-Memory Cache

```rust
use foundry_cache::CacheManager;

// Old way - in-memory cache
let cache = CacheManager::memory();

// Get/Set
cache.put("key", &value, Some(Duration::from_secs(3600))).await?;
let value: MyType = cache.get("key").await?;
```

### After (v1.0.0) - Redis Cache

```rust
use foundry_cache::CacheManager;
use std::env;

// New way - Redis cache
let redis_url = env::var("REDIS_URL")?;
let cache = CacheManager::redis(&redis_url).await?;

// Get/Set (same API)
cache.put("key", &value, Some(Duration::from_secs(3600))).await?;
let value: MyType = cache.get("key").await?;

// New: Cache tags
cache.tags(&["users", "posts"])
    .put("user:1:posts", &posts, None).await?;

// Invalidate all tagged cache
cache.tags(&["users"]).flush().await?;

// New: Remember pattern
let user = cache.remember("user:1", Duration::from_secs(3600), || async {
    database.find_user(1).await
}).await?;

// New: Increment/Decrement
cache.increment("page_views", 1).await?;
cache.decrement("stock:item:5", 1).await?;
```

---

## Step 5: Update Authentication Features

### Email Verification (NEW)

Add email verification to your user registration flow:

```rust
use rf_auth::verification::EmailVerification;

// After user registration
#[derive(Deserialize)]
struct RegisterRequest {
    name: String,
    email: String,
    password: String,
}

async fn register(
    Json(req): Json<RegisterRequest>,
) -> Result<Json<serde_json::Value>> {
    // Create user
    let user = User::create(&db, req).await?;

    // Generate verification token
    let token = EmailVerification::generate_token(user.id, &user.email)?;

    // Send verification email
    EmailVerification::send_verification_email(&user, &token, &mailer).await?;

    Ok(Json(json!({
        "message": "Registration successful. Please check your email to verify your account.",
        "user_id": user.id
    })))
}

// Verification endpoint
async fn verify_email(Path(token): Path<String>) -> Result<Response> {
    let claims = EmailVerification::verify_token(&token)?;
    let user = User::find_by_id(claims.user_id).one(&db).await?;
    user.mark_email_as_verified(&db).await?;

    Ok(Redirect::to("/dashboard").into_response())
}

// Protect routes with RequireVerified middleware
let app = Router::new()
    .route("/dashboard", get(dashboard))
    .layer(RequireVerifiedMiddleware::new());
```

Add to .env:
```env
EMAIL_VERIFICATION_TTL=86400  # 24 hours
EMAIL_VERIFICATION_SECRET=your-secret-key-min-32-chars
EMAIL_VERIFICATION_URL=https://yourapp.com/verify
```

### Password Reset (NEW)

Implement password reset functionality:

```rust
use rf_auth::password_reset::PasswordReset;

// Request password reset
async fn forgot_password(
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<Json<serde_json::Value>> {
    let user = User::find_by_email(&req.email, &db).await?
        .ok_or(AppError::NotFound)?;

    let token = PasswordReset::create_token(user.id, &user.email)?;
    PasswordReset::send_reset_email(&user, &token, &mailer).await?;

    Ok(Json(json!({"message": "Password reset email sent"})))
}

// Reset password
async fn reset_password(
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>> {
    let claims = PasswordReset::verify_token(&req.token)?;
    let user = User::find_by_id(claims.user_id).one(&db).await?;

    let hasher = PasswordHasher::new(HashAlgorithm::Argon2);
    let hashed = hasher.hash_password(&req.new_password)?;
    user.update_password(&hashed, &db).await?;

    PasswordReset::invalidate_token(&req.token, &redis).await?;

    Ok(Json(json!({"message": "Password reset successfully"})))
}
```

Add to .env:
```env
PASSWORD_RESET_TTL=3600  # 1 hour
PASSWORD_RESET_SECRET=your-secret-key-min-32-chars
PASSWORD_HASH_ALGORITHM=argon2
```

### Remember Me (NEW)

Add "Remember Me" functionality to login:

```rust
use rf_auth::remember::RememberMe;

#[derive(Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    remember_me: bool,  // NEW
}

async fn login(
    Json(req): Json<LoginRequest>,
) -> Result<impl IntoResponse> {
    let user = User::authenticate(&req.email, &req.password, &db).await?;
    let access_token = create_access_token(&user)?;

    let mut response = Json(json!({
        "access_token": access_token,
        "user": user,
    })).into_response();

    // NEW: Remember me functionality
    if req.remember_me {
        let remember_token = RememberMe::create_token(user.id)?;
        let cookie = RememberMe::create_cookie(&remember_token)?;

        response.headers_mut().insert(
            header::SET_COOKIE,
            cookie.to_string().parse()?,
        );
    }

    Ok(response)
}

// Add RememberMe middleware to your app
let app = Router::new()
    .route("/", get(home))
    .layer(RememberMeMiddleware::new());
```

Add to .env:
```env
REMEMBER_ME_TTL=2592000  # 30 days
REMEMBER_ME_SECRET=your-secret-key-min-32-chars
REMEMBER_ME_SECURE=true  # Use HTTPS in production
```

---

## Step 6: Adopt New ORM Features

### Query Scopes (NEW)

Replace repetitive query logic with reusable scopes:

```rust
use rf_orm::scopes::*;

// Before (v0.2.0) - Repetitive queries
let active_users = User::find()
    .filter(user::Column::Status.eq("active"))
    .all(&db).await?;

let verified_admins = User::find()
    .filter(user::Column::Status.eq("active"))
    .filter(user::Column::EmailVerifiedAt.is_not_null())
    .filter(user::Column::Role.eq("admin"))
    .all(&db).await?;

// After (v1.0.0) - Define scopes once, use everywhere
define_scopes! {
    UserScopes for User {
        active(query) {
            query.filter(user::Column::Status.eq("active"))
        }

        verified(query) {
            query.filter(user::Column::EmailVerifiedAt.is_not_null())
        }

        by_role(query, role: &str) {
            query.filter(user::Column::Role.eq(role))
        }
    }
}

// Use scopes
let active_users = User::find()
    .scope("active")
    .all(&db).await?;

let verified_admins = User::find()
    .scope("active")
    .scope("verified")
    .scope_with("by_role", "admin")
    .all(&db).await?;
```

### Laravel Collections (NEW)

Use collection methods for data transformation:

```rust
use rf_orm::collections::Collection;

// Load users
let users = User::find().all(&db).await?;
let users = Collection::from(users);

// Transform data with fluent API
let active_emails = users
    .filter(|u| u.is_active)
    .pluck("email")
    .unique();

// Aggregate data
let total_age = users.sum(|u| u.age);
let avg_age = users.avg(|u| u.age);

// Group by field
let users_by_role = users.group_by(|u| u.role.clone());

// Chunk for pagination
let chunks = users.chunk(50);
for chunk in chunks {
    process_batch(&chunk).await?;
}
```

---

## Step 7: Update Testing

### Database Assertions (NEW)

Replace manual database checks with assertions:

```rust
use rf_testing::assertions::*;

// Before (v0.2.0) - Manual checks
#[tokio::test]
async fn test_user_creation() {
    let user = create_user(&db).await?;
    let found = User::find_by_id(user.id).one(&db).await?;
    assert!(found.is_some());
    assert_eq!(found.unwrap().email, "test@example.com");
}

// After (v1.0.0) - Clean assertions
#[tokio::test]
async fn test_user_creation() {
    let user = create_user(&db).await?;

    assert_database_has!(db, "users", {
        "id": user.id,
        "email": "test@example.com"
    });

    assert_database_count!(db, "users", 1);
}

#[tokio::test]
async fn test_user_deletion() {
    let user = create_user(&db).await?;
    user.delete(&db).await?;

    assert_database_missing!(db, "users", {
        "id": user.id
    });
}
```

### Queue and Event Fakes (NEW)

Test job and event dispatching without side effects:

```rust
use rf_testing::fakes::{QueueFake, EventFake};

#[tokio::test]
async fn test_job_dispatch() {
    let queue = QueueFake::new();

    // Your code that dispatches jobs
    dispatch_welcome_email(&user, &queue).await?;

    // Assert job was dispatched
    queue.assert_pushed("send_email");
    queue.assert_pushed_times("send_email", 1);

    // Inspect payload
    let jobs = queue.get_pushed_jobs();
    assert_eq!(jobs[0].payload["to"], "user@example.com");
}

#[tokio::test]
async fn test_event_dispatch() {
    let events = EventFake::new();

    // Your code that fires events
    create_order(&order_data, &events).await?;

    // Assert events were fired in order
    events.assert_dispatched("order.created");
    events.assert_dispatched("payment.processed");
    events.assert_dispatched("order.shipped");
}
```

---

## Step 8: Add New Features (Optional)

### Notifications (NEW)

Send multi-channel notifications:

```rust
use rf_notifications::*;

// Define notification
pub struct WelcomeNotification {
    user_id: i32,
}

impl Notification for WelcomeNotification {
    fn via(&self, _notifiable: &dyn Notifiable) -> Vec<Channel> {
        vec![Channel::Mail, Channel::Database]
    }

    fn to_mail(&self) -> MailMessage {
        MailMessage::new()
            .subject("Welcome!")
            .greeting("Hello!")
            .line("Welcome to our platform.")
            .action("Get Started", "https://app.com/start")
    }

    fn to_database(&self) -> DatabaseNotification {
        DatabaseNotification::new()
            .data(json!({"message": "Welcome!"}))
    }
}

// Send notification
user.notify(WelcomeNotification { user_id: user.id }, &notifier).await?;

// Check unread notifications
let unread = user.unread_notifications_count(&db).await?;
let notifications = user.get_unread_notifications(10, &db).await?;
```

### Broadcasting & WebSockets (NEW)

Add real-time features:

```rust
use rf_broadcast::*;

// Define broadcastable event
pub struct NewMessage {
    pub channel_id: i32,
    pub message: String,
    pub user: String,
}

impl Broadcast for NewMessage {
    fn broadcast_on(&self) -> Vec<Channel> {
        vec![Channel::private(format!("chat.{}", self.channel_id))]
    }
}

// Broadcast event
broadcaster.broadcast(NewMessage {
    channel_id: 1,
    message: "Hello!".to_string(),
    user: "John".to_string(),
}).await?;

// Start WebSocket server
let ws_server = WebSocketServer::new("127.0.0.1:6001");
ws_server.start().await?;
```

### AWS S3 Storage (NEW)

Integrate cloud storage:

```rust
use rf_storage::*;

// Configure S3
let storage = StorageManager::from_env()?;
let s3 = storage.disk("s3");

// Upload file
s3.put("uploads/avatar.jpg", &image_bytes).await?;

// Download file
let bytes = s3.get("uploads/avatar.jpg").await?;

// Generate presigned URL (15 minutes)
let url = s3.presigned_url("uploads/avatar.jpg", Duration::from_secs(900)).await?;

// List files
let files = s3.list("uploads/").await?;
```

Add to .env:
```env
STORAGE_DRIVER=s3
AWS_ACCESS_KEY_ID=your-access-key
AWS_SECRET_ACCESS_KEY=your-secret-key
AWS_DEFAULT_REGION=us-east-1
AWS_BUCKET=my-bucket
```

---

## Step 9: Run Tests

After making all changes, run your test suite:

```bash
# Run all tests
cargo test --all

# Run specific test
cargo test test_user_creation

# Run with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'
```

Fix any failing tests. Common issues:
- Queue/Cache backend not initialized
- Redis not running
- Missing environment variables
- Import path changes

---

## Step 10: Update Deployment

### Docker Compose

Update your `docker-compose.yml` to include Redis:

```yaml
version: '3.8'

services:
  app:
    build: .
    ports:
      - "8000:8000"
    environment:
      - REDIS_URL=redis://redis:6379
      - QUEUE_DRIVER=redis
      - CACHE_DRIVER=redis
    depends_on:
      - redis
      - postgres

  # NEW: Redis service
  redis:
    image: redis:latest
    ports:
      - "6379:6379"
    volumes:
      - redis-data:/data
    command: redis-server --appendonly yes

  postgres:
    image: postgres:15
    environment:
      POSTGRES_USER: app
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: app_db
    volumes:
      - postgres-data:/var/lib/postgresql/data

volumes:
  redis-data:
  postgres-data:
```

### Kubernetes

Add Redis to your Kubernetes deployment:

```yaml
# redis-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: redis
spec:
  selector:
    matchLabels:
      app: redis
  template:
    metadata:
      labels:
        app: redis
    spec:
      containers:
      - name: redis
        image: redis:latest
        ports:
        - containerPort: 6379
        volumeMounts:
        - name: redis-storage
          mountPath: /data
      volumes:
      - name: redis-storage
        persistentVolumeClaim:
          claimName: redis-pvc

---
apiVersion: v1
kind: Service
metadata:
  name: redis
spec:
  selector:
    app: redis
  ports:
  - port: 6379
    targetPort: 6379
```

---

## Breaking Changes

### API Changes

1. **Queue Initialization**
   - `QueueManager::memory()` → `QueueManager::redis(&url).await?`
   - Now requires async initialization

2. **Cache Initialization**
   - `CacheManager::memory()` → `CacheManager::redis(&url).await?`
   - Now requires async initialization

3. **Job Trait Signature**
   - Old: `fn handle(&self) -> Result<()>`
   - New: `async fn execute(&self, ctx: &JobContext) -> Result<(), JobError>`

4. **Import Paths**
   - Some modules reorganized for clarity
   - Update imports as needed

### Deprecated Features

The following are deprecated and will be removed in v2.0.0:

- `foundry-queue/src/backends/memory.rs` - Use Redis backend
- `foundry-cache/src/backends/memory.rs` - Use Redis backend
- Blocking file I/O - Use async APIs

---

## Troubleshooting

### Redis Connection Errors

**Error:** `Connection refused (os error 111)`

**Solution:**
```bash
# Check if Redis is running
redis-cli ping

# Start Redis if not running
# macOS: brew services start redis
# Linux: sudo systemctl start redis-server
# Docker: docker start rustforge-redis
```

### Compilation Errors

**Error:** `cannot find type QueueManager in this scope`

**Solution:**
```rust
// Add missing import
use foundry_queue::QueueManager;
```

**Error:** `trait bound Job: Send not satisfied`

**Solution:**
```rust
// Make sure your Job struct is Send + Sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyJob {
    // Your fields
}

// Make sure any referenced data is also Send + Sync
```

### Test Failures

**Error:** Tests failing with `RedisError`

**Solution:**
```bash
# Make sure Redis is running before tests
redis-cli ping

# Or use test database
REDIS_URL=redis://localhost:6379/15 cargo test
```

### Performance Issues

**Issue:** Queue processing seems slow

**Solution:**
```rust
// Increase worker count
let worker = queue.worker("default")
    .workers(num_cpus::get())  // Use all CPU cores
    .build();

// Or increase connection pool size
QUEUE_CONNECTION_POOL_SIZE=20
```

---

## Migration Checklist

Use this checklist to track your migration progress:

- [ ] Updated all dependencies to v1.0.0
- [ ] Installed and configured Redis
- [ ] Updated .env file with Redis configuration
- [ ] Migrated Queue backend from memory to Redis
- [ ] Migrated Cache backend from memory to Redis
- [ ] Added Email Verification (if needed)
- [ ] Added Password Reset (if needed)
- [ ] Added Remember Me (if needed)
- [ ] Adopted Query Scopes for repetitive queries
- [ ] Using Laravel Collections for data transformation
- [ ] Updated tests with new assertion helpers
- [ ] Updated tests with Queue/Event fakes
- [ ] Added Notifications (if needed)
- [ ] Added Broadcasting/WebSockets (if needed)
- [ ] Configured AWS S3 Storage (if needed)
- [ ] All tests passing
- [ ] Updated Docker/Kubernetes configuration
- [ ] Updated CI/CD pipeline
- [ ] Deployed to staging environment
- [ ] Smoke tested in staging
- [ ] Ready for production deployment

---

## Getting Help

If you encounter issues during migration:

1. **Check Documentation**: [docs/](../docs/)
2. **Review Examples**: Check the [examples/](../examples/) directory
3. **Search Issues**: https://github.com/Chregu12/RustForge/issues
4. **Ask Questions**: Open a new issue with the `question` label
5. **Community**: Join our Discord (coming soon)

---

## Conclusion

Congratulations! You've successfully migrated to RustForge v1.0.0. Your application now benefits from:

- Production-ready Redis backends
- Enhanced security features
- Advanced ORM capabilities
- Comprehensive testing utilities
- Multi-channel notifications
- Real-time broadcasting
- Cloud storage integration

**Next Steps:**
- Review the [CHANGELOG.md](../CHANGELOG.md) for all new features
- Check out [RELEASE_NOTES_v1.0.0.md](RELEASE_NOTES_v1.0.0.md) for highlights
- Read the [DEPLOYMENT_GUIDE.md](DEPLOYMENT_GUIDE.md) for production deployment
- Explore new features in your application

Happy coding with RustForge v1.0.0!
