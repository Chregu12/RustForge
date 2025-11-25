# Migration Guide

This guide helps you migrate from other Rust frameworks to RustForge, or get started with RustForge's elegant syntax.

## Table of Contents

- [RustForge Syntax Guide](#rustforge-syntax-guide)
- [From Laravel (PHP)](#from-laravel-php)
- [From Actix-web](#from-actix-web)
- [From Rocket](#from-rocket)
- [From Axum](#from-axum)
- [Breaking Changes](#breaking-changes)

---

## RustForge Syntax Guide

RustForge provides an elegant, expressive syntax for building web applications in Rust.

### Models

Define models with the `#[model]` macro:

```rust
use rf_prelude::*;  // Single import - everything included!

#[model]
pub struct User {
    pub name: String,
    pub email: String,
    #[hidden]
    pub password: String,
}

// Relations
#[relations]
impl User {
    fn posts() -> HasMany<Post> {
        self.has_many()
    }
}
```

The `#[model]` macro automatically:
- Adds `id`, `created_at`, `updated_at` fields
- Adds all necessary derives
- Converts `#[hidden]` to skip serialization
- Implements Model trait for static methods

### Querying Data

Use the elegant query builder:

```rust
#[auto_await]
async fn examples() -> Result<()> {
    // Find by ID
    let user = User::find(1);

    // Filter records
    let admins = User::filter("role", "admin")
        .filter("active", true)
        .order_by("name", "asc")
        .limit(10)
        .get();

    // Get all
    let all_users = User::all();

    // First matching
    let user = User::filter("email", "john@example.com").first();

    // Count
    let total = User::count();

    // Check existence
    let exists = User::filter("email", "test@example.com").exists();

    Ok(())
}
```

### Creating & Updating

```rust
#[auto_await]
async fn crud_examples() -> Result<()> {
    // Create
    let user = User::create(json!({
        "name": "John",
        "email": "john@example.com"
    }));

    // Update by ID
    User::update_by_id(1, json!({
        "name": "John Doe"
    }));

    // Delete
    User::destroy(1);

    // First or create
    let user = User::first_or_create(
        json!({"email": "john@example.com"}),
        json!({"name": "John", "email": "john@example.com"})
    );

    // Update or create
    let user = User::update_or_create(
        json!({"email": "john@example.com"}),
        json!({"name": "John Updated"})
    );

    Ok(())
}
```

### Auto-Await Macro

Use `#[auto_await]` to write cleaner code without explicit `.await`:

```rust
// Without auto_await (verbose)
async fn verbose() -> Result<()> {
    let users = User::filter("active", true).get().await?;
    let cached = Cache::get("stats").await?;
    Ok(())
}

// With auto_await (clean)
#[auto_await]
async fn clean() -> Result<()> {
    let users = User::filter("active", true).get();
    let cached = Cache::get("stats");
    Ok(())
}
```

### Routing

```rust
Route::get("/users", user_controller::index);
Route::post("/users", user_controller::store);
Route::put("/users/{id}", user_controller::update);
Route::delete("/users/{id}", user_controller::destroy);

// Grouped routes with middleware
Route::middleware(&["auth"]).group(|| {
    Route::get("/profile", profile_controller::show);
    Route::put("/profile", profile_controller::update);
});

// Prefixed routes
Route::prefix("/api/v1").group(|| {
    Route::get("/users", api::users::index);
});
```

### Controllers

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3))]
    pub name: String,

    #[validate(email)]
    pub email: String,
}

#[auto_await]
pub async fn index() -> Result<Response> {
    let users = User::filter("active", true).get();
    Ok(Response::json(users))
}

#[auto_await]
pub async fn store(Json(payload): Json<CreateUserRequest>) -> Result<Response> {
    payload.validate()?;

    let user = User::create(json!({
        "name": payload.name,
        "email": payload.email
    }));

    Ok(Response::json(user).status(201))
}
```

### Authentication

```rust
#[auto_await]
async fn auth_examples() -> Result<()> {
    // Login attempt
    Auth::attempt(json!({
        "email": "user@example.com",
        "password": "secret"
    }));

    // Get current user
    let user = Auth::user::<User>();

    // Check authentication
    if Auth::check() {
        println!("User is logged in");
    }

    // Check if guest
    if Auth::guest() {
        println!("User is not logged in");
    }

    // Get user ID
    let id = Auth::id();

    // Login a user directly
    Auth::login(user);

    // Logout
    Auth::logout();

    Ok(())
}
```

### Caching

```rust
#[auto_await]
async fn cache_examples() -> Result<()> {
    // Store value (TTL in seconds)
    Cache::put("key", "value", 3600);

    // Get value
    let value: Option<String> = Cache::get("key");

    // Remember pattern
    let users = Cache::remember("users", 3600, || async {
        Ok(User::all().await?)
    });

    // Check existence
    if Cache::has("key") {
        println!("Key exists");
    }

    // Delete key
    Cache::forget("key");

    // Store forever
    Cache::forever("permanent_key", "value");

    // Add only if not exists
    Cache::add("new_key", "value", 60);

    // Clear all cache
    Cache::flush();

    Ok(())
}
```

### Validation

```rust
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email, unique(table = "users", column = "email"))]
    pub email: String,

    #[validate(length(min = 3, max = 50))]
    pub name: String,

    #[validate(range(min = 18))]
    pub age: u8,

    #[validate(length(min = 8))]
    pub password: String,
}

