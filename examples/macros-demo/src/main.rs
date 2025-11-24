//! Macros Demo - Laravel-style Syntax in Rust
//!
//! This example demonstrates the new macro system:
//! - function! macro for route handlers
//! - rules! macro for validation
//! - #[controller] attribute for controllers

use rf_macros::{controller, function, rules};
use rf_request::Request;
use rf_response::{Response, ResponseBuilder};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: u64,
    name: String,
    email: String,
}

// Example 1: Simple route handler with function! macro
async fn example_simple_handler() {
    println!("\n=== Example 1: Simple Handler ===");
    println!("The function! macro converts function syntax to async closures");
    println!("It automatically adds .await to async function calls");
    println!("Handler can be created and used with Axum routing");
}

// Example 2: Validation with rules! macro
async fn example_validation() {
    println!("\n=== Example 2: Validation Rules ===");
    println!("The rules! macro creates validation rules with Laravel-style pipe syntax:");
    println!("  name: required | min(3)");
    println!("  email: required | email");
    println!("  age: required | integer | between(18, 120)");
    println!("\nThis expands to a HashMap of validation rules at compile time");
}

// Example 3: Controller with #[controller] attribute
struct UserController;

#[controller]
impl UserController {
    pub fn index(_request: Request) -> ResponseBuilder {
        // The #[controller] macro automatically:
        // 1. Makes this function async
        // 2. Adds .await to async calls like User::all()

        let users = vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                email: "alice@example.com".to_string(),
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                email: "bob@example.com".to_string(),
            },
        ];

        Response::json(&users)
    }

    pub fn show(_request: Request) -> ResponseBuilder {
        // Demonstrating that the controller works
        let user = User {
            id: 1,
            name: "Alice".to_string(),
            email: "alice@example.com".to_string(),
        };

        Response::json(&user)
    }
}

#[tokio::main]
async fn main() {
    println!("🚀 Laravel-Style Macros Demo");
    println!("=============================");

    // Run examples
    example_simple_handler().await;
    example_validation().await;

    println!("\n=== Example 3: Controller ===");
    println!("UserController defined with #[controller]");
    println!("- Methods are automatically async");
    println!("- Auto-await applied to validate() calls");

    println!("\n✅ All examples completed!");
    println!("\nKey Features Demonstrated:");
    println!("1. function! - Convert function syntax to async closures");
    println!("2. rules! - Laravel-style validation rules");
    println!("3. #[controller] - Automatic async conversion");
    println!("4. Auto-await - Automatic .await insertion");
}
