//! Resource routing example demonstrating RESTful routes.
//!
//! Run with: cargo run --example resource_routing

use rf_routing::{
    api_resource, resource_except, resource_only, ControllerAction, ResourceCollection,
    ResourceRouter,
};

fn main() {
    println!("=== Resource Routing Example ===\n");

    // Full resource (all 7 actions)
    let posts = ResourceRouter::new("posts");
    println!("Full Resource: posts");
    println!("Actions: {:?}", posts.actions());
    println!("Paths:");
    for (action, path) in posts.paths(None) {
        println!("  {} {} -> {:?}", action.method(), path, action);
    }
    println!();

    // API resource (no create/edit forms)
    let users = api_resource("users");
    println!("API Resource: users");
    println!("Actions: {:?}", users.actions());
    println!("Paths:");
    for (action, path) in users.paths(None) {
        println!("  {} {} -> {:?}", action.method(), path, action);
    }
    println!();

    // Resource with 'only' filter
    let products = resource_only("products", vec![ControllerAction::Index, ControllerAction::Show]);
    println!("Resource with 'only': products");
    println!("Actions: {:?}", products.actions());
    println!("Paths:");
    for (action, path) in products.paths(None) {
        println!("  {} {} -> {:?}", action.method(), path, action);
    }
    println!();

    // Resource with 'except' filter
    let categories = resource_except("categories", vec![ControllerAction::Destroy]);
    println!("Resource with 'except': categories");
    println!("Actions: {:?}", categories.actions());
    println!("Number of actions: {}", categories.actions().len());
    println!();

    // Nested resources
    let posts_with_comments = ResourceRouter::new("posts").nest(ResourceRouter::new("comments"));
    println!("Nested Resource: posts -> comments");
    println!("Parent paths:");
    for (action, path) in posts_with_comments.paths(None) {
        println!("  {} {}", action.method(), path);
    }
    println!("\nNested comment paths:");
    if let Some(comments) = posts_with_comments.nested_resources().first() {
        for (action, path) in comments.paths(Some("/posts/:post_id")) {
            println!("  {} {}", action.method(), path);
        }
    }
    println!();

    // Shallow nested resources
    let shallow_comments = ResourceRouter::new("comments").shallow();
    println!("Shallow Nested Resource: comments");
    println!("When nested under /posts/:post_id:");
    for (action, path) in shallow_comments.paths(Some("/posts/:post_id")) {
        println!("  {} {} (shallow: {})", action.method(), path, shallow_comments.is_shallow());
    }
    println!();

    // Resource collection
    let collection = ResourceCollection::new()
        .add(ResourceRouter::new("posts"))
        .add(api_resource("users"))
        .add(resource_only(
            "products",
            vec![ControllerAction::Index, ControllerAction::Show],
        ));

    println!("Resource Collection:");
    for resource in collection.resources() {
        println!(
            "  {} - {} actions (API only: {})",
            resource.name(),
            resource.actions().len(),
            resource.is_api_only()
        );
    }
    println!();

    // Route naming
    let posts = ResourceRouter::new("posts");
    println!("Route Names for 'posts':");
    for (action, name) in posts.route_names(None) {
        println!("  {:?} -> {}", action, name);
    }
    println!();

    // Nested route naming
    println!("Route Names for nested 'posts.comments':");
    let comments = ResourceRouter::new("comments");
    for (action, name) in comments.route_names(Some("posts")) {
        println!("  {:?} -> {}", action, name);
    }

    println!("\n=== Example Complete ===");
}