// Use in handler
pub async fn store(Json(payload): Json<CreateUserRequest>) -> Result<Response> {
    payload.validate()?;  // Returns error if validation fails
    // ... create user
}
```

### Jobs & Queues

```rust

#[derive(Debug, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub user_id: i32,
    pub template: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: &JobContext) -> Result<(), Error> {
        let user = User::find(self.user_id).await?;

        Mail::to(&user.email)
            .template(&self.template)
            .send()
            .await?;

        Ok(())
    }
}

// Dispatch job
Queue::push(SendEmailJob {
    user_id: 1,
    template: "welcome".to_string()
}).await?;
```

### CLI Commands

| Command | Description |
|---------|-------------|
| `forge make:model User` | Create model |
| `forge make:controller UserController` | Create controller |
| `forge make:migration create_users` | Create migration |
| `forge migrate` | Run migrations |
| `forge migrate:rollback` | Rollback migration |
| `forge db:seed` | Seed database |
| `forge queue:work` | Start queue worker |
| `forge cache:clear` | Clear cache |
| `forge route:list` | List routes |

---

## From Laravel (PHP)

RustForge provides a familiar API for Laravel developers, with the performance benefits of Rust.

### Key Differences

| Laravel (PHP) | RustForge (Rust) | Notes |
|---------------|------------------|-------|
| Dynamically typed | Statically typed | Type safety at compile time |
| Runtime errors | Compile-time errors | Catch bugs before deployment |
| `.env` config | `.env` config | Same approach |
| Eloquent ORM | Model trait | Similar API, better performance |
| Artisan CLI | Forge CLI | Similar commands |

### Routing

**Laravel:**
```php
Route::get('/users', [UserController::class, 'index']);
Route::post('/users', [UserController::class, 'store']);
Route::middleware('auth')->group(function () {
    Route::get('/profile', [ProfileController::class, 'show']);
});
```

**RustForge:**
```rust
use rf_route_facade::Route;

Route::get("/users", user_controller::index);
Route::post("/users", user_controller::store);
Route::middleware(&["auth"]).group(|| {
    Route::get("/profile", profile_controller::show);
});
```

### Models & Queries

**Laravel:**
```php
// Define model
class User extends Model {
    protected $fillable = ['name', 'email'];
    protected $hidden = ['password'];
}

// Query
$users = User::where('active', true)->get();
$user = User::find(1);
$admins = User::where('role', 'admin')
    ->where('active', true)
    ->orderBy('name')
    ->limit(10)
    ->get();
```

**RustForge:**
```rust
use rf_prelude::*;  // Single import - everything included!

