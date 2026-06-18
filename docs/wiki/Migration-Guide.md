# Migration Guide

This guide helps you migrate from other Rust frameworks to RustForge, or get started with RustForge's elegant syntax.

## Table of Contents

- [RustForge Syntax Guide](#rustforge-syntax-guide)
  - [The Ultimate Experience: rustforge! Block](#the-ultimate-experience-rustforge-block)
  - [Models](#models-alternative-manual-imports)
  - [Relationships](#relationships)
  - [Querying Data](#querying-data)
  - [Creating & Updating](#creating--updating)
  - [The #[auto_await] Macro](#the-auto_await-macro)
  - [Helper Macros](#helper-macros)
  - [Routing](#routing)
  - [Controllers](#controllers)
  - [Authentication](#authentication)
  - [Caching](#caching)
  - [Validation](#validation)
- [What's New](#whats-new-in-latest-version)
  - [New Helper Macros](#new-helper-macros)
  - [FormRequest Validation](#formrequest---laravel-style-validation)
  - [Exception Handler](#exception-handler---laravel-style-error-handling)
  - [Blade Templates](#blade-templates---laravel-style-templating)
  - [Mailable & Notifications](#mailable--notifications---laravel-style-emails)
- [From Laravel (PHP)](#from-laravel-php)
- [From Actix-web](#from-actix-web)
- [From Rocket](#from-rocket)
- [From Axum](#from-axum)
- [Breaking Changes](#breaking-changes)

---

## RustForge Syntax Guide

RustForge provides an elegant, expressive syntax for building web applications in Rust.

### The Ultimate Experience: `rustforge!` Block

**Write Rust exactly like Laravel PHP** - no imports, no `#[auto_await]`, no `.await`!

```rust
// That's it! No imports needed!
rustforge! {
    Model!(User: name, email, hidden password);
    Model!(Post: title, body, user_id);

    // Define routes
    fn routes() {
        Route::get("/", index);
        Route::get("/users", users_index);
        Route::post("/users", users_store);
        Route::get("/users/:id", users_show);
    }

    // Handlers - NO .await needed!
    async fn index() -> Response {
        Response::text("Welcome to RustForge!")
    }

    async fn users_index() -> Response {
        // Exactly like Laravel!
        let users = User::where("active", true)
            .orderBy("name", "asc")
            .take(10)
            .get();  // No .await!

        Response::json(users)
    }

    async fn users_show(id: i64) -> Response {
        let user = User::findOrFail(id);  // No .await!
        Response::json(user)
    }

    async fn users_store(data: Json<Value>) -> Response {
        let user = User::create(data.0);  // No .await!
        Response::json(user).status(201)
    }

    // Use #[sync] to opt-out for synchronous helpers
    #[sync]
    fn format_name(name: &str) -> String {
        name.to_uppercase()
    }
}
```

**What `rustforge!` does automatically:**
- ✅ Adds `use rustforge::*;` - no manual imports
- ✅ Applies `#[auto_await]` to all async functions
- ✅ Transforms `where` to `r#where` automatically
- ✅ Adds `.await` to all async calls automatically

**Use `#[sync]` to opt-out** for functions that shouldn't have auto_await applied.

---

### Models (Alternative: Manual Imports)

If you prefer explicit imports, RustForge bietet 3 Syntax-Optionen - von minimal bis Laravel-ähnlich:

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
async fn show_user(id: i64) -> Response {
    let user = User::find(id);

    // Get all posts for a user (hasMany)
    let posts = user.posts().get();

    // Get user's profile (hasOne)
    let profile = user.profile();

    Response::json(json!({ "user": user, "posts": posts, "profile": profile }))
}

async fn show_post(id: i64) -> Response {
    let post = Post::find(id);
    let author = post.user();  // belongsTo
    Response::json(json!({ "post": post, "author": author }))
}
```

**Soft Deletes:**
```rust
#[auto_await]
async fn delete_user(id: i64) -> Response {
    let mut user = User::find(id);
    user.soft_delete();  // Sets deleted_at
    Response::json(json!({ "message": "User deleted" }))
}

async fn restore_user(id: i64) -> Response {
    let mut user = User::with_trashed().find(id);
    user.restore();  // Clears deleted_at
    Response::json(user)
}

async fn get_trashed() -> Response {
    let trashed = User::only_trashed().get();
    Response::json(trashed)
}

async fn force_delete(id: i64) -> Response {
    let user = User::with_trashed().find(id);
    user.force_delete();  // Permanently removes
    Response::ok()
}
```

### Querying Data

Use the elegant query builder - **exactly like Laravel!**

```rust
#[auto_await]
async fn index() -> Response {
    // WHERE - exactly like Laravel!
    let admins = User::where("role", "admin")
        .where("active", true)
        .orderBy("name", "asc")
        .take(10)
        .get();
    Response::json(admins)
}

async fn show(id: i64) -> Response {
    let user = User::findOrFail(id);  // 404 if not found
    Response::json(user)
}

async fn search(role: &str) -> Response {
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

    Response::json(users)
}

async fn stats() -> Response {
    let total = User::count();
    let emails = User::where("active", true).pluck("email");
    let exists = User::where("email", "test@example.com").exists();

    Response::json(json!({ "total": total, "emails": emails }))
}

// Conditional queries
async fn list_users(is_admin: bool) -> Response {
    let users = User::query()
        .when(is_admin, |q| q.where("role", "admin"))
        .latest()
        .get();
    Response::json(users)
}

// Increment/Decrement
async fn login(id: i64) -> Response {
    User::where("id", id).increment("login_count", 1);
    Response::ok()
}
```

### Creating & Updating

```rust
#[auto_await]
async fn store(data: Json<Value>) -> Response {
    let user = User::create(json!({
        "name": data["name"],
        "email": data["email"]
    }));
    Response::json(user).status(201)
}

async fn update(id: i64, data: Json<Value>) -> Response {
    User::updateById(id, data.0);
    let user = User::find(id);
    Response::json(user)
}

async fn destroy(id: i64) -> Response {
    User::destroy(id);
    Response::ok()
}

// First or create - finds or creates
async fn find_or_create(email: &str) -> Response {
    let user = User::firstOrCreate(
        json!({"email": email}),
        json!({"name": "New User", "email": email})
    );
    Response::json(user)
}

// Update or create - upsert single record
async fn upsert_user(email: &str, name: &str) -> Response {
    let user = User::updateOrCreate(
        json!({"email": email}),
        json!({"name": name})
    );
    Response::json(user)
}

// Bulk upsert
async fn bulk_import(users: Vec<Value>) -> Response {
    User::upsert(
        users,
        &["email"],  // Unique columns
        &["name"]    // Columns to update on conflict
    );
    Response::json(json!({ "message": "Import complete" }))
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
async fn login(data: Json<LoginRequest>) -> Response {
    if Auth::attempt(json!({
        "email": data.email,
        "password": data.password
    })) {
        Response::json(json!({ "message": "Login successful" }))
    } else {
        Response::json(json!({ "error": "Invalid credentials" })).status(401)
    }
}

async fn profile() -> Response {
    if let Some(user) = Auth::user::<User>() {
        Response::json(user)
    } else {
        Response::json(json!({ "error": "Not authenticated" })).status(401)
    }
}

async fn logout() -> Response {
    Auth::logout();
    Response::json(json!({ "message": "Logged out" }))
}
```

### Caching

```rust
#[auto_await]
async fn get_users() -> Response {
    // Remember pattern - cache for 1 hour
    let users = Cache::remember("users:all", 3600, || async {
        User::all()
    });
    Response::json(users)
}

async fn clear_cache() -> Response {
    Cache::forget("users:all");
    Response::json(json!({ "message": "Cache cleared" }))
}

async fn cache_stats() -> Response {
    let has_users = Cache::has("users:all");
    Response::json(json!({ "cached": has_users }))
}
```

> New in recent versions: `Cache::touch(key, ttl)` extends an existing entry's
> TTL without rewriting its value (`rf_cache::CacheFacade::touch`, returns
> `CacheResult<bool>`).

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

> New: `JobRouter` (`rf_jobs::JobRouter`) routes job classes to specific
> queues/connections — `JobRouter::route::<SendEmailJob>("emails")`. Per-job
> queue selection is also available via `Job::queue()`.

> New crates: `rf-ai` (LLM SDK with an Anthropic provider, agents, embeddings),
> `rf-vector` (vector search with pgvector helpers), and the `jsonapi` module in
> `rf-api-resources` (JSON:API documents: `JsonApiDocument`, `ResourceObject`,
> `Relationship`).

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

### Side-by-Side Comparison

**Laravel (PHP):**
```php
<?php
use App\Models\User;

class UserController extends Controller
{
    public function index()
    {
        $users = User::where('active', true)
            ->orderBy('name', 'asc')
            ->take(10)
            ->get();

        return response()->json($users);
    }

    public function show($id)
    {
        $user = User::findOrFail($id);
        return response()->json($user);
    }

    public function store(Request $request)
    {
        $user = User::create($request->all());
        return response()->json($user, 201);
    }
}
```

**RustForge (Rust) - Almost identical!**
```rust
rustforge! {
    Model!(User: name, email, hidden password);

    async fn index() -> Response {
        let users = User::where("active", true)
            .orderBy("name", "asc")
            .take(10)
            .get();

        Response::json(users)
    }

    async fn show(id: i64) -> Response {
        let user = User::findOrFail(id);
        Response::json(user)
    }

    async fn store(data: Json<Value>) -> Response {
        let user = User::create(data.0);
        Response::json(user).status(201)
    }
}
```

### Key Differences

| Laravel (PHP) | RustForge (Rust) | Notes |
|---------------|------------------|-------|
| Dynamically typed | Statically typed | Type safety at compile time |
| Runtime errors | Compile-time errors | Catch bugs before deployment |
| `.env` config | `.env` config | Same approach |
| Eloquent ORM | Model trait | Similar API, better performance |
| Artisan CLI | Forge CLI | Similar commands |
| `$array` | `json!({...})` | JSON literals |
| `->` | `.` | Method chaining |
| No async/await | Async by default | Better concurrency |

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
use rf::Route;

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
async fn get_admins() -> Response {
    let admins = query! {
        User::where("role", "admin")
            .where("active", true)
            .orderBy("name", "asc")
            .limit(10)
            .get()
    };
    Response::json(admins)
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
async fn store(data: Json<Value>) -> Response {
    let user = User::create(json!({
        "name": data["name"],
        "email": data["email"]
    }));
    Response::json(user).status(201)
}

async fn update(id: i64) -> Response {
    User::updateById(id, json!({"name": "John Doe"}));
    Response::ok()
}

async fn destroy(id: i64) -> Response {
    User::destroy(id);
    Response::ok()
}
```

### Caching

**Laravel:**
```php
Cache::put('key', 'value', 3600);
$value = Cache::get('key');
$users = Cache::remember('users', 3600, fn() => User::all());
```

**RustForge:**
```rust
#[auto_await]
async fn cached_users() -> Response {
    let users = Cache::remember("users", 3600, || async { User::all() });
    Response::json(users)
}
```

### Authentication

**Laravel:**
```php
Auth::attempt(['email' => $email, 'password' => $password]);
$user = Auth::user();
Auth::logout();
```

**RustForge:**
```rust
#[auto_await]
async fn login(email: &str, password: &str) -> Response {
    if Auth::attempt(json!({ "email": email, "password": password })) {
        let user = Auth::user::<User>();
        Response::json(user)
    } else {
        Response::error(401, "Invalid credentials")
    }
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
use rf::Route;

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
use rf::Route;

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
use rf::Route;

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
// Note: there is no `rf-http` crate. `Request` lives in `rf-request`,
// `Response` in `rf-response`, routing in `rf-routing`/`rf-route-facade`.
use rf_request::Request;
// The JWT type is `JwtManager` (not `JwtAuth`):
use rf_auth::JwtManager;
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

### `rustforge!` Block - The Ultimate Laravel Experience

Write Rust exactly like Laravel PHP with zero boilerplate:

```rust
rustforge! {
    Model!(User: name, email, hidden password);

    async fn index() -> Response {
        let users = User::where("active", true).get();
        Response::json(users)
    }
}
```

**Benefits:**
| Feature | Before | After (with `rustforge!`) |
|---------|--------|---------------------------|
| Imports | `use rustforge::*;` | Automatic |
| Auto-await | `#[auto_await]` | Automatic |
| Await calls | `.await` everywhere | Automatic |
| `where` keyword | `query!()` or `r#where` | Just `where()` |

**Opt-out:** Use `#[sync]` for functions that shouldn't have auto_await.

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
| `now!` | Current datetime | `now!()`, `now!("%Y-%m-%d")` |
| `bcrypt!` | Password hashing | `bcrypt!(password)`, `bcrypt!(verify: pwd, hash)` |
| `back!` | Redirect back | `back!()`, `back!("/fallback")` |
| `view!` | Render view | `view!("welcome")`, `view!("users.index", data)` |
| `redirect!` | Create redirect | `redirect!("/home")`, `redirect!(route: "users.show", id = 1)` |
| `session!` | Session access | `session!("key")`, `session!(set: "key", value)` |
| `auth!` | Auth helpers | `auth!()`, `auth!(check)`, `auth!(logout)` |
| `csrf!` | CSRF token | `csrf!()`, `csrf!(field)`, `csrf!(meta)` |
| `cache!` | Cache access | `cache!("key")`, `cache!(put: "key", val, 3600)` |
| `logger!` | Logging | `logger!(info: "message")`, `logger!(error: msg)` |
| `event!` | Dispatch event | `event!(UserCreated { user_id: 123 })` |
| `storage!` | File storage | `storage!("file.txt")`, `storage!(put: "file", data)` |

### FormRequest - Laravel-style Validation

Define form requests with automatic validation, just like Laravel:

```rust
use rustforge::*;

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
        // Return true to allow, false for 403
        auth!(check)
    }

    fn messages() -> HashMap<&'static str, &'static str> {
        HashMap::from([
            ("email.required", "Email is required"),
            ("email.email", "Please provide a valid email"),
            ("password.min", "Password must be at least 8 characters"),
        ])
    }
}

// Use in handler - automatic validation!
async fn store(Validated(req): Validated<CreateUserRequest>) -> Response {
    let user = User::create(json!({
        "email": req.email,
        "password": bcrypt!(req.password),
        "name": req.name,
    })).await;
    Response::json(user).status(201)
}
```

**Available Validation Rules:**

| Category | Rules |
|----------|-------|
| Basic | `required`, `nullable`, `string`, `integer`, `numeric`, `boolean`, `array` |
| String | `email`, `url`, `ip`, `uuid`, `alpha`, `alpha_num`, `lowercase`, `uppercase`, `regex("pattern")` |
| Length | `min(n)`, `max(n)`, `between(min, max)`, `min_length(n)`, `max_length(n)`, `size(n)` |
| Date | `date`, `date_format("fmt")`, `before("date")`, `after("date")` |
| Database | `unique("table", "column")`, `exists("table", "column")` |
| Compare | `same("field")`, `different("field")`, `confirmed` |
| Conditional | `required_if("field", "value")`, `required_unless("field", "value")`, `required_with("field")`, `required_without("field")` |

### Exception Handler - Laravel-style Error Handling

Define a global exception handler:

```rust
use rustforge::*;

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
                    Response::json(json!({ "error": "Not found" })).status(404)
                } else {
                    view!("errors.404").status(404)
                }
            }
            _ => Response::error(500, "Server Error")
        }
    }

    // Custom exception reporting (logging, Sentry, etc.)
    fn report(error: &AppError) {
        logger!(error: "Application error: {:?}", error);
        // Send to Sentry, Bugsnag, etc.
    }
}
```

**Exception Helper Macros:**

| Macro | Description | Example |
|-------|-------------|---------|
| `abort_if!` | Abort if condition true | `abort_if!(user.is_banned(), 403, "Banned")` |
| `abort_unless!` | Abort unless condition true | `abort_unless!(user.can_edit(&post), 403)` |
| `report!` | Report without throwing | `report!(error)` |
| `rescue!` | Rescue with fallback | `rescue!(User::find(id).await, User::default())` |

**Example Usage:**

```rust
#[auto_await]
async fn show(id: i64) -> Response {
    // Abort if user cannot view
    abort_unless!(auth!(check), 401, "Please login");

    let user = User::find(id);
    abort_if!(user.is_none(), 404, "User not found");

    // Rescue with fallback
    let profile = rescue!(user.profile(), Profile::default());

    Response::json(json!({
        "user": user,
        "profile": profile
    }))
}
```

### Blade Templates - Laravel-style Templating

Write HTML templates with familiar Blade-like syntax:

```rust
use rustforge::*;

let user = User::find(1).await;
let posts = user.posts().get().await;

let html = blade! {
    <div class="container">
        @if let Some(user) = user {
            <h1>Welcome, {{ user.name }}!</h1>

            @if user.is_admin {
                <span class="badge badge-admin">Admin</span>
            } @else {
                <span class="badge">User</span>
            }

            <h2>Your Posts</h2>
            <ul>
            @foreach post in posts {
                <li>
                    <a href="/posts/{{ post.id }}">{{ post.title }}</a>
                </li>
            }
            </ul>
        } @else {
            <p>Please <a href="/login">log in</a></p>
        }

        @auth {
            <a href="/logout" class="btn">Logout</a>
        }

        @guest {
            <a href="/login" class="btn">Login</a>
            <a href="/register" class="btn">Register</a>
        }

        <form method="POST" action="/posts">
            @csrf
            @method("PUT")
            <input type="text" name="title" />
            <button type="submit">Submit</button>
        </form>
    </div>
};
```

**Available Blade Directives:**

| Category | Directive | Description |
|----------|-----------|-------------|
| **Control Flow** | `@if condition { }` | Conditional rendering |
| | `@else { }` | Else branch |
| | `@else if condition { }` | Else if branch |
| | `@foreach item in collection { }` | Loop iteration |
| | `@for expr { }` | For loop |
| | `@while condition { }` | While loop |
| | `@match expr { }` | Match expression |
| **Auth** | `@auth { }` | Content for authenticated users |
| | `@guest { }` | Content for guests |
| **Forms** | `@csrf` | CSRF token hidden input |
| | `@method("PUT")` | HTTP method spoofing |
| **Output** | `{{ expr }}` | Escaped output |
| | `{!! expr !!}` | Unescaped/raw HTML output |
| | `@json(data)` | JSON output |
| **Include** | `@include("partial")` | Include template |
| **Utility** | `@isset(var) { }` | Check if variable is set |
| | `@empty(collection) { }` | Check if collection is empty |
| | `@env("KEY")` | Environment variable |
| | `@rust { code }` | Execute Rust code |
| | `@class([...])` | Conditional CSS classes |

**Additional Template Macros:**

```rust
// Simple HTML template
let name = "World";
let html = html! {
    <div>Hello, {name}!</div>
};

// Define template sections
section!("content") {
    <h1>Page Content</h1>
    <p>This goes in the content section</p>
}

// Push content to a stack (for scripts/styles)
push!("scripts") {
    <script src="/js/app.js"></script>
}

// Render a stack
let scripts = stack!("scripts");
```

### Mailable & Notifications - Laravel-style Emails

Define structured emails with envelope, content, and attachments:

```rust
use rustforge::*;

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
            Attachment::from_path("/docs/getting-started.pdf")
                .as_("Getting Started Guide.pdf")
                .with_mime("application/pdf"),
        ]
    }
}

// Send email
Mail::to("user@example.com")
    .send(WelcomeEmail {
        user,
        activation_url: "https://rustforge.dev/activate/abc123".into(),
    })
    .await?;

// Queue for later sending
Mail::to("user@example.com")
    .queue(WelcomeEmail { user, activation_url })
    .delay(Duration::from_secs(60))
    .await?;
```

**Simple Attribute Syntax:**

```rust
#[mail(subject = "Welcome!", view = "emails.welcome")]
pub struct WelcomeEmail {
    pub user: User,
}

// Send
Mail::to(&user.email).send(WelcomeEmail { user }).await?;
```

**Notifications - Multi-channel Messaging:**

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
            .with("order", &self.order)
    }

    fn to_database(&self) -> Value {
        json!({
            "type": "order_shipped",
            "order_id": self.order.id,
            "message": format!("Order #{} has shipped!", self.order.id)
        })
    }

    fn to_slack(&self) -> SlackMessage {
        SlackMessage::new()
            .to("#orders")
            .content(format!("Order #{} has shipped!", self.order.id))
    }
}

// Send notification to a user
user.notify(OrderShipped { order }).await?;

// Send to multiple users
Notification::send(users, OrderShipped { order }).await?;
```

**Markdown Email Content:**

```rust
let content = markdown! {
    # Welcome {{ user.name }}!

    Thanks for joining us. Here's what you can do next:

    - Create your first project
    - Invite team members
    - Start building

    @component("button", url: "https://app.rustforge.dev")
        Get Started
    @endcomponent

    Thanks,
    The RustForge Team
};
```

**Mailable Methods:**

| Method | Description |
|--------|-------------|
| `envelope()` | Define email metadata (subject, from, to, cc, bcc, reply_to) |
| `content()` | Define email content (view or markdown) |
| `attachments()` | Add file attachments |
| `headers()` | Custom email headers |

**Notification Channels:**

| Channel | Method | Description |
|---------|--------|-------------|
| `Channel::Mail` | `to_mail()` | Send as email |
| `Channel::Database` | `to_database()` | Store in database |
| `Channel::Slack` | `to_slack()` | Send to Slack |
| `Channel::Broadcast` | `to_broadcast()` | WebSocket broadcast |

---

## Next Steps

- **[Installation Guide](Installation)** - Install RustForge
- **[Quick Start](Quick-Start)** - Build your first app
- **[Features](Features)** - Explore all features
- **[Examples](Examples)** - See code examples

---

*Need help migrating? Open an issue on [GitHub](https://github.com/Chregu12/RustForge/issues) or join our community.*
