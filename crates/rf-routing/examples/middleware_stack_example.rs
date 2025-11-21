//! Middleware Stack Example
//!
//! This example demonstrates how to organize and apply middleware using the middleware stack.

use rf_routing::{
    middleware_pipeline::{register_middleware, MiddlewareRegistry},
    middleware_stack::{MiddlewareStack, MiddlewareStackBuilder},
};
use std::sync::Arc;

fn main() {
    println!("=== Middleware Stack Example ===\n");

    // 1. Register middleware handlers
    setup_middleware();

    // 2. Create middleware stack
    let stack = setup_middleware_stack();

    // 3. Demonstrate middleware resolution
    demonstrate_resolution(&stack);

    // 4. Demonstrate builder pattern
    demonstrate_builder();
}

fn setup_middleware() {
    println!("1. Registering middleware handlers...");

    // Register global middleware
    register_middleware("cors", |req, next| {
        Box::pin(async move {
            println!("  [CORS] Adding CORS headers");
            Ok(next.run(req).await)
        })
    });

    register_middleware("logging", |req, next| {
        Box::pin(async move {
            println!("  [LOGGING] Request logged");
            Ok(next.run(req).await)
        })
    });

    register_middleware("compression", |req, next| {
        Box::pin(async move {
            println!("  [COMPRESSION] Response compressed");
            Ok(next.run(req).await)
        })
    });

    // Register authentication middleware
    register_middleware("auth", |req, next| {
        Box::pin(async move {
            println!("  [AUTH] Checking authentication");
            Ok(next.run(req).await)
        })
    });

    register_middleware("session", |req, next| {
        Box::pin(async move {
            println!("  [SESSION] Loading session");
            Ok(next.run(req).await)
        })
    });

    // Register specialized middleware
    register_middleware("csrf", |req, next| {
        Box::pin(async move {
            println!("  [CSRF] Verifying CSRF token");
            Ok(next.run(req).await)
        })
    });

    register_middleware("throttle", |req, next| {
        Box::pin(async move {
            println!("  [THROTTLE] Rate limiting check");
            Ok(next.run(req).await)
        })
    });

    register_middleware("validate", |req, next| {
        Box::pin(async move {
            println!("  [VALIDATE] Validating request");
            Ok(next.run(req).await)
        })
    });

    println!("✓ Middleware registered\n");
}

fn setup_middleware_stack() -> MiddlewareStack {
    println!("2. Setting up middleware stack...");

    let stack = MiddlewareStack::new();

    // Add global middleware (applied to ALL routes)
    stack.add_global("cors");
    stack.add_global("logging");
    stack.add_global("compression");
    println!("  ✓ Global middleware: cors, logging, compression");

    // Create middleware groups
    stack.add_group(
        "web",
        vec![
            "session".to_string(),
            "csrf".to_string(),
        ],
    );
    println!("  ✓ Group 'web': session, csrf");

    stack.add_group(
        "api",
        vec![
            "auth".to_string(),
            "throttle".to_string(),
        ],
    );
    println!("  ✓ Group 'api': auth, throttle");

    stack.add_group(
        "admin",
        vec![
            "auth".to_string(),
            "session".to_string(),
        ],
    );
    println!("  ✓ Group 'admin': auth, session");

    // Add route-specific middleware
    stack.add_route_middleware(
        "users.create",
        vec!["validate".to_string()],
    );
    println!("  ✓ Route 'users.create': validate");

    stack.add_route_middleware(
        "posts.store",
        vec!["validate".to_string()],
    );
    println!("  ✓ Route 'posts.store': validate\n");

    stack
}

fn demonstrate_resolution(stack: &MiddlewareStack) {
    println!("3. Demonstrating middleware resolution...\n");

    // Example 1: Web route
    println!("Example 1: Web route (login form)");
    println!("  Route: login.show");
    println!("  Groups: [web]");
    let middleware = stack.resolve("login.show", &vec!["web".to_string()]);
    println!("  Resolved middleware order:");
    for (i, mw) in middleware.iter().enumerate() {
        println!("    {}. {}", i + 1, mw);
    }
    println!();

    // Example 2: API route
    println!("Example 2: API route (list users)");
    println!("  Route: api.users.index");
    println!("  Groups: [api]");
    let middleware = stack.resolve("api.users.index", &vec!["api".to_string()]);
    println!("  Resolved middleware order:");
    for (i, mw) in middleware.iter().enumerate() {
        println!("    {}. {}", i + 1, mw);
    }
    println!();

    // Example 3: API route with validation
    println!("Example 3: API route with validation (create user)");
    println!("  Route: users.create");
    println!("  Groups: [api]");
    let middleware = stack.resolve("users.create", &vec!["api".to_string()]);
    println!("  Resolved middleware order:");
    for (i, mw) in middleware.iter().enumerate() {
        println!("    {}. {}", i + 1, mw);
    }
    println!();

    // Example 4: Admin route with multiple groups
    println!("Example 4: Admin route (manage users)");
    println!("  Route: admin.users.edit");
    println!("  Groups: [web, admin]");
    let middleware = stack.resolve(
        "admin.users.edit",
        &vec!["web".to_string(), "admin".to_string()],
    );
    println!("  Resolved middleware order:");
    for (i, mw) in middleware.iter().enumerate() {
        println!("    {}. {}", i + 1, mw);
    }
    println!("  Note: Duplicate 'session' removed, first occurrence kept\n");

    // Example 5: Route with no groups
    println!("Example 5: Route with no groups (public page)");
    println!("  Route: home.index");
    println!("  Groups: []");
    let middleware = stack.resolve("home.index", &vec![]);
    println!("  Resolved middleware order:");
    for (i, mw) in middleware.iter().enumerate() {
        println!("    {}. {}", i + 1, mw);
    }
    println!();
}

fn demonstrate_builder() {
    println!("4. Demonstrating builder pattern...\n");

    let stack = MiddlewareStackBuilder::new()
        .global("cors")
        .global("logging")
        .group(
            "api",
            vec!["auth".to_string(), "throttle".to_string()],
        )
        .route("users.create", vec!["validate".to_string()])
        .build();

    println!("  ✓ Built stack with:");
    println!("    - 2 global middleware");
    println!("    - 1 group (api)");
    println!("    - 1 route-specific middleware");

    let middleware = stack.resolve("users.create", &vec!["api".to_string()]);
    println!("\n  Resolved for 'users.create' with 'api' group:");
    for (i, mw) in middleware.iter().enumerate() {
        println!("    {}. {}", i + 1, mw);
    }
    println!();
}
