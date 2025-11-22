use rf_views::prelude::*;
use serde::Serialize;

#[derive(Serialize)]
struct Post {
    id: u32,
    title: String,
    body: String,
    user: User,
    created_at: String,
}

#[derive(Serialize)]
struct User {
    id: u32,
    name: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a view engine
    let engine = ViewEngine::new("examples/views")?;

    // Example 1: Simple rendering
    let context = context! {
        "title" => "Welcome",
        "message" => "Hello from RustForge Views!"
    };

    println!("=== Example 1: Simple Context ===");
    // This would render if we had a welcome.tera template
    // let html = engine.render("welcome", &context)?;
    // println!("{}", html);

    // Example 2: Rendering with data
    let posts = vec![
        Post {
            id: 1,
            title: "First Post".to_string(),
            body: "This is the first post content. It's very interesting and full of useful information.".to_string(),
            user: User {
                id: 1,
                name: "Alice".to_string(),
            },
            created_at: "2025-01-01T10:00:00Z".to_string(),
        },
        Post {
            id: 2,
            title: "Second Post".to_string(),
            body: "This is the second post. It contains even more fascinating details about Rust and web development.".to_string(),
            user: User {
                id: 2,
                name: "Bob".to_string(),
            },
            created_at: "2025-01-02T14:30:00Z".to_string(),
        },
    ];

    println!("=== Example 2: Posts Listing ===");
    let html = engine.render_with_data(
        "posts/index",
        serde_json::json!({
            "posts": posts,
            "pagination": {
                "current_page": 1,
                "total_pages": 1,
            }
        }),
    )?;
    println!("{}", html);

    // Example 3: CSRF Token
    println!("\n=== Example 3: CSRF Token ===");
    engine.set_csrf_token("secure_token_12345");

    // Example 4: Flash Messages
    println!("\n=== Example 4: Flash Messages ===");
    engine.set_flash("success", "Post created successfully!");
    engine.set_flash("info", "Welcome back!");

    // Example 5: Validation Errors
    println!("\n=== Example 5: Validation Errors ===");
    let mut errors = std::collections::HashMap::new();
    errors.insert("title".to_string(), vec!["Title is required".to_string()]);
    errors.insert(
        "body".to_string(),
        vec!["Body must be at least 10 characters".to_string()],
    );
    engine.set_errors(errors);

    // Example 6: Old Input
    println!("\n=== Example 6: Old Input ===");
    let mut old_input = std::collections::HashMap::new();
    old_input.insert("title".to_string(), serde_json::json!("My Draft Title"));
    old_input.insert("body".to_string(), serde_json::json!("Draft content..."));
    engine.set_all_old_input(old_input);

    // Render create form with errors and old input
    let create_html = engine.render_with_data("posts/create", serde_json::json!({}))?;
    println!("Form with errors:");
    println!("{}", &create_html[..500.min(create_html.len())]); // Print first 500 chars

    Ok(())
}
