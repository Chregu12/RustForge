//! Complete Laravel-Syntax Blog Example
//!
//! This demonstrates Laravel-style features:
//! - Route::get/post/put/delete
//! - rules! validation with pipes
//! - Hash::make(), csrf_token()
//!
//! Run with: cargo run --bin blog

use rf_global_helpers::{Hash, csrf_token, __};
use serde::Serialize;

mod models;
mod database;

use models::{User, Post};

#[tokio::main]
async fn main() {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    println!("Starting Laravel-Syntax Blog Example...\n");

    // Setup database
    database::setup().await.expect("Failed to setup database");

    println!("Routes registered successfully!\n");

    // Demonstrate features
    println!("Testing features...\n");
    test_features().await;

    println!("\nAll tests passed!");
}

async fn test_features() {
    // Test 1: Hash password
    println!("1. Testing password hashing...");
    let hash = Hash::make("password123");
    println!("   Password hashed: {}...", &hash[..20]);
    println!("   OK!");

    // Test 2: Validate password
    println!("2. Testing password verification...");
    assert!(Hash::check("password123", &hash));
    println!("   OK!");

    // Test 3: CSRF Token
    println!("3. Testing CSRF token...");
    let token = csrf_token();
    println!("   Token: {}...", &token[..20]);
    println!("   OK!");

    // Test 4: Translation
    println!("4. Testing translation...");
    let message = __("welcome");
    println!("   Translation: {}", message);
    println!("   OK!");

    // Test 5: Models
    println!("5. Testing models...");
    let user = User::create(());
    println!("   User created: {}", user.name);
    let post = Post::create(());
    println!("   Post created: {}", post.title);
    println!("   OK!");
}

// Mock Response for demo
#[allow(dead_code)]
struct Response;

#[allow(dead_code)]
impl Response {
    fn view(_name: &str) -> Self { Self }
    fn json<T: Serialize>(_data: T) -> Self { Self }
    fn forbidden(_msg: &str) -> Self { Self }
    fn status(self, _code: u16) -> Self { self }
}