// Define model
#[model]
pub struct User {
    pub name: String,
    pub email: String,
    #[hidden]
    pub password: String,
}

// Query (with #[auto_await] - no .await needed!)
#[auto_await]
async fn queries() -> Result<()> {
    let users = User::filter("active", true).get();
    let user = User::find(1);
    let admins = User::filter("role", "admin")
        .filter("active", true)
        .order_by("name", "asc")
        .limit(10)
        .get();
    Ok(())
}
```

### CRUD Operations

**Laravel:**
```php
// Create
$user = User::create([
    'name' => 'John',
    'email' => 'john@example.com'
]);

// Update
User::where('id', 1)->update(['name' => 'John Doe']);

// Delete
User::destroy(1);

// First or create
$user = User::firstOrCreate(
    ['email' => 'john@example.com'],
    ['name' => 'John']
);
```

**RustForge:**
```rust
#[auto_await]
async fn crud() -> Result<()> {
    // Create
    let user = User::create(json!({
        "name": "John",
        "email": "john@example.com"
    }));

    // Update
    User::update_by_id(1, json!({"name": "John Doe"}));

    // Delete
    User::destroy(1);

    // First or create
    let user = User::first_or_create(
        json!({"email": "john@example.com"}),
        json!({"name": "John"})
    );
    Ok(())
}
```

### Caching

**Laravel:**
```php
Cache::put('key', 'value', 3600);
$value = Cache::get('key');
Cache::forget('key');

$users = Cache::remember('users', 3600, function () {
    return User::all();
});
```

**RustForge:**
```rust
#[auto_await]
async fn caching() -> Result<()> {
    Cache::put("key", "value", 3600);
    let value: Option<String> = Cache::get("key");
    Cache::forget("key");

    let users = Cache::remember("users", 3600, || async {
        Ok(User::all().await?)
    });
    Ok(())
}
```

### Authentication

**Laravel:**
```php
Auth::attempt(['email' => $email, 'password' => $password]);
$user = Auth::user();
Auth::logout();

if (Auth::check()) {
    // User is logged in
}
```

**RustForge:**
```rust
#[auto_await]
async fn auth() -> Result<()> {
    Auth::attempt(json!({
        "email": email,
        "password": password
    }));
    let user = Auth::user::<User>();
    Auth::logout();

    if Auth::check() {
        // User is logged in
    }
    Ok(())
}
```

### Key Benefits

- **100x faster**: Rust's performance vs PHP
- **Type safety**: Catch errors at compile time
- **Memory safety**: No null pointer exceptions
- **Same familiar API**: Easy transition from Laravel
- **`#[auto_await]`**: Write code almost like PHP (no manual `.await`)

---

## From Actix-web

Migrating from Actix-web to RustForge provides a higher-level API while maintaining performance.

### Routing

**Actix-web:**
```rust
HttpServer::new(|| {
    App::new()
        .route("/", web::get().to(index))
        .route("/users", web::post().to(create_user))
        .service(
            web::scope("/api")
                .wrap(AuthMiddleware)
                .route("/profile", web::get().to(get_profile))
        )
})
.bind("127.0.0.1:8080")?
.run()
.await
```

**RustForge:**
```rust
use rf_route_facade::Route;

Route::get("/", index);
Route::post("/users", create_user);

Route::prefix("/api").middleware(&["auth"]).group(|| {
    Route::get("/profile", get_profile);
});

app.serve(Route::router()).await?;
```

### Handlers

**Actix-web:**
```rust
async fn create_user(
    Json(payload): Json<CreateUserRequest>,
    db: Data<Database>,
) -> Result<HttpResponse, Error> {
    let user = User::create(payload).await?;
    Ok(HttpResponse::Created().json(user))
}
```

**RustForge:**
```rust
pub async fn create_user(
    Json(payload): Json<CreateUserRequest>,
    db: Database,
) -> Result<Response, Error> {
    let user = User::create(payload).await?;
    Ok(Response::json(user).status(201))
}
```

### Middleware

**Actix-web:**
```rust
impl<S, B> Transform<S, ServiceRequest> for AuthMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    // Complex implementation
}
```

