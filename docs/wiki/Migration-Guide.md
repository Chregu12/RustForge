# Migration Guide

This guide helps you migrate from Laravel, Actix-web, Rocket, or other frameworks to RustForge.

## Table of Contents

- [From Laravel (PHP)](#from-laravel-php)
- [From Actix-web](#from-actix-web)
- [From Rocket](#from-rocket)
- [From Axum](#from-axum)
- [Breaking Changes](#breaking-changes)

---

## From Laravel (PHP)

RustForge is heavily inspired by Laravel, so the transition should feel familiar.

### Key Differences

| Laravel (PHP) | RustForge (Rust) | Notes |
|---------------|------------------|-------|
| Dynamically typed | Statically typed | Type safety at compile time |
| Runtime errors | Compile-time errors | Catch bugs before deployment |
| `.env` config | `.env` config | Same approach |
| Eloquent ORM | SeaORM-based | Similar API, better performance |
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

**RustForge (Laravel-style!):**
```rust
use rf_route_facade::Route;

// Almost identical syntax to Laravel!
Route::get("/users", user_controller::index);
Route::post("/users", user_controller::store);
Route::middleware(&["auth"]).group(|| {
    Route::get("/profile", profile_controller::show);
});
```

### Controllers

**Laravel:**
```php
class UserController extends Controller
{
    public function index(Request $request)
    {
        $users = User::where('active', true)->get();
        return response()->json($users);
    }

    public function store(Request $request)
    {
        $validated = $request->validate([
            'name' => 'required|min:3',
            'email' => 'required|email',
        ]);

        $user = User::create($validated);
        return response()->json($user, 201);
    }
}
```

**RustForge:**
```rust
use rf_db_facade::DB;
use rf_http::{Response, Json};
use rf_validation::Validate;

pub async fn index() -> Result<Response, Error> {
    let users = DB::table("users")
        .r#where("active", true)
        .get().await?;

    Ok(Response::json(users))
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(length(min = 3))]
    pub name: String,

    #[validate(email)]
    pub email: String,
}

pub async fn store(Json(payload): Json<CreateUserRequest>) -> Result<Response, Error> {
    payload.validate()?;

    let user = DB::table("users").create(json!({
        "name": payload.name,
        "email": payload.email
    })).await?;

    Ok(Response::json(user).status(201))
}
```

### Unterschiede zu Laravel

| Aspekt | Laravel | RustForge | Grund |
|--------|---------|-----------|-------|
| Query | `User::where()` | `DB::table("users").r#where()` | Rust hat keine Magic Methods |
| Async | implizit | `.await?` | Rust ist explizit async |
| Validation | Inline Rules | Derive Macro | Compile-time Prüfung |
| Response | `response()->json()` | `Response::json()` | Ähnlich |

### Models

**Laravel:**
```php
class User extends Model
{
    protected $fillable = ['name', 'email', 'password'];

    protected $hidden = ['password'];

    public function posts()
    {
        return $this->hasMany(Post::class);
    }
}
```

**RustForge:**
```rust
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub name: String,
    pub email: String,

    #[serde(skip_serializing)]
    pub password: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post::Entity")]
    Posts,
}

impl Related<super::post::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}
```

### Migrations

**Laravel:**
```php
Schema::create('users', function (Blueprint $table) {
    $table->id();
    $table->string('email')->unique();
    $table->string('name');
    $table->string('password');
    $table->timestamps();
});
```

**RustForge:**
```rust
manager
    .create_table(
        Table::create()
            .table(Users::Table)
            .if_not_exists()
            .col(
                ColumnDef::new(Users::Id)
                    .integer()
                    .not_null()
                    .auto_increment()
                    .primary_key(),
            )
            .col(ColumnDef::new(Users::Email).string().not_null().unique_key())
            .col(ColumnDef::new(Users::Name).string().not_null())
            .col(ColumnDef::new(Users::Password).string().not_null())
            .col(
                ColumnDef::new(Users::CreatedAt)
                    .timestamp()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                ColumnDef::new(Users::UpdatedAt)
                    .timestamp()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned(),
    )
    .await
```

### Authentication

**Laravel:**
```php
Auth::attempt($credentials);
Auth::user();
Auth::logout();
```

**RustForge (Laravel-style!):**
```rust
use rf_auth_facade::Auth;

// Identical to Laravel!
Auth::attempt(json!({
    "email": "user@example.com",
    "password": "secret"
})).await?;

// Get current user
let user = Auth::user::<User>().await;

// Logout
Auth::logout().await;

// Additional Laravel-style methods
Auth::check().await;      // Check if authenticated
Auth::guest().await;      // Check if guest
Auth::id().await;         // Get user ID
Auth::login(user).await?; // Login a user
```

### Validation

**Laravel:**
```php
$request->validate([
    'email' => 'required|email|unique:users',
    'name' => 'required|min:3|max:50',
    'age' => 'required|integer|min:18',
]);
```

**RustForge:**
```rust
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUserRequest {
    #[validate(email, unique(table = "users", column = "email"))]
    pub email: String,

    #[validate(length(min = 3, max = 50))]
    pub name: String,

    #[validate(range(min = 18))]
    pub age: u8,
}
```

### Queues & Jobs

**Laravel:**
```php
class SendEmailJob implements ShouldQueue
{
    public function handle()
    {
        Mail::to($this->user)->send(new WelcomeMail());
    }
}

SendEmailJob::dispatch($user);
```

**RustForge:**
```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub user_id: i32,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: &JobContext) -> Result<(), Error> {
        let user = User::find_by_id(self.user_id)
            .one(ctx.db())
            .await?
            .unwrap();

        Mail::to(&user.email)
            .send()
            .await?;

        Ok(())
    }
}

Queue::push(SendEmailJob { user_id: 1 }).await?;
```

### Caching

**Laravel:**
```php
Cache::put('key', 'value', 3600);
$value = Cache::get('key');
Cache::remember('users', 3600, function () {
    return User::all();
});
```

**RustForge (Laravel-style!):**
```rust
use rf_cache_facade::Cache;
use std::time::Duration;

// Similar to Laravel
Cache::put("key", &"value", Duration::from_secs(3600)).await?;
let value: Option<String> = Cache::get("key").await?;
let users = Cache::remember("users", Duration::from_secs(3600), || async {
    Ok(User::find().all(&db).await?)
}).await?;

// Additional methods
Cache::has("key").await?;           // Check existence
Cache::forget("key").await?;        // Delete key
Cache::forever("key", &"value").await?;  // Store forever
Cache::flush().await?;              // Clear all
```

### CLI Commands

| Laravel Artisan | RustForge Forge | Description |
|----------------|-----------------|-------------|
| `php artisan make:model User` | `forge make:model User` | Create model |
| `php artisan make:controller UserController` | `forge make:controller UserController` | Create controller |
| `php artisan make:migration create_users` | `forge make:migration create_users` | Create migration |
| `php artisan migrate` | `forge migrate` | Run migrations |
| `php artisan migrate:rollback` | `forge migrate:rollback` | Rollback migration |
| `php artisan db:seed` | `forge db:seed` | Seed database |
| `php artisan queue:work` | `forge queue:work` | Start queue worker |
| `php artisan cache:clear` | `forge cache:clear` | Clear cache |
| `php artisan route:list` | `forge route:list` | List routes |

### Migration Checklist

- [ ] Install RustForge and dependencies
- [ ] Convert `.env` file (mostly compatible)
- [ ] Convert routes to RustForge syntax
- [ ] Convert models (add SeaORM derives)
- [ ] Convert controllers (add async/await)
- [ ] Convert migrations
- [ ] Update validation rules
- [ ] Convert jobs to Job trait
- [ ] Test all endpoints
- [ ] Update tests

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
        // Simple implementation
        let token = req.headers().get("Authorization");
        // Verify token
        next.run(req).await
    }
}
```

### Key Benefits of Migrating

- **Higher-level API**: Less boilerplate
- **Better ORM**: Integrated ORM instead of manual SQL
- **Built-in features**: Auth, caching, queues out of the box
- **Laravel-like syntax**: More intuitive for web developers
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

### State Management

**Rocket:**
```rust
#[get("/")]
fn index(db: &State<Database>) -> String {
    // Use db
}
```

**RustForge:**
```rust
async fn index(db: Database) -> Result<Response> {
    // Use db (injected automatically)
}
```

### Key Benefits

- **Async by default**: Better performance for I/O-heavy apps
- **No macros needed**: Cleaner syntax
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

### Extractors

**Axum:**
```rust
async fn create_user(
    Extension(db): Extension<Database>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<User>, StatusCode> {
    // ...
}
```

**RustForge:**
```rust
async fn create_user(
    Json(payload): Json<CreateUserRequest>,
    db: Database,
) -> Result<Response, Error> {
    // ...
}
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

The Storage API has been updated:

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

Queue now uses `rf-jobs` instead of `foundry-queue`:

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
   # etc.
   ```

2. **Update Imports**:
   ```bash
   # Use find and replace
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
