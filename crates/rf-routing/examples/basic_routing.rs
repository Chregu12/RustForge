//! Basic routing example demonstrating route groups and middleware.
//!
//! Run with: cargo run --example basic_routing

use axum::{
    extract::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use futures::future::BoxFuture;
use rf_routing::{pipeline, register_middleware, RouteGroup};

async fn home() -> &'static str {
    "Welcome Home!"
}

async fn about() -> &'static str {
    "About Us"
}

async fn api_users() -> &'static str {
    "API: Users List"
}

async fn api_posts() -> &'static str {
    "API: Posts List"
}

async fn admin_dashboard() -> &'static str {
    "Admin Dashboard"
}

#[tokio::main]
async fn main() {
    println!("=== Basic Routing Example ===\n");

    // Register global middleware
    register_middleware("auth", |req: Request, next: Next| {
        Box::pin(async move {
            println!("[Auth Middleware] Checking authentication...");
            Ok(next.run(req).await)
        }) as BoxFuture<'static, Result<Response, Response>>
    });

    register_middleware("throttle", |req: Request, next: Next| {
        Box::pin(async move {
            println!("[Throttle Middleware] Checking rate limit...");
            Ok(next.run(req).await)
        }) as BoxFuture<'static, Result<Response, Response>>
    });

    register_middleware("admin", |req: Request, next: Next| {
        Box::pin(async move {
            println!("[Admin Middleware] Verifying admin access...");
            Ok(next.run(req).await)
        }) as BoxFuture<'static, Result<Response, Response>>
    });

    // Create public routes
    let public_routes = Router::new()
        .route("/", get(home))
        .route("/about", get(about));

    println!("Public routes created:");
    println!("  GET /");
    println!("  GET /about\n");

    // Create API routes with prefix and middleware
    let api_group = RouteGroup::new()
        .prefix("/api")
        .middleware("auth")
        .middleware("throttle")
        .name("api.");

    let api_routes = Router::new()
        .route("/users", get(api_users))
        .route("/posts", get(api_posts));

    let api_routes = api_group.apply(api_routes);

    println!("API routes created:");
    println!("  GET /api/users (middleware: auth, throttle)");
    println!("  GET /api/posts (middleware: auth, throttle)\n");

    // Create admin routes with nested groups
    let admin_group = RouteGroup::new()
        .prefix("/admin")
        .middleware("auth")
        .middleware("admin")
        .name("admin.");

    let admin_routes = Router::new().route("/dashboard", get(admin_dashboard));

    let admin_routes = admin_group.apply(admin_routes);

    println!("Admin routes created:");
    println!("  GET /admin/dashboard (middleware: auth, admin)\n");

    // Combine all routes
    let app = public_routes.merge(api_routes).merge(admin_routes);

    println!("All routes combined successfully!");
    println!("\nExample middleware pipeline:");
    let pipe = pipeline().push("auth").push("throttle");
    println!("  Middleware stack: {:?}", pipe.stack());

    println!("\n=== Example Complete ===");
}