**RustForge:**
```rust
pub struct AuthMiddleware;

#[async_trait]
impl Middleware for AuthMiddleware {
    async fn handle(&self, req: Request, next: Next) -> Result<Response> {
        let token = req.headers().get("Authorization");
        // Verify token
        next.run(req).await
    }
}
```

### Key Benefits

- **Higher-level API**: Less boilerplate
- **Better ORM**: Integrated ORM instead of manual SQL
- **Built-in features**: Auth, caching, queues out of the box
- **Same performance**: Still built on Tokio

---

## From Rocket

RustForge provides async support and a more flexible architecture compared to Rocket.

### Routing

**Rocket:**
```rust
#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[post("/users", data = "<user>")]
fn create_user(user: Json<CreateUserRequest>) -> Json<User> {
    // ...
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount("/", routes![index, create_user])
}
```

**RustForge:**
```rust
use rf_route_facade::Route;

async fn index() -> &'static str {
    "Hello, world!"
}

async fn create_user(Json(user): Json<CreateUserRequest>) -> Result<Response> {
    // ...
}

#[tokio::main]
async fn main() {
    Route::get("/", index);
    Route::post("/users", create_user);

    app.serve(Route::router()).await?;
}
```

### Key Benefits

- **Async by default**: Better performance for I/O-heavy apps
- **No route macros needed**: Cleaner syntax
- **Better ecosystem**: More middleware and plugins
- **ORM included**: No need for Diesel or SeaORM separately

---

## From Axum

Axum users will find RustForge familiar but with more batteries included.

### Routing

**Axum:**
```rust
let app = Router::new()
    .route("/", get(index))
    .route("/users", post(create_user))
    .layer(Extension(db))
    .layer(middleware::from_fn(auth_middleware));
```

**RustForge:**
```rust
use rf_route_facade::Route;

Route::get("/", index);
Route::post("/users", create_user);
Route::use_middleware(middleware::auth());
// Database injected automatically
```

### Key Benefits

- **ORM built-in**: No need to manually integrate SeaORM
- **Auth system**: JWT, sessions out of the box
- **Validation**: Built-in validation framework
- **CLI tools**: Code generation and migration tools
- **More features**: Caching, queues, mail, storage included

---

## Breaking Changes

### From RustForge 0.x to 1.0

#### Package Names

All packages have been renamed from `foundry-*` to `rf-*`:

```rust
// Old (0.x)
use foundry_orm::prelude::*;
use foundry_http::{Router, Request};
use foundry_auth::JwtAuth;

// New (1.0)
use rf_orm::prelude::*;
use rf_http::{Router, Request};
use rf_auth::JwtAuth;
```

#### Storage API

```rust
// Old (0.x)
let disk = storage_manager.disk(Some("s3"))?;
disk.put("file.txt", data).await?;

// New (1.0)
Storage::disk("s3")
    .put("file.txt", data)
    .await?;
```

#### Queue API

```rust
// Old (0.x)
use foundry_queue::Queue;

// New (1.0)
use rf_jobs::Job;
use rf_queue::Queue;
```

### Migration Steps for 1.0

1. **Update Dependencies**:
   ```toml
   # In Cargo.toml
   rf-core = "1.0.0"  # was foundry-core
   rf-orm = "1.0.0"   # was foundry-orm
   ```

2. **Update Imports**:
   ```bash
   find . -name "*.rs" -exec sed -i 's/foundry_/rf_/g' {} +
   ```

3. **Update Storage Calls**:
   Replace `disk(Some("name"))` with `disk("name")`

4. **Test Everything**:
   ```bash
   cargo test
   cargo build
   ```

---

## Next Steps

- **[Installation Guide](Installation)** - Install RustForge
- **[Quick Start](Quick-Start)** - Build your first app
- **[Features](Features)** - Explore all features
- **[Examples](Examples)** - See code examples

---

*Need help migrating? Open an issue on [GitHub](https://github.com/Chregu12/RustForge/issues) or join our community.*
