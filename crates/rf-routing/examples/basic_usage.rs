//! Basic usage example for rf-routing

use rf_routing::{route_params, NamedRoute, RouteRegistry, SignedUrlBuilder};

fn main() {
    println!("=== rf-routing Example ===\n");

    // Create route registry
    let mut registry = RouteRegistry::new();

    // Register routes
    registry.register(NamedRoute::new("home", "/"));
    registry.register(NamedRoute::new("users.index", "/users"));
    registry.register(NamedRoute::new("users.show", "/users/{id}"));
    registry.register(NamedRoute::new("posts.show", "/posts/{id}"));
    registry.register(NamedRoute::new(
        "posts.comments",
        "/posts/{post_id}/comments/{id}",
    ));

    // Generate URLs
    println!("Generated URLs:");

    let url = registry.url("home", &route_params! {});
    println!("  home: {}", url.unwrap());

    let url = registry.url("users.index", &route_params! {});
    println!("  users.index: {}", url.unwrap());

    let url = registry.url("users.show", &route_params! { "id" => 123 });
    println!("  users.show: {}", url.unwrap());

    let url = registry.url(
        "posts.comments",
        &route_params! {
            "post_id" => 456,
            "id" => 789
        },
    );
    println!("  posts.comments: {}\n", url.unwrap());

    // Signed URLs
    println!("Signed URLs:");
    const SECRET: &str = "my-secret-key";

    let signed = SignedUrlBuilder::new("/api/download/file.pdf", SECRET)
        .expires_in_hours(24)
        .build();

    println!("  URL: {}", signed.to_string());
    println!("  Valid: {}", signed.verify(SECRET));
    println!("  Expired: {}", signed.is_expired());
}
