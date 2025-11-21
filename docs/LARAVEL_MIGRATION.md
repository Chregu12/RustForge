# Laravel to RustForge Migration Guide

**For Laravel developers making the switch to RustForge**

---

## Table of Contents

1. [Introduction](#introduction)
2. [Why RustForge?](#why-rustforge)
3. [Key Differences: PHP vs Rust](#key-differences-php-vs-rust)
4. [Syntax Comparison](#syntax-comparison)
5. [Feature-by-Feature Migration](#feature-by-feature-migration)
6. [Common Patterns Translation](#common-patterns-translation)
7. [Gotchas and Tips](#gotchas-and-tips)
8. [Migration Strategy](#migration-strategy)
9. [FAQ](#faq)

---

## Introduction

Welcome, Laravel developer! If you're reading this, you're considering migrating to RustForge or just curious about how it compares to Laravel. This guide will help you understand the similarities and differences, and make the transition as smooth as possible.

### What is RustForge?

RustForge is a web application framework for Rust, heavily inspired by Laravel's elegant API and developer experience. It aims to bring Laravel's productivity and joy to the Rust ecosystem.

### Who Is This Guide For?

- Laravel developers exploring Rust
- Teams migrating from Laravel to RustForge
- Developers comfortable with Laravel who want Rust's performance and safety

---

## Why RustForge?

### What You Keep from Laravel

- ✅ **Familiar API** - Routes, controllers, models look similar
- ✅ **Eloquent-style ORM** - Relationships, eager loading, query builder
- ✅ **Blade templates** - Same syntax you know
- ✅ **Artisan-like CLI** - `forge` command works like `php artisan`
- ✅ **Service Container** - Dependency injection
- ✅ **Validation** - Fluent validation rules
- ✅ **Queue Jobs** - Background job processing
- ✅ **Developer Experience** - Built for productivity

### What You Gain with Rust

- 🚀 **Performance** - 10-100x faster than PHP
- 🔒 **Type Safety** - Catch errors at compile time
- 💪 **Memory Safety** - No null pointer exceptions, no memory leaks
- ⚡ **Concurrency** - Safe async/await without data races
- 📦 **Zero-Cost Abstractions** - High-level code, low-level performance
- 🛡️ **Reliability** - If it compiles, it probably works

### What Changes

- ❗ **Compiled Language** - No more "upload and refresh"
- ❗ **Strict Type System** - More upfront, fewer runtime errors
- ❗ **Ownership Model** - New concept to learn
- ❗ **Async/Await Required** - Different from PHP's synchronous model

---

## Key Differences: PHP vs Rust

### Philosophy

| Laravel (PHP) | RustForge (Rust) |
|---------------|------------------|
| Interpreted, dynamic | Compiled, static |
| Runtime errors | Compile-time errors |
| Duck typing | Strong typing |
| Reference counting | Ownership system |
| Garbage collected | No GC needed |
| Synchronous by default | Async/await required |

### Development Workflow

**Laravel:**
```bash
# Edit code
vim app/Http/Controllers/UserController.php
# Refresh browser - changes live immediately
```

**RustForge:**
```bash
# Edit code
vim src/controllers/user_controller.rs
# Rebuild (but watch mode auto-rebuilds)
forge serve --watch
```

### Error Discovery

**Laravel:** Runtime errors found when code executes

**RustForge:** Most errors found at compile time

---

## Syntax Comparison

### Routes

**Laravel:**
```php
// routes/web.php
Route::get('/', [HomeController::class, 'index']);
Route::get('/users/{id}', [UserController::class, 'show']);

Route::middleware(['auth'])->group(function () {
    Route::get('/dashboard', [DashboardController::class, 'index']);
});
```

**RustForge:**
```rust
// src/routes.rs
use rf_routing::{Router, Route};

pub fn register_routes() -> Router {
    Router::new()
        .route("/", Route::get(home_controller::index))
        .route("/users/:id", Route::get(user_controller::show))
        .group(|router| {
            router
                .middleware(auth_middleware())
                .route("/dashboard", Route::get(dashboard_controller::index))
        })
}
```

**Key Differences:**
- Rust uses `::` instead of `->`
- Parameters use `:id` instead of `{id}`
- Middleware applied via `.middleware()`
- Must explicitly import types

---

### Controllers

**Laravel:**
```php
namespace App\Http\Controllers;

class UserController extends Controller
{
    public function index()
    {
        $users = User::all();
        return view('users.index', compact('users'));
    }

    public function show($id)
    {
        $user = User::findOrFail($id);
        return view('users.show', compact('user'));
    }

    public function store(Request $request)
    {
        $validated = $request->validate([
            'name' => 'required|max:255',
            'email' => 'required|email|unique:users',
        ]);

        $user = User::create($validated);

        return redirect()->route('users.show', $user->id);
    }
}
```

**RustForge:**
```rust
use rf_http::{Request, Response};
use rf_views::View;
use crate::models::User;

pub async fn index(req: Request) -> Response {
    let users = User::all(req.db()).await.unwrap();
    View::make("users.index").with("users", users).render()
}

pub async fn show(req: Request) -> Response {
    let id: i32 = req.param("id").unwrap();
    let user = User::find(id, req.db()).await.unwrap();
    View::make("users.show").with("user", user).render()
}

pub async fn store(req: Request) -> Response {
    let validated = req.validate(|v| {
        v.rule("name", vec![Required, MaxLength(255)])
         .rule("email", vec![Required, Email, Unique("users", "email")])
    }).await.unwrap();

    let user = User::create(req.db(), validated).await.unwrap();

    Response::redirect(format!("/users/{}", user.id))
}
```

**Key Differences:**
- `async fn` required for async operations
- `.await` needed for async calls
- Explicit error handling with `Result` and `.unwrap()`
- Types must be declared
- `compact()` becomes `.with()`

---

### Eloquent Models

**Laravel:**
```php
namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class User extends Model
{
    protected $fillable = ['name', 'email', 'password'];

    protected $hidden = ['password'];

    protected $casts = [
        'email_verified_at' => 'datetime',
    ];

    public function posts()
    {
        return $this->hasMany(Post::class);
    }

    public function roles()
    {
        return $this->belongsToMany(Role::class);
    }
}
```

**RustForge:**
```rust
use rf_eloquent::{Model, HasMany, BelongsToMany};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

#[derive(Model, Serialize, Deserialize)]
#[table_name = "users"]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,

    #[serde(skip_serializing)]
    pub password: String,

    pub email_verified_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn posts(&self) -> HasMany<Post> {
        self.has_many()
    }

    pub fn roles(&self) -> BelongsToMany<Role> {
        self.belongs_to_many()
    }
}
```

**Key Differences:**
- Use `#[derive(Model)]` instead of `extends Model`
- Fields are explicit with types
- `$hidden` becomes `#[serde(skip_serializing)]`
- Relationships are methods returning relationship types
- Traits provide functionality instead of inheritance

---

### Queries

**Laravel:**
```php
// Simple queries
$users = User::all();
$user = User::find(1);
$active = User::where('active', true)->get();

// Complex queries
$users = User::where('active', true)
    ->where('created_at', '>', now()->subDays(7))
    ->orderBy('name')
    ->limit(10)
    ->get();

// With relationships
$users = User::with('posts')->get();

// Aggregates
$count = User::where('active', true)->count();
$avg = Order::where('status', 'completed')->avg('total');
```

**RustForge:**
```rust
use rf_eloquent::Model;
use chrono::Duration;

// Simple queries
let users = User::all(db).await?;
let user = User::find(1, db).await?;
let active = User::where_eq("active", true, db).await?;

// Complex queries
let users = User::query()
    .filter(user::Column::Active.eq(true))
    .filter(user::Column::CreatedAt.gt(Utc::now() - Duration::days(7)))
    .order_by_asc(user::Column::Name)
    .limit(10)
    .all(db)
    .await?;

// With relationships
let users = User::with("posts", db).await?;

// Aggregates
let count = User::where_eq("active", true, db).count(db).await?;
let avg = Order::where_eq("status", "completed", db)
    .avg("total", db)
    .await?;
```

**Key Differences:**
- All queries are async (`.await`)
- Explicit database connection (`db`)
- Use `?` operator for error propagation
- Column names are type-safe enums
- Filter methods are more explicit

---

### Blade Templates

**Laravel:**
```blade
@extends('layouts.app')

@section('content')
    <h1>Users</h1>

    @if($users->count() > 0)
        <ul>
            @foreach($users as $user)
                <li>
                    {{ $user->name }}
                    @if($user->isAdmin())
                        <span class="badge">Admin</span>
                    @endif
                </li>
            @endforeach
        </ul>
    @else
        <p>No users found.</p>
    @endif

    @auth
        <a href="{{ route('users.create') }}">Add User</a>
    @endauth
@endsection
```

**RustForge:**
```blade
@extends('layouts.app')

@section('content')
    <h1>Users</h1>

    @if(users.len() > 0)
        <ul>
            @foreach(user in users)
                <li>
                    {{ user.name }}
                    @if(user.is_admin())
                        <span class="badge">Admin</span>
                    @endif
                </li>
            @endforeach
        </ul>
    @else
        <p>No users found.</p>
    @endif

    @auth
        <a href="/users/create">Add User</a>
    @endauth
@endsection
```

**Key Differences:**
- Nearly identical syntax! 🎉
- `.count()` becomes `.len()`
- `->` becomes `.`
- Route helpers might be slightly different

---

### Validation

**Laravel:**
```php
$request->validate([
    'name' => 'required|max:255',
    'email' => 'required|email|unique:users',
    'age' => 'required|integer|min:18|max:100',
    'website' => 'nullable|url',
]);
```

**RustForge:**
```rust
req.validate(|v| {
    v.rule("name", vec![Required, MaxLength(255)])
     .rule("email", vec![Required, Email, Unique("users", "email")])
     .rule("age", vec![Required, Integer, Min(18), Max(100)])
     .rule("website", vec![Nullable, Url])
}).await?;
```

**Key Differences:**
- Validation is async
- Rules are function calls instead of strings
- Type-safe rule names

---

### Middleware

**Laravel:**
```php
namespace App\Http\Middleware;

class CheckAge
{
    public function handle($request, Closure $next)
    {
        if ($request->age < 18) {
            return redirect('home');
        }

        return $next($request);
    }
}
```

**RustForge:**
```rust
use rf_http::{Request, Response, Next};

pub async fn check_age(req: Request, next: Next) -> Response {
    if req.input::<i32>("age").unwrap_or(0) < 18 {
        return Response::redirect("/home");
    }

    next.run(req).await
}
```

**Key Differences:**
- Async middleware
- Explicit types for inputs
- `next.run()` instead of `$next()`

---

## Feature-by-Feature Migration

### Routing

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| `Route::get()` | `Route::get()` ✅ Same |
| `Route::post()` | `Route::post()` ✅ Same |
| `Route::resource()` | `Route::resource()` ✅ Same |
| `Route::middleware()` | `.middleware()` ✅ Same |
| `Route::group()` | `.group()` ✅ Same |
| `route('name')` | Named routes supported |

### Database

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| Eloquent ORM | `rf-eloquent` ✅ |
| Query Builder | `rf-orm` query builder ✅ |
| Migrations | `forge migrate` ✅ |
| Seeders | `forge db:seed` ✅ |
| Factories | `rf-testing` factories ✅ |

### Authentication

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| `Auth::user()` | `req.user()` ✅ |
| `Auth::login()` | `Auth::login()` ✅ |
| `@auth` | `@auth` ✅ Same |
| Password reset | Email-based reset ✅ |
| Email verification | Token-based verification ✅ |

### Validation

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| `required` | `Required` ✅ |
| `email` | `Email` ✅ |
| `unique:table` | `Unique("table", "column")` ✅ |
| `min:value` | `Min(value)` ✅ |
| `max:value` | `Max(value)` ✅ |
| Custom rules | Custom validators ✅ |

### Views

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| Blade templates | Blade templates ✅ Same |
| `@extends` | `@extends` ✅ Same |
| `@section` | `@section` ✅ Same |
| `@foreach` | `@foreach` ✅ Same |
| `@if` | `@if` ✅ Same |
| Components | Components ✅ |

### Queues & Jobs

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| Jobs | Jobs with `rf-jobs` ✅ |
| `dispatch()` | `.dispatch()` ✅ |
| Queue workers | `forge queue:work` ✅ |
| Failed jobs | Failed job handling ✅ |
| Horizon | `rf-horizon` dashboard ✅ |

### Caching

| Laravel Feature | RustForge Equivalent |
|----------------|---------------------|
| `Cache::get()` | `cache.get()` ✅ |
| `Cache::put()` | `cache.put()` ✅ |
| `Cache::remember()` | `cache.remember()` ✅ |
| Redis driver | Redis support ✅ |

---

## Common Patterns Translation

### 1. Resource Controllers

**Laravel:**
```php
class PostController extends Controller
{
    public function index() { }
    public function create() { }
    public function store(Request $request) { }
    public function show($id) { }
    public function edit($id) { }
    public function update(Request $request, $id) { }
    public function destroy($id) { }
}
```

**RustForge:**
```rust
pub async fn index(req: Request) -> Response { }
pub async fn create(req: Request) -> Response { }
pub async fn store(req: Request) -> Response { }
pub async fn show(req: Request) -> Response { }
pub async fn edit(req: Request) -> Response { }
pub async fn update(req: Request) -> Response { }
pub async fn destroy(req: Request) -> Response { }
```

### 2. Form Requests

**Laravel:**
```php
class StorePostRequest extends FormRequest
{
    public function rules()
    {
        return [
            'title' => 'required|max:255',
            'body' => 'required',
        ];
    }
}
```

**RustForge:**
```rust
#[derive(Deserialize, Validate)]
pub struct StorePostRequest {
    #[validate(length(min = 1, max = 255))]
    pub title: String,

    #[validate(length(min = 1))]
    pub body: String,
}
```

### 3. API Resources

**Laravel:**
```php
class UserResource extends JsonResource
{
    public function toArray($request)
    {
        return [
            'id' => $this->id,
            'name' => $this->name,
            'email' => $this->email,
            'created_at' => $this->created_at,
        ];
    }
}
```

**RustForge:**
```rust
#[derive(Serialize)]
pub struct UserResource {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

impl From<User> for UserResource {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            name: user.name,
            email: user.email,
            created_at: user.created_at,
        }
    }
}
```

### 4. Service Providers

**Laravel:**
```php
class AppServiceProvider extends ServiceProvider
{
    public function register()
    {
        $this->app->singleton(ApiClient::class, function ($app) {
            return new ApiClient(config('api.key'));
        });
    }
}
```

**RustForge:**
```rust
pub fn register_services(container: &mut ServiceContainer) {
    container.singleton::<ApiClient>(|c| {
        let config = c.resolve::<Config>();
        ApiClient::new(config.get("api.key"))
    });
}
```

---

## Gotchas and Tips

### 1. Async/Await Everywhere

**Gotcha:** Forgetting `.await` causes compilation errors.

```rust
// ❌ Won't compile
let users = User::all(db);

// ✅ Correct
let users = User::all(db).await?;
```

**Tip:** If you see "expected `Future`" error, you forgot `.await`.

### 2. Error Handling

**Gotcha:** No automatic exception handling.

**Laravel:**
```php
$user = User::findOrFail($id); // Throws exception if not found
```

**RustForge:**
```rust
// Option 1: Propagate error
let user = User::find(id, db).await?;

// Option 2: Handle error
let user = match User::find(id, db).await {
    Ok(u) => u,
    Err(_) => return Response::not_found(),
};
```

**Tip:** Use `?` operator to propagate errors up the call stack.

### 3. Ownership & Borrowing

**Gotcha:** Can't use value after moving it.

```rust
// ❌ Won't compile
let user = User::find(1, db).await?;
process_user(user); // Moves user
log_user(user); // Error: user was moved

// ✅ Correct - borrow instead
let user = User::find(1, db).await?;
process_user(&user); // Borrows user
log_user(&user); // OK: user still available
```

**Tip:** Use `&` to borrow instead of move.

### 4. Mutable vs Immutable

**Gotcha:** Variables are immutable by default.

```rust
// ❌ Won't compile
let user = User::find(1, db).await?;
user.name = "New Name"; // Error: user is immutable

// ✅ Correct
let mut user = User::find(1, db).await?;
user.name = "New Name".to_string(); // OK
```

**Tip:** Use `mut` when you need to modify.

### 5. String Types

**Gotcha:** `&str` vs `String` confusion.

```rust
// &str - borrowed, stack-allocated
let name: &str = "Alice";

// String - owned, heap-allocated
let name: String = "Alice".to_string();
let name: String = String::from("Alice");
```

**Tip:** Use `String` for owned data, `&str` for borrowed.

### 6. Database Connections

**Gotcha:** Must pass DB connection explicitly.

**Laravel:**
```php
$users = User::all(); // Connection implicit
```

**RustForge:**
```rust
let users = User::all(db).await?; // Connection explicit
```

**Tip:** Get connection from request: `req.db()`

### 7. Collection Methods

**Gotcha:** No fluent collections like Laravel.

**Laravel:**
```php
$names = $users->pluck('name')->unique()->sort();
```

**RustForge:**
```rust
use itertools::Itertools;

let mut names: Vec<_> = users.iter()
    .map(|u| &u.name)
    .unique()
    .collect();
names.sort();
```

**Tip:** Use iterator methods and `itertools` crate.

---

## Migration Strategy

### Option 1: Fresh Start (Recommended for New Projects)

Start a new RustForge project from scratch:

```bash
forge new my-app
```

**Pros:**
- Clean slate
- Idiomatic Rust from the start
- No legacy code

**Cons:**
- Longer initial development time
- Need to rebuild everything

### Option 2: Incremental Migration

Migrate feature by feature:

1. **Start with API** - Easier to migrate than frontend
2. **New features in RustForge** - Keep old features in Laravel
3. **Gradually rewrite** - One module at a time
4. **Share database** - Both apps read/write same DB

```
Laravel App (legacy)  ────┐
                          ├──→  PostgreSQL
RustForge App (new)  ─────┘
```

**Pros:**
- Gradual transition
- Lower risk
- Can migrate over months

**Cons:**
- Maintain two codebases
- Complexity in shared database

### Option 3: Proxy Pattern

Use RustForge as a proxy/gateway:

```
Browser → RustForge → Laravel (for old routes)
               └────→ RustForge handlers (for new routes)
```

**Pros:**
- Single entry point
- Transparent to users
- Easy rollback

**Cons:**
- Additional latency
- More complex deployment

---

## FAQ

### Q: Is RustForge production-ready?

**A:** RustForge is at 90% feature parity with Laravel. Core features (routing, ORM, authentication, jobs) are production-ready. Some advanced features are still in development.

### Q: Will my Laravel knowledge transfer?

**A:** Yes! About 80% of concepts transfer directly. The main learning curve is Rust syntax and ownership model, not the framework itself.

### Q: How much faster is RustForge vs Laravel?

**A:** Typically 10-100x faster depending on the workload. Database-heavy apps see less improvement than compute-heavy apps.

### Q: Can I use my existing MySQL/PostgreSQL database?

**A:** Yes! RustForge works with the same databases as Laravel.

### Q: What about packages/libraries?

**A:** Rust has a rich ecosystem (crates.io). Most Laravel package functionality has Rust equivalents, though you might need to adapt.

### Q: How long does it take to learn?

**A:** If you know Laravel:
- Basic Rust syntax: 1-2 weeks
- Ownership model: 2-4 weeks
- Proficient with RustForge: 4-8 weeks

### Q: Should I migrate my existing Laravel app?

**A:** Consider migrating if:
- ✅ You need better performance
- ✅ You want type safety
- ✅ You have compute-heavy workloads
- ✅ You value memory safety

Don't migrate if:
- ❌ Your Laravel app is working fine
- ❌ You don't have time to learn Rust
- ❌ Your team isn't interested in Rust

---

## Next Steps

1. **Try the Tutorials**
   - [Getting Started](./tutorials/01-getting-started.md)
   - [Building a Blog](./tutorials/02-building-a-blog/README.md)

2. **Read the Guides**
   - [Routing Guide](./guides/routing.md)
   - [Eloquent Guide](./guides/eloquent.md)
   - [Validation Guide](./guides/validation.md)

3. **Join the Community**
   - GitHub: [github.com/rustforge/rustforge](https://github.com/rustforge/rustforge)
   - Discord: [discord.gg/rustforge](https://discord.gg/rustforge)

4. **Learn Rust**
   - [The Rust Book](https://doc.rust-lang.org/book/)
   - [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
   - [Rustlings](https://github.com/rust-lang/rustlings)

---

## Conclusion

Migrating from Laravel to RustForge is not as daunting as it might seem. The framework APIs are intentionally similar, and your Laravel knowledge is directly applicable. The main learning curve is Rust itself, but the payoff in performance, safety, and reliability is worth it.

**Welcome to RustForge! We're excited to have you here.** 🚀

---

**Questions?** Ask on [Discord](https://discord.gg/rustforge) or open an issue on [GitHub](https://github.com/rustforge/rustforge/issues).
