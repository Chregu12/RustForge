//! Laravel vs rf-routing comparison example.
//!
//! This example shows how rf-routing features map to Laravel's routing.
//!
//! Run with: cargo run --example laravel_comparison

use rf_routing::{
    api_resource, route_params, ControllerAction, MiddlewareGroup, NamedRoute, ResourceRouter,
    RouteGroup, RouteRegistry,
};

fn main() {
    println!("=== Laravel vs rf-routing Comparison ===\n");

    // ============================================================================
    // Named Routes
    // ============================================================================
    println!("1. NAMED ROUTES");
    println!("---------------");

    println!("\nLaravel:");
    println!(r#"  Route::get('/users/{{id}}', [UserController::class, 'show'])->name('users.show');"#);
    println!(r#"  route('users.show', ['id' => 123]); // "/users/123""#);

    println!("\nrf-routing:");
    let mut registry = RouteRegistry::new();
    registry.register(NamedRoute::new("users.show", "/users/{id}"));
    let params = route_params! { "id" => 123 };
    let url = registry.url("users.show", &params).unwrap();
    println!(r#"  let route = NamedRoute::new("users.show", "/users/{{id}}");"#);
    println!(r#"  registry.url("users.show", &params); // "{}""#, url);
    println!();

    // ============================================================================
    // Route Groups
    // ============================================================================
    println!("2. ROUTE GROUPS");
    println!("---------------");

    println!("\nLaravel:");
    println!(r#"  Route::prefix('api')"#);
    println!(r#"      ->middleware(['auth', 'throttle'])"#);
    println!(r#"      ->name('api.')"#);
    println!(r#"      ->group(function () {{"#);
    println!(r#"          Route::get('users', [UserController::class, 'index']);"#);
    println!(r#"      }});"#);

    println!("\nrf-routing:");
    let api_group = RouteGroup::new()
        .prefix("/api")
        .middleware("auth")
        .middleware("throttle")
        .name("api.");
    println!(r#"  let group = RouteGroup::new()"#);
    println!(r#"      .prefix("/api")"#);
    println!(r#"      .middleware("auth")"#);
    println!(r#"      .middleware("throttle")"#);
    println!(r#"      .name("api.");"#);
    println!(
        "  // Prefix: {:?}, Middleware: {:?}",
        api_group.get_prefix(),
        api_group.get_middleware()
    );
    println!();

    // ============================================================================
    // Nested Groups
    // ============================================================================
    println!("3. NESTED GROUPS");
    println!("----------------");

    println!("\nLaravel:");
    println!(r#"  Route::prefix('api')->group(function () {{"#);
    println!(r#"      Route::prefix('v1')->group(function () {{"#);
    println!(r#"          Route::get('users', ...);"#);
    println!(r#"      }});"#);
    println!(r#"  }}); // Creates /api/v1/users"#);

    println!("\nrf-routing:");
    let parent = RouteGroup::new().prefix("/api").name("api.");
    let child = RouteGroup::new().prefix("/v1").name("v1.");
    let nested = parent.nest(child);
    println!(r#"  let nested = parent.nest(child);"#);
    println!(
        "  // Prefix: {:?}, Name: {:?}",
        nested.get_prefix(),
        nested.get_name()
    );
    println!();

    // ============================================================================
    // Resource Routing
    // ============================================================================
    println!("4. RESOURCE ROUTING");
    println!("-------------------");

    println!("\nLaravel:");
    println!(r#"  Route::resource('posts', PostController::class);"#);
    println!("  // Creates: index, create, store, show, edit, update, destroy");

    println!("\nrf-routing:");
    let posts = ResourceRouter::new("posts");
    println!(r#"  let posts = ResourceRouter::new("posts");"#);
    println!("  // Actions: {:?}", posts.actions());
    println!("  // Paths:");
    for (action, path) in posts.paths(None).iter().take(3) {
        println!("    {} {}", action.method(), path);
    }
    println!("    ...");
    println!();

    // ============================================================================
    // API Resources
    // ============================================================================
    println!("5. API RESOURCES");
    println!("----------------");

    println!("\nLaravel:");
    println!(r#"  Route::apiResource('posts', PostController::class);"#);
    println!("  // Creates: index, store, show, update, destroy (no create/edit)");

    println!("\nrf-routing:");
    let api_posts = api_resource("posts");
    println!(r#"  let posts = api_resource("posts");"#);
    println!("  // Actions: {:?}", api_posts.actions());
    println!("  // Count: {}", api_posts.actions().len());
    println!();

    // ============================================================================
    // Resource Filtering
    // ============================================================================
    println!("6. RESOURCE FILTERING");
    println!("---------------------");

    println!("\nLaravel:");
    println!(r#"  Route::resource('posts', PostController::class)"#);
    println!(r#"      ->only(['index', 'show']);"#);

    println!("\nrf-routing:");
    let filtered = ResourceRouter::new("posts")
        .only(vec![ControllerAction::Index, ControllerAction::Show]);
    println!(r#"  let posts = ResourceRouter::new("posts")"#);
    println!(r#"      .only(vec![ControllerAction::Index, ControllerAction::Show]);"#);
    println!("  // Actions: {:?}", filtered.actions());
    println!();

    // ============================================================================
    // Shallow Nesting
    // ============================================================================
    println!("7. SHALLOW NESTING");
    println!("------------------");

    println!("\nLaravel:");
    println!(r#"  Route::resource('posts.comments', CommentController::class)"#);
    println!(r#"      ->shallow();"#);
    println!(r#"  // Creates: /posts/{{post}}/comments/{{comment}} becomes /comments/{{comment}}"#);

    println!("\nrf-routing:");
    let shallow = ResourceRouter::new("comments").shallow();
    println!(r#"  let comments = ResourceRouter::new("comments").shallow();"#);
    let paths = shallow.paths(Some("/posts/:post_id"));
    let show_path = paths
        .iter()
        .find(|(a, _)| *a == ControllerAction::Show)
        .map(|(_, p)| p);
    println!("  // Show path: {:?}", show_path);
    println!();

    // ============================================================================
    // Middleware Groups
    // ============================================================================
    println!("8. MIDDLEWARE GROUPS");
    println!("--------------------");

    println!("\nLaravel:");
    println!(r#"  // In RouteServiceProvider:"#);
    println!(r#"  protected $middlewareGroups = ["#);
    println!(r#"      'web' => ['session', 'csrf', 'errors'],"#);
    println!(r#"      'api' => ['auth:api', 'throttle:60,1'],"#);
    println!(r#"  ];"#);

    println!("\nrf-routing:");
    let web_group = MiddlewareGroup::new("web")
        .add("session")
        .add("csrf")
        .add("errors");
    let api_middleware = MiddlewareGroup::new("api")
        .add("auth:api")
        .add("throttle:60,1");
    println!(r#"  let web = MiddlewareGroup::new("web")"#);
    println!(r#"      .add("session").add("csrf").add("errors");"#);
    println!(
        "  // Middleware: {:?}",
        web_group.middleware()
    );
    println!();

    // ============================================================================
    // Controller Actions
    // ============================================================================
    println!("9. CONTROLLER ACTIONS");
    println!("---------------------");

    println!("\nLaravel:");
    println!(r#"  Route::get('users', [UserController::class, 'index']);"#);
    println!(r#"  Route::post('users', [UserController::class, 'store']);"#);
    println!(r#"  Route::get('users/{{id}}', [UserController::class, 'show']);"#);

    println!("\nrf-routing:");
    println!("  // Using ControllerAction enum:");
    for action in ControllerAction::resource_actions() {
        println!(
            "    {:?}: {} {}",
            action,
            action.method(),
            action.path("users")
        );
    }
    println!();

    // ============================================================================
    // Summary
    // ============================================================================
    println!("=== FEATURE PARITY SUMMARY ===");
    println!("✓ Named routes with parameters");
    println!("✓ Route groups with prefix/middleware/name");
    println!("✓ Nested route groups");
    println!("✓ Resource routing (full CRUD)");
    println!("✓ API resources (no forms)");
    println!("✓ Resource filtering (only/except)");
    println!("✓ Shallow nesting");
    println!("✓ Middleware groups");
    println!("✓ Controller action routing");
    println!("✓ Route naming");
    println!("\n=== Example Complete ===");
}
