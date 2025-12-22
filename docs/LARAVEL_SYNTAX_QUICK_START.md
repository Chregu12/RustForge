# Laravel Syntax - Quick Start Guide

Get started with Laravel-style syntax in RustForge in under 5 minutes.

## Installation

Add the consolidated `rf` crate to your `Cargo.toml`:

```toml
[dependencies]
rf = "1.0"
```

## Basic Usage

### 1. Password Hashing

```rust
use rf::Hash;

// Hash a password
let hash = Hash::make("my_password");

// Verify a password
if Hash::check("my_password", &hash) {
    println!("Password correct!");
}
```

### 2. CSRF Protection

```rust
use rf::csrf_token;

// Generate a token
let token = csrf_token();

// Use in your forms
html! {
    <input type="hidden" name="_token" value={token} />
}
```

### 3. Validation Rules

```rust
use rf_macros::rules;

// Define rules with Laravel-style pipes
let validation = rules! {
    email: required | email,
    password: required | min(8),
    age: integer | between(18, 120),
};
```

### 4. Routes

```rust
use rf::Route;

// Simple routes
Route::get("/", "HomeController@index");
Route::post("/users", "UserController@store");

// Named routes
Route::get("/dashboard", "DashboardController@index")
    .name("dashboard");

// Middleware
Route::get("/admin", "AdminController@index")
    .middleware("auth");

// Groups
Route::group()
    .prefix("/api")
    .middleware("api")
    .routes(|group| {
        group.get("/users", "UserController@index");
        group.get("/posts", "PostController@index");
    });
```

## Running the Example

Try the complete working example:

```bash
cargo run --bin simple
```

Expected output:
```
🚀 Laravel Syntax Simple Example
=================================
✅ Hash works!
✅ CSRF token works!
✅ Validation rules work!
✅ Routes registered!
=================================
```

## What Works Now

✅ Hash::make() - Password hashing
✅ Hash::check() - Password verification
✅ csrf_token() - CSRF token generation
✅ rules! - Validation rules
✅ Route facade - Route registration

## What's Coming

- Route execution (not just registration)
- Request validation integration
- Response type system
- Middleware execution
- Named route resolution

## Learn More

- [Full Documentation](./LARAVEL_SYNTAX.md)
- [Implementation Status](./LARAVEL_SYNTAX_FIXES_REPORT.md)
- [Simple Example Code](../examples/laravel-syntax-simple/)

## Need Help?

Check the [wiki](../../wiki) or open an issue on GitHub.
