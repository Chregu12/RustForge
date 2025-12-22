//! # All Laravel Facades Demo
//!
//! This example demonstrates all Laravel-style facades working together.
//! All facades are now **synchronous** - no `.await` needed!

use rf::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct User {
    id: u64,
    email: String,
    name: String,
}

fn main() {
    println!("╔════════════════════════════════════════════════════╗");
    println!("║  RustForge: Laravel Facades Demo                  ║");
    println!("║  SYNC API - No .await needed!                     ║");
    println!("╚════════════════════════════════════════════════════╝\n");

    // ═══════════════════════════════════════════════════════
    // 1. Auth Facade
    // ═══════════════════════════════════════════════════════
    println!("1. Auth Facade");

    let user = User {
        id: 1,
        email: "developer@rustforge.com".to_string(),
        name: "Rust Developer".to_string(),
    };

    let _ = Auth::login(user.clone());
    println!("   Auth::login() - User: {}", user.name);
    println!("   Auth::check() = {}", Auth::check());
    println!("   Auth::id() = {:?}", Auth::id());
    println!();

    // ═══════════════════════════════════════════════════════
    // 2. Config Facade
    // ═══════════════════════════════════════════════════════
    println!("2. Config Facade");

    Config::set("app.name", "RustForge");
    println!("   Config::set('app.name', 'RustForge')");

    let app_name = Config::get("app.name");
    println!("   Config::get('app.name') = {:?}", app_name);
    println!();

    // ═══════════════════════════════════════════════════════
    // 3. Session Facade
    // ═══════════════════════════════════════════════════════
    println!("3. Session Facade");

    Session::put("user_id", json!(1));
    println!("   Session::put('user_id', 1)");

    let user_id = Session::get("user_id");
    println!("   Session::get('user_id') = {:?}", user_id);
    println!();

    // ═══════════════════════════════════════════════════════
    // 4. Log Facade
    // ═══════════════════════════════════════════════════════
    println!("4. Log Facade");

    Log::info("Application started");
    Log::debug("Debug message");
    Log::warning("Warning message");
    println!("   Log::info(), debug(), warning() called");
    println!();

    // ═══════════════════════════════════════════════════════
    // 5. View Facade
    // ═══════════════════════════════════════════════════════
    println!("5. View Facade");

    let mut view_data = std::collections::HashMap::new();
    view_data.insert("title".to_string(), json!("Welcome"));

    let _view = View::make("welcome", view_data);
    println!("   View::make('welcome') created");
    println!();

    // ═══════════════════════════════════════════════════════
    // 6. Logout
    // ═══════════════════════════════════════════════════════
    Auth::logout();
    println!("6. Auth::logout() - User logged out");
    println!("   Auth::check() = {}", Auth::check());
    println!();

    // ═══════════════════════════════════════════════════════
    // Summary
    // ═══════════════════════════════════════════════════════
    println!("╔════════════════════════════════════════════════════╗");
    println!("║  ALL FACADES: SYNC API - No .await!               ║");
    println!("╚════════════════════════════════════════════════════╝");
    println!();
    println!("Usage:");
    println!("   use rf::prelude::*;");
    println!("   // Then just call: Auth::check(), Config::get(), etc.");
}
