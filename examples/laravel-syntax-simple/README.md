# Laravel Syntax Simple Example

This example demonstrates the **working** Laravel-style syntax features in RustForge.

## Features Demonstrated

### 1. Password Hashing with Hash Facade

```rust
use rf_global_helpers::Hash;

// Hash a password
let hash = Hash::make("my_password");

// Verify a password
if Hash::check("my_password", &hash) {
    println!("Password is correct!");
}
```

### 2. CSRF Token Generation

```rust
use rf_global_helpers::csrf_token;

// Generate a CSRF token
let token = csrf_token();
println!("CSRF Token: {}", token);
```

### 3. Validation Rules with Pipes

```rust
use rf_macros::rules;

// Define validation rules with Laravel-style pipe syntax
let rules = rules! {
    email: required | email,
    password: required | min(8) | max(72),
    age: integer | between(18, 120),
    name: required | min(3) | max(50),
};
```

### 4. Route Facade (Registration Only)

```rust
use rf_route_facade::Route;

// Simple routes
Route::get("/", "HomeController@index");
Route::post("/users", "UserController@store");

// Named routes
Route::get("/dashboard", "DashboardController@index")
    .name("dashboard");

// Routes with middleware
Route::get("/admin", "AdminController@index")
    .middleware("auth");

// Route groups
Route::group()
    .prefix("/api")
    .middleware("api")
    .routes(|group| {
        group.get("/users", "ApiUserController@index");
        group.get("/posts", "ApiPostController@index");
    });
```

## Running the Example

```bash
cargo run --bin simple
```

## Expected Output

```
🚀 Laravel Syntax Simple Example

=================================

1️⃣  Testing Hash::make() and Hash::check()...
   📝 Original: my_secure_password_123
   🔐 Hashed:   $2b$12$...
   ✅ Correct password verified
   ✅ Wrong password rejected

2️⃣  Testing csrf_token()...
   🎫 CSRF Token: 5aa5c85a-69e1-4e00-b23d-51dca7...
   ✅ Token length: 36 bytes
   ✅ Tokens are unique

3️⃣  Testing rules! macro...
   ✅ Basic rules compiled
   ✅ Advanced rules compiled
   ✅ Rules with parameters compiled

4️⃣  Testing Route facade...
   ✅ GET / registered
   ✅ POST /users registered
   ✅ PUT /users/:id registered
   ✅ DELETE /users/:id registered
   ✅ Named route registered
   ✅ Route with middleware registered
   ✅ Route group registered
   📊 Total routes registered: 8

=================================
✅ All Laravel-syntax features work!
=================================
```

## What Works

✅ **Hash::make()** - Password hashing with bcrypt
✅ **Hash::check()** - Password verification
✅ **csrf_token()** - CSRF token generation
✅ **rules!** - Validation rules macro with pipe syntax
✅ **Route facade** - Route registration (not execution)

## What Doesn't Work (Yet)

❌ **function! macro** - Not implemented correctly for parameter binding
❌ **Response types** - Need proper integration with handlers
❌ **request.validate()** - Request integration incomplete
❌ **Actual route execution** - Routes only register, don't execute

## Next Steps

1. **Fix function! macro** - Implement proper parameter binding
2. **Response integration** - Create unified Response type system
3. **Request validation** - Integrate validation with Request
4. **Route execution** - Make routes actually callable

## See Also

- [Complete Example](../laravel-syntax-complete) - Full blog example (has compile errors)
- [Laravel Syntax Documentation](../../docs/LARAVEL_SYNTAX.md) - Complete feature guide
