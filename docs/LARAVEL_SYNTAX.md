# Laravel Syntax in RustForge

RustForge now supports Laravel-style syntax for a familiar developer experience. This document outlines all available Laravel-inspired features and their current implementation status.

## Table of Contents

1. [Routing](#routing)
2. [Validation](#validation)
3. [Global Helpers](#global-helpers)
4. [Implementation Status](#implementation-status)
5. [Migration Guide](#migration-guide)

---

## Routing

### Route Facade

RustForge provides a `Route` facade for defining routes in Laravel style.

#### Basic Routes

```rust
use rf::Route;

// GET route
Route::get("/", "HomeController@index");

// POST route
Route::post("/users", "UserController@store");

// PUT route
Route::put("/users/:id", "UserController@update");

// DELETE route
Route::delete("/users/:id", "UserController@destroy");
```

#### Named Routes

```rust
Route::get("/dashboard", "DashboardController@index")
    .name("dashboard");

Route::post("/login", "AuthController@login")
    .name("auth.login");
```

#### Middleware

```rust
// Single middleware
Route::get("/admin", "AdminController@index")
    .middleware("auth");

// Multiple middleware (chained)
Route::post("/users", "UserController@store")
    .middleware("auth")
    .middleware("validate");
```

#### Route Groups

```rust
Route::group()
    .prefix("/api")
    .middleware("api")
    .name("api.")
    .routes(|group| {
        group.get("/users", "UserController@index");
        group.get("/posts", "PostController@index");
    });
```

#### Special Routes

```rust
// Redirect routes
Route::redirect("/home", "/");
Route::permanent_redirect("/old-blog", "/posts");

// View routes (static pages)
Route::view("/about", "about");
```

### Comparison: Before vs After

#### Before (Axum-style)

```rust
use axum::{Router, routing::get, Json};
use axum::response::IntoResponse;

async fn list_users() -> impl IntoResponse {
    Json(vec!["user1", "user2"])
}

let app = Router::new()
    .route("/users", get(list_users));
```

#### After (Laravel-style)

```rust
use rf::Route;

Route::get("/users", "UserController@index")
    .name("users.index");
```

---

## Validation

### Rules Macro

Define validation rules using Laravel's pipe syntax.

#### Basic Rules

```rust
use rf_macros::rules;

let validation_rules = rules! {
    email: required | email,
    password: required | min(8),
};
```

#### Available Rules

```rust
rules! {
    // String rules
    name: required | min(3) | max(50),
    email: required | email,

    // Numeric rules
    age: integer | between(18, 120),
    price: numeric | min(0),

    // Boolean rules
    accepted: boolean,
}
```

#### Rules with Parameters

```rust
rules! {
    password: required | min(8) | max(72),
    username: required | min(3) | max(20),
    age: integer | between(18, 120),
}
```

### Comparison: Before vs After

#### Before (Struct-based)

```rust
use validator::Validate;

#[derive(Validate)]
struct CreateUser {
    #[validate(email)]
    email: String,

    #[validate(length(min = 8))]
    password: String,
}
```

#### After (Laravel-style)

```rust
use rf_macros::rules;

let rules = rules! {
    email: required | email,
    password: required | min(8),
};
```

---

## Global Helpers

### Hash Facade

Password hashing and verification using bcrypt.

```rust
use rf::Hash;

// Hash a password
let hash = Hash::make("my_password");

// Verify a password
if Hash::check("my_password", &hash) {
    println!("Password correct!");
}
```

**Features:**
- Uses bcrypt algorithm
- Automatic salt generation
- Cost factor: 12 (configurable)

### CSRF Token

Generate CSRF tokens for form protection.

```rust
use rf::csrf_token;

let token = csrf_token();
println!("CSRF Token: {}", token);
```

**Features:**
- UUID v4 based tokens
- Unique per generation
- 36 characters long

### Translation Helper

Laravel-style translation helper (currently returns key as placeholder).

```rust
use rf::__;

let message = __("auth.failed");
// Returns: "auth.failed"
```

### Redirect Helpers

```rust
use rf::{redirect, back};

// Redirect to URL
redirect("/dashboard")
    .with_success("Operation successful!");

// Redirect back
back()
    .with_errors(vec![("email", vec!["Invalid email"])])
    .with_input(request.except(&["password"]));
```

---

## Implementation Status

### ✅ Fully Working

| Feature | Status | Notes |
|---------|--------|-------|
| `Hash::make()` | ✅ Working | Bcrypt password hashing |
| `Hash::check()` | ✅ Working | Password verification |
| `csrf_token()` | ✅ Working | UUID-based token generation |
| `rules!` macro | ✅ Working | Validation rules compilation |
| Route registration | ✅ Working | Routes register successfully |

### ⚠️ Partially Working

| Feature | Status | Notes |
|---------|--------|-------|
| Route groups | ⚠️ Partial | Registration works, execution pending |
| Middleware | ⚠️ Partial | Registration works, not enforced |
| Named routes | ⚠️ Partial | Names registered, lookup pending |

### ❌ Not Yet Implemented

| Feature | Status | Notes |
|---------|--------|-------|
| `function!` macro | ❌ Not Working | Parameter binding issues |
| Response types | ❌ Not Working | No unified Response system |
| `request.validate()` | ❌ Not Working | Request integration incomplete |
| Route execution | ❌ Not Working | Only registration, no execution |
| `request.user()` | ❌ Not Working | Auth integration pending |
| Database rules | ❌ Not Working | `unique()` requires DB connection |

---

## Migration Guide

### Step 1: Update Dependencies

```toml
[dependencies]
rf = "1.0"
```

### Step 2: Convert Routes

**Before:**
```rust
use axum::{Router, routing::get};

let app = Router::new()
    .route("/users", get(list_users))
    .route("/users/:id", get(show_user));
```

**After:**
```rust
use rf::Route;

Route::get("/users", "UserController@index");
Route::get("/users/:id", "UserController@show");
```

### Step 3: Convert Validation

**Before:**
```rust
#[derive(Validate)]
struct CreateUser {
    #[validate(email)]
    email: String,
    #[validate(length(min = 8))]
    password: String,
}
```

**After:**
```rust
let rules = rules! {
    email: required | email,
    password: required | min(8),
};
```

### Step 4: Use Global Helpers

**Password Hashing:**
```rust
use rf::Hash;

// Hash password
let hash = Hash::make(&password);

// Store in database
user.password = hash;
```

**CSRF Protection:**
```rust
use rf::csrf_token;

// Generate token
let token = csrf_token();

// Include in form
html! {
    <input type="hidden" name="_token" value={token} />
}
```

---

## Examples

### Working Example

See [`examples/laravel-syntax-simple`](../examples/laravel-syntax-simple) for a complete working example demonstrating:

- Hash::make() and Hash::check()
- csrf_token()
- rules! macro
- Route registration

**Run it:**
```bash
cargo run --bin simple
```

### Complete Example (Work in Progress)

See [`examples/laravel-syntax-complete`](../examples/laravel-syntax-complete) for a full blog application example.

**Note:** This example has compile errors due to incomplete features (`function!` macro, Response types, etc.)

---

## Roadmap

### Phase 1: Core Features (Current)
- ✅ Hash facade
- ✅ CSRF tokens
- ✅ Validation rules macro
- ✅ Route registration

### Phase 2: Request/Response Integration
- ❌ Fix `function!` macro
- ❌ Unified Response type system
- ❌ `request.validate()` integration
- ❌ `request.user()` auth integration

### Phase 3: Route Execution
- ❌ Actual route handling
- ❌ Middleware execution
- ❌ Named route resolution
- ❌ Route model binding

### Phase 4: Advanced Features
- ❌ Database validation rules
- ❌ Event system integration
- ❌ Translation system
- ❌ Form request classes

---

## Contributing

### Priority Fixes Needed

1. **`function!` macro** - Fix parameter binding for `request: Request, id: i32`
2. **Response types** - Create unified `Response` type with `json()`, `view()`, `forbidden()`, etc.
3. **Route execution** - Make registered routes actually callable
4. **Request validation** - Integrate `request.validate(rules!)` properly

### How to Help

1. Pick a feature from the "Not Yet Implemented" section
2. Create tests in `examples/laravel-syntax-simple`
3. Implement the feature
4. Update this documentation

---

## FAQ

### Why isn't the complete example working?

The complete example (`laravel-syntax-complete`) demonstrates the **target API** we're building towards. Many features are not yet implemented, which is why it has compile errors. Use `laravel-syntax-simple` to see what currently works.

### Can I use this in production?

Not yet. The Laravel syntax features are still in development. Only the features marked as "✅ Fully Working" are safe to use.

### How is this different from Axum?

RustForge provides a Laravel-style API layer **on top of** Axum. Under the hood, it still uses Axum for routing and HTTP handling, but provides a more familiar syntax for Laravel developers.

### Will this support all Laravel features?

Our goal is 100% Laravel parity where it makes sense. Some Laravel features (like Blade templates) may have Rust-native alternatives that better fit the ecosystem.

---

## See Also

- [RustForge Consolidated Crate](../crates/rf/README.md)
- [Global Helpers Documentation](../crates/rf-global-helpers/README.md)
- [Validation Documentation](../crates/rf-validation/README.md)
- [Macros Documentation](../crates/rf-macros/README.md)
