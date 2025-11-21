# RustForge Best Practices Guide

**A comprehensive guide to writing production-ready RustForge applications**

---

## Table of Contents

1. [Project Structure](#project-structure)
2. [Naming Conventions](#naming-conventions)
3. [Error Handling](#error-handling)
4. [Security Best Practices](#security-best-practices)
5. [Performance Optimization](#performance-optimization)
6. [Testing Strategies](#testing-strategies)
7. [Code Organization](#code-organization)
8. [Database Best Practices](#database-best-practices)
9. [API Design](#api-design)
10. [Deployment](#deployment)

---

## Project Structure

### Recommended Structure

```
my-app/
├── src/
│   ├── main.rs                 # Application entry point
│   ├── lib.rs                  # Library root (for testing)
│   ├── routes.rs               # Route definitions
│   │
│   ├── controllers/            # Request handlers
│   │   ├── mod.rs
│   │   ├── user_controller.rs
│   │   └── post_controller.rs
│   │
│   ├── models/                 # Database models
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   └── post.rs
│   │
│   ├── services/               # Business logic
│   │   ├── mod.rs
│   │   ├── user_service.rs
│   │   └── email_service.rs
│   │
│   ├── repositories/           # Data access layer
│   │   ├── mod.rs
│   │   └── user_repository.rs
│   │
│   ├── middleware/             # Custom middleware
│   │   ├── mod.rs
│   │   ├── auth.rs
│   │   └── rate_limit.rs
│   │
│   ├── requests/               # Form validation
│   │   ├── mod.rs
│   │   └── user_request.rs
│   │
│   ├── resources/              # API resources
│   │   ├── mod.rs
│   │   └── user_resource.rs
│   │
│   ├── jobs/                   # Background jobs
│   │   ├── mod.rs
│   │   └── send_email_job.rs
│   │
│   ├── events/                 # Event definitions
│   │   ├── mod.rs
│   │   └── user_registered.rs
│   │
│   ├── listeners/              # Event listeners
│   │   ├── mod.rs
│   │   └── send_welcome_email.rs
│   │
│   ├── policies/               # Authorization policies
│   │   ├── mod.rs
│   │   └── post_policy.rs
│   │
│   ├── views/                  # Blade templates
│   │   ├── layouts/
│   │   │   └── app.blade.html
│   │   ├── users/
│   │   │   ├── index.blade.html
│   │   │   └── show.blade.html
│   │   └── posts/
│   │
│   └── utils/                  # Helper functions
│       ├── mod.rs
│       └── date.rs
│
├── migrations/                 # Database migrations
├── seeders/                    # Database seeders
├── tests/                      # Tests
│   ├── integration/
│   └── unit/
├── public/                     # Static files
├── storage/                    # File storage
│   ├── app/
│   ├── logs/
│   └── framework/
├── .env                        # Environment variables
├── .env.example                # Example environment
└── Cargo.toml                  # Dependencies
```

### Principles

1. **Separation of Concerns**: Controllers handle requests, services contain business logic, repositories access data
2. **Single Responsibility**: Each module has one clear purpose
3. **Dependency Direction**: Dependencies flow inward (controllers → services → repositories)
4. **Testability**: Structure makes unit testing easy

---

## Naming Conventions

### Files and Modules

```rust
// ✅ Good: snake_case for files
user_controller.rs
post_service.rs
email_job.rs

// ❌ Bad: camelCase or PascalCase
UserController.rs
postService.rs
```

### Types

```rust
// ✅ Good: PascalCase for types
struct User { }
enum Status { }
trait Authenticatable { }

// ❌ Bad: snake_case or camelCase
struct user { }
enum status { }
```

### Functions and Variables

```rust
// ✅ Good: snake_case
fn get_user_by_email() { }
let total_count = 10;

// ❌ Bad: camelCase or PascalCase
fn GetUserByEmail() { }
let TotalCount = 10;
```

### Constants

```rust
// ✅ Good: SCREAMING_SNAKE_CASE
const MAX_UPLOAD_SIZE: usize = 10_000_000;
const API_VERSION: &str = "v1";

// ❌ Bad: lowercase or PascalCase
const max_upload_size: usize = 10_000_000;
const ApiVersion: &str = "v1";
```

### Route Names

```rust
// ✅ Good: Descriptive, RESTful
.route("/users", Route::get(user_controller::index))           // users.index
.route("/users/:id", Route::get(user_controller::show))        // users.show
.route("/users", Route::post(user_controller::store))          // users.store

// ❌ Bad: Unclear, inconsistent
.route("/u", Route::get(get_users))
.route("/user/:id", Route::get(show))
```

---

## Error Handling

### Use Result Types

```rust
// ✅ Good: Return Result for operations that can fail
pub async fn create_user(data: UserData) -> Result<User, AppError> {
    let user = User::create(db, data).await?;
    Ok(user)
}

// ❌ Bad: Panic on errors
pub async fn create_user(data: UserData) -> User {
    User::create(db, data).await.unwrap() // ❌ Never use unwrap in production!
}
```

### Custom Error Types

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("User not found")]
    UserNotFound,

    #[error("Validation failed: {0}")]
    Validation(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Internal server error")]
    Internal,
}

// Implement response conversion
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::UserNotFound => (StatusCode::NOT_FOUND, self.to_string()),
            AppError::Validation(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            AppError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "Internal error".to_string()),
        }.into_response()
    }
}
```

### Error Propagation

```rust
// ✅ Good: Use ? operator
pub async fn update_user(id: i32, data: UserData) -> Result<User, AppError> {
    let user = User::find(id, db).await?;
    user.update(db, data).await?;
    Ok(user)
}

// ❌ Bad: Manual error handling
pub async fn update_user(id: i32, data: UserData) -> Result<User, AppError> {
    let user = match User::find(id, db).await {
        Ok(u) => u,
        Err(e) => return Err(e.into()),
    };
    match user.update(db, data).await {
        Ok(u) => Ok(u),
        Err(e) => Err(e.into()),
    }
}
```

### Logging Errors

```rust
use tracing::{error, warn};

pub async fn process_payment(order: &Order) -> Result<Payment, AppError> {
    match charge_card(&order.card_token, order.total).await {
        Ok(payment) => Ok(payment),
        Err(e) => {
            error!("Payment failed for order {}: {}", order.id, e);
            Err(AppError::PaymentFailed(e.to_string()))
        }
    }
}
```

---

## Security Best Practices

### 1. Input Validation

```rust
// ✅ Always validate user input
pub async fn store(req: Request) -> Result<Response, AppError> {
    let validated = req.validate(|v| {
        v.rule("email", vec![Required, Email, MaxLength(255)])
         .rule("password", vec![Required, MinLength(8)])
    }).await?;

    // validated data is safe to use
}

// ❌ Never trust user input directly
pub async fn store(req: Request) -> Result<Response, AppError> {
    let email = req.input::<String>("email")?; // Unvalidated!
    User::create(db, email).await?; // Dangerous!
}
```

### 2. SQL Injection Prevention

```rust
// ✅ Good: Use query builder (parameterized)
let users = User::query()
    .filter(user::Column::Email.eq(email))
    .all(db)
    .await?;

// ❌ Bad: Raw SQL with string interpolation
let query = format!("SELECT * FROM users WHERE email = '{}'", email);
db.execute_raw(&query).await?; // SQL injection risk!
```

### 3. Password Hashing

```rust
use rf_hashing::Hash;

// ✅ Good: Always hash passwords
let hashed = Hash::make(&password)?;
user.password = hashed;

// ❌ Bad: Store plain text passwords
user.password = password; // Never do this!
```

### 4. CSRF Protection

```rust
// ✅ Good: Enable CSRF middleware
Router::new()
    .middleware(CsrfMiddleware::new())
    .route("/users", Route::post(user_controller::store));

// In forms
<form method="POST">
    @csrf
    <!-- form fields -->
</form>
```

### 5. Rate Limiting

```rust
// ✅ Good: Rate limit sensitive endpoints
Router::new()
    .route("/api/login", Route::post(auth::login))
    .middleware(RateLimit::new(5, Duration::minutes(1))); // 5 attempts per minute
```

### 6. Secure Headers

```rust
// ✅ Good: Add security headers
Router::new()
    .middleware(SecureHeaders::new()
        .hsts()
        .x_frame_options("DENY")
        .x_content_type_options("nosniff")
        .csp("default-src 'self'")
    );
```

### 7. Environment Variables

```rust
// ✅ Good: Never commit secrets
# .env (gitignored)
DATABASE_URL=postgres://user:pass@localhost/db
API_KEY=secret-key-here

# .env.example (committed)
DATABASE_URL=postgres://user:password@localhost/database
API_KEY=your-api-key-here
```

---

## Performance Optimization

### 1. Database Query Optimization

```rust
// ❌ Bad: N+1 queries
let users = User::all(db).await?;
for user in users {
    let posts = user.posts().get(db).await?; // N queries!
}

// ✅ Good: Eager loading
let users = User::with("posts", db).await?;
for user in users {
    let posts = &user.posts; // Already loaded!
}
```

### 2. Select Only Needed Columns

```rust
// ❌ Bad: Select all columns
let users = User::all(db).await?;

// ✅ Good: Select only needed columns
let users = User::query()
    .select_only()
    .column(user::Column::Id)
    .column(user::Column::Name)
    .all(db)
    .await?;
```

### 3. Caching

```rust
use rf_cache::Cache;

// ✅ Good: Cache expensive queries
let users = Cache::remember("users:all", 3600, || async {
    User::all(db).await
}).await?;

// Cache with tags for easy invalidation
Cache::tags(vec!["users"])
    .remember("users:active", 3600, || async {
        User::where_eq("active", true, db).await
    })
    .await?;

// Invalidate when data changes
pub async fn update_user(user: User) -> Result<User, AppError> {
    let updated = user.save(db).await?;
    Cache::tags(vec!["users"]).flush().await?; // Invalidate cache
    Ok(updated)
}
```

### 4. Background Jobs

```rust
// ❌ Bad: Send email synchronously
pub async fn register(req: Request) -> Response {
    let user = User::create(db, user_data).await?;
    send_welcome_email(&user).await?; // Blocks response!
    Response::redirect("/dashboard")
}

// ✅ Good: Queue email job
pub async fn register(req: Request) -> Response {
    let user = User::create(db, user_data).await?;
    SendWelcomeEmailJob::new(user.id).dispatch().await?; // Non-blocking!
    Response::redirect("/dashboard")
}
```

### 5. Database Indexes

```rust
// In migration
manager.create_index(
    Index::create()
        .name("idx_users_email")
        .table(User::Table)
        .col(User::Email) // Index frequently queried columns
        .to_owned()
).await?;
```

### 6. Connection Pooling

```rust
// Configure in .env
DATABASE_MAX_CONNECTIONS=20
DATABASE_MIN_CONNECTIONS=5
DATABASE_ACQUIRE_TIMEOUT=30
```

---

## Testing Strategies

### 1. Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_full_name() {
        let user = User {
            first_name: "John".to_string(),
            last_name: "Doe".to_string(),
            ..Default::default()
        };

        assert_eq!(user.full_name(), "John Doe");
    }
}
```

### 2. Integration Tests

```rust
#[tokio::test]
async fn test_create_user() {
    let db = setup_test_db().await;

    let user = User::create(&db, UserData {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    }).await.unwrap();

    assert_eq!(user.name, "Test User");
    assert!(user.id > 0);
}
```

### 3. HTTP Tests

```rust
#[tokio::test]
async fn test_user_registration() {
    let app = create_test_app().await;

    let response = app
        .post("/register")
        .json(&json!({
            "name": "Test User",
            "email": "test@example.com",
            "password": "password123",
        }))
        .send()
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
}
```

### 4. Use Factories

```rust
// ✅ Good: Use factories for test data
let user = UserFactory::create(db).await?;
let admin = UserFactory::new().with("role", "admin").create(db).await?;

// ❌ Bad: Manual test data creation
let user = User {
    id: 1,
    name: "Test".to_string(),
    email: "test@example.com".to_string(),
    // ... lots of fields
};
```

### 5. Test Coverage

```bash
# Run tests with coverage
cargo tarpaulin --out Html

# Aim for 80%+ coverage on critical paths
```

---

## Code Organization

### 1. Keep Controllers Thin

```rust
// ❌ Bad: Fat controller
pub async fn store(req: Request) -> Response {
    let validated = req.validate(...).await?;
    let hashed_password = Hash::make(&validated.password)?;
    let user = User::create(db, UserData { ... }).await?;
    send_welcome_email(&user).await?;
    update_statistics().await?;
    log_user_creation(&user);
    Response::redirect("/dashboard")
}

// ✅ Good: Thin controller, delegate to service
pub async fn store(req: Request) -> Response {
    let validated = req.validate(...).await?;
    let user = UserService::register(validated).await?;
    Response::redirect("/dashboard")
}
```

### 2. Use Services for Business Logic

```rust
pub struct UserService;

impl UserService {
    pub async fn register(data: UserData) -> Result<User, AppError> {
        // Hash password
        let hashed = Hash::make(&data.password)?;

        // Create user
        let user = User::create(db, UserData {
            password: hashed,
            ..data
        }).await?;

        // Send welcome email (async)
        SendWelcomeEmailJob::new(user.id).dispatch().await?;

        // Update statistics
        Statistics::increment("total_users").await?;

        Ok(user)
    }
}
```

### 3. Repository Pattern

```rust
pub struct UserRepository {
    db: DatabaseConnection,
}

impl UserRepository {
    pub async fn find_by_email(&self, email: &str) -> Result<Option<User>, DbErr> {
        User::query()
            .filter(user::Column::Email.eq(email))
            .one(&self.db)
            .await
    }

    pub async fn active_users(&self) -> Result<Vec<User>, DbErr> {
        User::query()
            .filter(user::Column::Active.eq(true))
            .order_by_desc(user::Column::CreatedAt)
            .all(&self.db)
            .await
    }
}
```

---

## Database Best Practices

### 1. Use Migrations

```bash
# ✅ Good: Version control your schema
forge make:migration create_users_table
forge migrate

# ❌ Bad: Manual SQL changes
psql -c "ALTER TABLE users ADD COLUMN age INT"
```

### 2. Foreign Keys

```rust
// ✅ Good: Define foreign key constraints
manager.create_table(
    Table::create()
        .table(Post::Table)
        .col(ColumnDef::new(Post::UserId).integer().not_null())
        .foreign_key(
            ForeignKey::create()
                .from(Post::Table, Post::UserId)
                .to(User::Table, User::Id)
                .on_delete(ForeignKeyAction::Cascade)
        )
        .to_owned()
).await?;
```

### 3. Soft Deletes for Critical Data

```rust
#[derive(Model)]
#[soft_deletes]
pub struct Order {
    pub id: i32,
    pub total: f64,
    pub deleted_at: Option<DateTime<Utc>>,
}
```

### 4. Use Transactions

```rust
// ✅ Good: Use transactions for multi-step operations
db.transaction(|txn| {
    Box::pin(async move {
        Order::create(txn, order_data).await?;
        Inventory::decrement_stock(txn, product_id, quantity).await?;
        Ok(())
    })
}).await?;
```

---

## API Design

### 1. Versioning

```rust
// ✅ Good: Version your API
Router::new()
    .nest("/api/v1", v1_routes())
    .nest("/api/v2", v2_routes());
```

### 2. Consistent Response Format

```rust
// ✅ Good: Consistent JSON structure
#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub meta: Option<Meta>,
}
```

### 3. HTTP Status Codes

```rust
// ✅ Use appropriate status codes
Response::ok() // 200
Response::created() // 201
Response::no_content() // 204
Response::bad_request() // 400
Response::unauthorized() // 401
Response::forbidden() // 403
Response::not_found() // 404
Response::unprocessable_entity() // 422
Response::internal_server_error() // 500
```

### 4. Pagination

```rust
// ✅ Good: Paginate large collections
let page = req.query("page").unwrap_or(1);
let users = User::query()
    .paginate(db, 15)
    .fetch_page(page)
    .await?;
```

---

## Deployment

### 1. Environment Configuration

```bash
# .env.production
APP_ENV=production
APP_DEBUG=false
APP_URL=https://myapp.com

DATABASE_URL=postgres://...
REDIS_URL=redis://...
```

### 2. Docker

```dockerfile
# Dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/my-app /usr/local/bin/
CMD ["my-app"]
```

### 3. Health Checks

```rust
// Add health check endpoint
Router::new()
    .route("/health", Route::get(health_check));

async fn health_check(req: Request) -> Response {
    // Check database
    if db.ping().await.is_err() {
        return Response::service_unavailable();
    }

    // Check Redis
    if redis.ping().await.is_err() {
        return Response::service_unavailable();
    }

    Response::ok()
}
```

### 4. Monitoring

```rust
use rf_telescope::Telescope;
use rf_horizon::Horizon;

// Enable monitoring in production
Telescope::enable();
Horizon::enable();
```

---

## Summary

Following these best practices will help you build:

- ✅ **Secure** applications
- ✅ **Performant** applications
- ✅ **Maintainable** code
- ✅ **Testable** architecture
- ✅ **Production-ready** systems

**Remember:** These are guidelines, not absolute rules. Adapt them to your project's specific needs.

---

**Questions?** Join our [Discord](https://discord.gg/rustforge) or check the [documentation](https://docs.rustforge.dev).
