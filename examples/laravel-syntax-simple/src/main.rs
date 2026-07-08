//! Simplified Laravel-Syntax Example
//!
//! This demonstrates the working Laravel-style features:
//! - Hash::make() / Hash::check()
//! - csrf_token()
//! - rules! validation macro
//! - Route facade (registration only)
//!
//! Run with: cargo run --bin simple

use rf::{rules, Route, Hash, csrf_token};

#[tokio::main]
async fn main() {
    println!("🚀 Laravel Syntax Simple Example\n");
    println!("=================================\n");

    // Test 1: Hash - Password Hashing
    println!("1️⃣  Testing Hash::make() and Hash::check()...");
    test_hash();

    // Test 2: CSRF Token
    println!("\n2️⃣  Testing csrf_token()...");
    test_csrf();

    // Test 3: Validation Rules
    println!("\n3️⃣  Testing rules! macro...");
    test_validation_rules();

    // Test 4: Route Registration
    println!("\n4️⃣  Testing Route facade...");
    test_routes();

    println!("\n=================================");
    println!("✅ All Laravel-syntax features work!");
    println!("=================================\n");
}

/// Test Hash::make() and Hash::check()
fn test_hash() {
    let password = "my_secure_password_123";

    // Hash the password
    let hash = Hash::make(password);
    println!("   📝 Original: {}", password);
    println!("   🔐 Hashed:   {}...", &hash[..40]);

    // Check correct password
    assert!(Hash::check(password, &hash), "Password check failed!");
    println!("   ✅ Correct password verified");

    // Check wrong password
    assert!(!Hash::check("wrong_password", &hash), "Wrong password should fail!");
    println!("   ✅ Wrong password rejected");
}

/// Test csrf_token()
fn test_csrf() {
    let token = csrf_token();
    println!("   🎫 CSRF Token: {}...", &token[..30]);

    // Token should be sufficiently long
    assert!(token.len() >= 32, "CSRF token too short!");
    println!("   ✅ Token length: {} bytes", token.len());

    // Generate another token (should be different)
    let token2 = csrf_token();
    assert_ne!(token, token2, "Tokens should be different!");
    println!("   ✅ Tokens are unique");
}

/// Test rules! macro
fn test_validation_rules() {
    // Basic rules
    let _rules1 = rules! {
        email: required | email,
        password: required | min(8),
    };
    println!("   ✅ Basic rules compiled");

    // Advanced rules with pipes
    let _rules2 = rules! {
        email: required | email,
        password: required | min(8) | max(72),
        age: integer | between(18, 120),
        name: required | min(3) | max(50),
    };
    println!("   ✅ Advanced rules compiled");

    // Rules with parameters
    let _rules3 = rules! {
        username: required | min(3) | max(20),
        password: required | min(8),
    };
    println!("   ✅ Rules with parameters compiled");
}

/// Test Route facade registration
fn test_routes() {
    // Register simple routes
    Route::get("/", "HomeController@index");
    println!("   ✅ GET / registered");

    Route::post("/users", "UserController@store");
    println!("   ✅ POST /users registered");

    Route::put("/users/{id}", "UserController@update");
    println!("   ✅ PUT /users/{{id}} registered");

    Route::delete("/users/{id}", "UserController@destroy");
    println!("   ✅ DELETE /users/{{id}} registered");

    // Named routes
    Route::get("/dashboard", "DashboardController@index")
        .name("dashboard");
    println!("   ✅ Named route registered");

    // Routes with middleware
    Route::get("/admin", "AdminController@index")
        .middleware("auth");
    println!("   ✅ Route with middleware registered");

    // Route groups
    Route::group()
        .prefix("/api")
        .middleware("api")
        .routes(|group| {
            group.get("/users", "ApiUserController@index");
            group.get("/posts", "ApiPostController@index");
        });
    println!("   ✅ Route group registered");

    // Count registered routes
    use rf::global_router;
    let router = global_router();
    let route_count = router.routes().len();
    println!("   📊 Total routes registered: {}", route_count);
}
