//! Integration tests for the Route facade.

use rf_route_facade::{Route, global_router};
use rf_routing::HttpMethod;

#[test]
fn test_route_registration() {
    global_router().clear();

    Route::get("/users", "handler".to_string())
        .name("users.index")
        .middleware("auth");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].uri, "/users");
    assert_eq!(routes[0].name, Some("users.index".to_string()));
    assert!(routes[0].has_middleware("auth"));
}

#[test]
fn test_multiple_routes() {
    global_router().clear();

    Route::get("/users", "handler".to_string()).name("users.index");
    Route::post("/users", "handler".to_string()).name("users.store");
    Route::get("/posts", "handler".to_string()).name("posts.index");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 3);

    let names: Vec<String> = routes
        .iter()
        .filter_map(|r| r.name.clone())
        .collect();

    assert!(names.contains(&"users.index".to_string()));
    assert!(names.contains(&"users.store".to_string()));
    assert!(names.contains(&"posts.index".to_string()));
}

#[test]
fn test_route_with_multiple_middleware() {
    global_router().clear();

    Route::get("/admin", "handler".to_string())
        .middleware("auth")
        .middleware("admin")
        .middleware("throttle");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].middleware.len(), 3);
}

#[test]
fn test_route_groups() {
    global_router().clear();

    Route::group()
        .prefix("/api")
        .middleware("auth")
        .routes(|group| {
            group.get("/users", "handler".to_string()).name("users");
            group.post("/posts", "handler".to_string()).name("posts");
        });

    let routes = global_router().routes();
    assert_eq!(routes.len(), 2);

    // Check prefixes are applied
    let uris: Vec<String> = routes.iter().map(|r| r.uri.clone()).collect();
    assert!(uris.contains(&"/api/users".to_string()));
    assert!(uris.contains(&"/api/posts".to_string()));

    // Check middleware is applied
    for route in &routes {
        assert!(route.has_middleware("auth"));
    }
}

#[test]
fn test_nested_groups() {
    global_router().clear();

    Route::group()
        .prefix("/api")
        .middleware("auth")
        .routes(|group| {
            group
                .group()
                .prefix("/v1")
                .middleware("throttle")
                .routes(|nested| {
                    nested.get("/users", "handler".to_string());
                });
        });

    let routes = global_router().routes();
    assert!(!routes.is_empty());

    // Should have both prefixes
    let uri = &routes[0].uri;
    assert!(uri.contains("/api"));
    assert!(uri.contains("/v1"));
}

#[test]
fn test_resource_routes() {
    global_router().clear();

    Route::resource("posts", "PostController");

    let routes = global_router().routes();

    // Should have all RESTful routes
    let names: Vec<String> = routes
        .iter()
        .filter_map(|r| r.name.clone())
        .collect();

    assert!(names.contains(&"posts.index".to_string()));
    assert!(names.contains(&"posts.create".to_string()));
    assert!(names.contains(&"posts.store".to_string()));
    assert!(names.contains(&"posts.show".to_string()));
    assert!(names.contains(&"posts.edit".to_string()));
    assert!(names.contains(&"posts.update".to_string()));
    assert!(names.contains(&"posts.destroy".to_string()));
}

#[test]
fn test_api_resource_routes() {
    global_router().clear();

    Route::api_resource("users", "UserController");

    let routes = global_router().routes();
    let names: Vec<String> = routes
        .iter()
        .filter_map(|r| r.name.clone())
        .collect();

    // Should have API routes
    assert!(names.contains(&"users.index".to_string()));
    assert!(names.contains(&"users.store".to_string()));
    assert!(names.contains(&"users.show".to_string()));
    assert!(names.contains(&"users.update".to_string()));
    assert!(names.contains(&"users.destroy".to_string()));

    // Should NOT have HTML form routes
    assert!(!names.contains(&"users.create".to_string()));
    assert!(!names.contains(&"users.edit".to_string()));
}

#[test]
fn test_redirect_route() {
    global_router().clear();

    Route::redirect("/old-path", "/new-path");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(
        routes[0].metadata("redirect"),
        Some(&"/new-path".to_string())
    );
}

#[test]
fn test_permanent_redirect_route() {
    global_router().clear();

    Route::permanent_redirect("/old", "/new");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].metadata("redirect"), Some(&"/new".to_string()));
    assert_eq!(routes[0].metadata("status"), Some(&"301".to_string()));
}

#[test]
fn test_view_route() {
    global_router().clear();

    Route::view("/about", "about");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].metadata("view"), Some(&"about".to_string()));
}

#[test]
fn test_match_methods_route() {
    global_router().clear();

    Route::match_methods(
        vec![HttpMethod::Get, HttpMethod::Post],
        "/form",
        "handler".to_string(),
    );

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert_eq!(routes[0].methods.len(), 2);
    assert!(routes[0].methods.contains(&HttpMethod::Get));
    assert!(routes[0].methods.contains(&HttpMethod::Post));
}

#[test]
fn test_any_method_route() {
    global_router().clear();

    Route::any("/fallback", "handler".to_string());

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);
    assert!(routes[0].methods.len() >= 5); // At least GET, POST, PUT, PATCH, DELETE
}

#[test]
fn test_url_generation() {
    global_router().clear();

    Route::get("/users/{id}", "handler".to_string()).name("users.show");

    let mut params = std::collections::HashMap::new();
    params.insert("id".to_string(), "123".to_string());

    let url = global_router().url("users.show", &params);
    assert_eq!(url, Some("/users/123".to_string()));
}

#[test]
fn test_get_route_by_name() {
    global_router().clear();

    Route::get("/users", "handler".to_string())
        .name("users.index")
        .middleware("auth");

    let route = global_router().get_route("users.index");
    assert!(route.is_some());

    let route = route.unwrap();
    assert_eq!(route.uri, "/users");
    assert!(route.has_middleware("auth"));
}

#[test]
fn test_route_chaining() {
    global_router().clear();

    Route::post("/api/posts", "handler".to_string())
        .name("api.posts.store")
        .middleware("auth")
        .middleware("validate")
        .group("api")
        .metadata("rate_limit", "100");

    let routes = global_router().routes();
    assert_eq!(routes.len(), 1);

    let route = &routes[0];
    assert_eq!(route.name, Some("api.posts.store".to_string()));
    assert_eq!(route.middleware.len(), 2);
    assert!(route.in_group("api"));
    assert_eq!(route.metadata("rate_limit"), Some(&"100".to_string()));
}
