# Migration Guide

This guide helps you migrate from other Rust frameworks to RustForge, or get started with RustForge's elegant syntax.

## Table of Contents

- [RustForge Syntax Guide](#rustforge-syntax-guide)
  - [Models](#models)
  - [Relationships](#relationships)
  - [Querying Data](#querying-data)
  - [Creating & Updating](#creating--updating)
  - [Helper Macros](#helper-macros)
  - [Routing](#routing)
  - [Authentication](#authentication)
  - [Caching](#caching)
  - [Validation](#validation)
- [From Laravel (PHP)](#from-laravel-php)
- [From Actix-web](#from-actix-web)
- [From Rocket](#from-rocket)
- [From Axum](#from-axum)
- [Breaking Changes](#breaking-changes)

---

## RustForge Syntax Guide

RustForge provides an elegant, expressive syntax for building web applications in Rust.

### Models

RustForge bietet 3 Syntax-Optionen - von minimal bis Laravel-ähnlich:

```rust
use rustforge::*;
```

**Option 1: Ultra-Minimal (eine Zeile!)**
```rust
Model!(User: name, email, hidden password);
```

**Option 2: Mit expliziten Typen**
```rust
Model!(User {
    name: String,
    email: String,
    hidden password: String,
    age: i32,
});
```

**Option 3: Laravel-Syntax**
```rust
laravel! {
    class User extends Model {
        protected fillable = [name: String, email: String];
        protected hidden = [password: String];
    }
}
```

Alle drei Optionen generieren automatisch:
- `id`, `created_at`, `updated_at` Felder
- `Model` Trait Implementation
- `FILLABLE` und `HIDDEN` Konstanten
- `Default` Implementation

### Relationships

Define Laravel-style relationships directly in your models:

```rust
Model!(User {
    name: String,
    email: String,
    hidden password: String,

    // One-to-Many: User has many Posts
    hasMany posts: Post,

    // One-to-One: User has one Profile
    hasOne profile: Profile,

    // Enable soft deletes
    softDeletes,
});

Model!(Post {
    title: String,
    body: String,
    user_id: i64,

    // Many-to-One: Post belongs to User
    belongsTo user: User,

    // Many-to-Many: Post has many Tags
    belongsToMany tags: Tag,

    // Disable timestamps
    timestamps = false,
});

Model!(Profile {
    bio: String,
    avatar: String,
    user_id: i64,

    belongsTo user: User,
});
```

**Using Relationships:**
```rust
#[auto_await]
async fn relationship_examples() -> Result<()> {
    let user = User::find(1);

    // Get all posts for a user (hasMany)
    let posts = user.posts().get();

    // Get user's profile (hasOne)
    let profile = user.profile();

    // Get post's author (belongsTo)
    let post = Post::find(1);
    let author = post.user();

    Ok(())
}
```

**Soft Deletes:**
```rust
#[auto_await]
async fn soft_delete_examples() -> Result<()> {
    let mut user = User::find(1);

    // Soft delete (sets deleted_at)
    user.soft_delete();

    // Check if trashed
    if user.trashed() {
        println!("User is soft-deleted");
    }

    // Restore soft-deleted record
    user.restore();

    // Query including soft-deleted
    let all_users = User::with_trashed().get();

    // Query only soft-deleted
    let trashed = User::only_trashed().get();

    // Permanently delete
    user.force_delete();

    Ok(())
}
```

### Querying Data

Use the elegant query builder - **exactly like Laravel!**

```rust
#[auto_await]
async fn examples() -> Result<()> {
    // Find by ID
    let user = User::find(1);
    let user = User::findOrFail(1);  // Error if not found

    // WHERE - exactly like Laravel!
    let admins = User::where("role", "admin")
        .where("active", true)
        .orderBy("name", "asc")
        .take(10)
        .get();

    // OR conditions
    let staff = User::where("role", "admin")
        .orWhere("role", "moderator")
        .get();

    // Advanced WHERE
    let users = User::whereIn("id", vec![1, 2, 3])
        .whereNotNull("email_verified_at")
        .whereBetween("age", 18, 65)
        .get();

    // Date queries
    let recent = User::whereYear("created_at", 2024)
        .whereMonth("created_at", 11)
        .get();

    // Get all
    let all_users = User::all();

    // First matching
    let user = User::where("email", "john@example.com").first();
    let user = User::where("email", "john@example.com").firstOrFail();

    // Aggregates
    let total = User::count();
    let emails = User::where("active", true).pluck("email");
    let email = User::find(1).value("email");

    // Check existence
    let exists = User::where("email", "test@example.com").exists();

    // Conditional queries
    let users = User::query()
        .when(is_admin, |q| q.where("role", "admin"))
        .latest()
        .get();

    // Process large datasets in chunks
    User::query().chunk(100, |users| {
        for user in users {
            // Process each batch
        }
        true // Continue processing
    });

    // Lazy iteration for memory efficiency
    let mut lazy = User::query().lazy(100);
    while let Some(user) = lazy.next() {
        // Process one at a time
    }

    // Get exactly one record (error if 0 or >1)
    let user = User::where("email", "unique@example.com").sole();

    // NOT conditions
    let users = User::whereNotBetween("age", 13, 17)
        .whereNotLike("email", "%@spam.com")
        .get();

    // Increment/Decrement
    User::where("id", 1).increment("login_count", 1);
    User::where("id", 1).decrement("credits", 10);

    // Debugging
    User::where("active", true).dump();  // Print query info
    let sql = User::where("role", "admin").toSql();  // Get SQL string
    // User::where("active", true).dd();  // Dump and die

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

    // Update by ID (Laravel camelCase!)
    User::updateById(1, json!({
        "name": "John Doe"
    }));

    // Delete
    User::destroy(1);

    // First or create (Laravel camelCase!)
    let user = User::firstOrCreate(
        json!({"email": "john@example.com"}),
        json!({"name": "John", "email": "john@example.com"})
    );

    // Update or create (Laravel camelCase!)
    let user = User::updateOrCreate(
        json!({"email": "john@example.com"}),
        json!({"name": "John Updated"})
    );

    // First or new (not saved)
    let user = User::firstOrNew(
        json!({"email": "john@example.com"}),
        json!({"name": "John"})
    );

    // Upsert (bulk insert/update)
    User::upsert(
        vec![
            json!({"email": "john@ex.com", "name": "John"}),
            json!({"email": "jane@ex.com", "name": "Jane"}),
        ],
        &["email"],  // Unique columns
        &["name"]    // Columns to update on conflict
    );

    // Touch timestamps
    User::where("id", 1).touch();

    // Delete by IDs
    User::destroy(vec![1, 2, 3]);

    // Truncate table
    User::truncate();  // Careful! Deletes all!

    Ok(())
}
```

### The `#[auto_await]` Macro

The `#[auto_await]` macro does **TWO things** automatically:
1. **Transforms `where` to `r#where`** - use `where()` like Laravel!
2. **Adds `.await` automatically** - no explicit `.await` needed!

**Recommended file structure - `#[auto_await]` once at top:**

```rust
// main.rs or lib.rs
use rustforge::*;

Model!(User: name, email, hidden password);
Model!(Post: title, body, user_id);

#[auto_await]  // <- Once here, applies to EVERYTHING below!
mod app {
    use super::*;

    // Routes
    pub fn routes() {
        Route::get("/users", index);
        Route::get("/users/:id", show);
        Route::post("/users", store);
        Route::delete("/users/:id", destroy);
    }

    // Handlers - no .await, no query! needed!
    pub async fn index() -> Response {
        let users = User::where("active", true)
            .orderBy("name", "asc")
            .get();
        Response::json(users)
    }

    pub async fn show(id: i64) -> Response {
        let user = User::findOrFail(id);
        Response::json(user)
    }

    pub async fn store(data: Json<Value>) -> Response {
        let user = User::create(data.0);
        Response::json(user)
    }

    pub async fn destroy(id: i64) -> Response {
        User::destroy(id);
        Response::ok()
    }
}

pub use app::*;  // Re-export everything
```

### Helper Macros

RustForge provides Laravel-style helper macros for common tasks:

#### Collections
```rust
use rustforge::*;

// Create a collection
let numbers = collect![1, 2, 3, 4, 5];
let users = collect![user1, user2, user3];
```

#### Configuration & Environment
```rust
// Get config value
let db_host = config!("database.host");
let timeout = config!("cache.timeout", 3600);  // with default

// Get environment variable
let app_env = env_var!("APP_ENV");
let debug = env_var!("APP_DEBUG", "false");  // with default
```

#### Routes & URLs
```rust
// Generate named route URL
let url = route!("users.show", id = 123);
let home = route!("home");

// Generate asset URL
let css = asset!("css/app.css");  // -> /assets/css/app.css
let js = asset!("js/app.js");

// Generate full URL
let full = url!("/users/123");  // -> https://myapp.com/users/123
```

#### Responses
```rust
// JSON response
return response!(json: users);

// Text response
return response!(text: "Hello World");

// Redirect
return response!(redirect: "/home");

// View with data
return response!(view: "users.index", users_data);

// Status code only
return response!(status: 204);

// File download
return response!(download: "/path/to/file.pdf");

// Serve file
return response!(file: "/path/to/image.png");
```

#### Error Handling
```rust
// Abort with status code
abort!(404);
abort!(403, "Forbidden");
abort!(500, "Internal Server Error");
```

#### Debugging
```rust
// Dump and die - prints and exits
dd!(user, request, config);

// Dump without stopping
dump!(user, config);

// Output:
// === DD (Dump & Die) ===
// [0] user = User { id: 1, name: "John" ... }
// [1] request = Request { ... }
// =======================
```

#### Form Helpers
```rust
// Get old form input (for repopulating after validation errors)
let email = old!("email");
let name = old!("name", "Default Name");  // with default
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
class User extends Model {
    protected $fillable = ['name', 'email'];
    protected $hidden = ['password'];
}

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
use rustforge::*;

Model!(User: name, email, hidden password);

// Mit query! macro - `where` wie in Laravel!
let users = query!(User::where("active", true).get()).await;
let user = User::find(1).await;
let admins = query! {
    User::where("role", "admin")
        .where("active", true)
        .orderBy("name", "asc")
        .limit(10)
        .get()
}.await;
```

**Mit `#[auto_await]` - kein `.await` nötig:**
```rust
use rustforge::*;

Model!(User: name, email, hidden password);

#[auto_await]
async fn queries() -> Result<()> {
    let users = query!(User::where("active", true).get());
    let user = User::find(1);
    let admins = query! {
        User::where("role", "admin")
            .where("active", true)
            .orderBy("name", "asc")
            .limit(10)
            .get()
    };
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
    User::updateById(1, json!({"name": "John Doe"}));

    // Delete
    User::destroy(1);

    // First or create
    let user = User::firstOrCreate(
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

## What's New in Latest Version

### Model Relationships

Models now support Laravel-style relationships directly in the `Model!` macro:

```rust
Model!(User {
    name: String,
    email: String,

    hasMany posts: Post,
    hasOne profile: Profile,
    belongsTo company: Company,
    belongsToMany roles: Role,

    softDeletes,
    timestamps = true,
});
```

**Relationship Types:**
| Type | Description | Example |
|------|-------------|---------|
| `hasMany` | One-to-Many | User has many Posts |
| `hasOne` | One-to-One | User has one Profile |
| `belongsTo` | Many-to-One / Inverse | Post belongs to User |
| `belongsToMany` | Many-to-Many | Post has many Tags |

**Additional Options:**
- `softDeletes` - Enable soft delete functionality
- `timestamps = false` - Disable auto timestamps

### New Query Builder Methods

| Method | Description |
|--------|-------------|
| `whereNotBetween(col, min, max)` | NOT BETWEEN condition |
| `whereNotLike(col, pattern)` | NOT LIKE condition |
| `whereRaw(sql, bindings)` | Raw SQL WHERE |
| `increment(col, amount)` | Increment column value |
| `decrement(col, amount)` | Decrement column value |
| `sole()` | Get exactly one or error |
| `chunk(size, callback)` | Process in batches |
| `each(callback)` | Iterate all records |
| `lazy(size)` | Memory-efficient iteration |
| `toSql()` | Get SQL query string |
| `dump()` | Debug print query |
| `dd()` | Dump and die |

### New Eloquent Methods

| Method | Description |
|--------|-------------|
| `firstOrCreate(search, create)` | Find or create |
| `firstOrNew(search, create)` | Find or new instance |
| `updateOrCreate(search, update)` | Update or create |
| `updateOrInsert(search, update)` | Update or insert |
| `upsert(records, unique, update)` | Bulk upsert |
| `touch()` | Update timestamps |
| `destroy(ids)` | Delete by IDs |
| `truncate()` | Delete all |

### New Helper Macros

| Macro | Description | Example |
|-------|-------------|---------|
| `collect!` | Create collection | `collect![1, 2, 3]` |
| `config!` | Get config value | `config!("app.name")` |
| `env_var!` | Get env variable | `env_var!("APP_ENV", "prod")` |
| `route!` | Generate route URL | `route!("users.show", id = 1)` |
| `response!` | Create response | `response!(json: data)` |
| `abort!` | HTTP error | `abort!(404, "Not found")` |
| `dd!` | Dump and die | `dd!(user, config)` |
| `dump!` | Dump without exit | `dump!(request)` |
| `old!` | Old form input | `old!("email")` |
| `asset!` | Asset URL | `asset!("css/app.css")` |
| `url!` | Full URL | `url!("/users")` |

---

## Next Steps

- **[Installation Guide](Installation)** - Install RustForge
- **[Quick Start](Quick-Start)** - Build your first app
- **[Features](Features)** - Explore all features
- **[Examples](Examples)** - See code examples

---

*Need help migrating? Open an issue on [GitHub](https://github.com/Chregu12/RustForge/issues) or join our community.*
