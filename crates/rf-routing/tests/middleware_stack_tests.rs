//! Comprehensive tests for middleware stack

use rf_routing::middleware_stack::{MiddlewareStack, MiddlewareStackBuilder};

#[test]
fn test_middleware_stack_creation() {
    let stack = MiddlewareStack::new();
    assert_eq!(stack.global().len(), 0);
    assert_eq!(stack.groups().len(), 0);
}

#[test]
fn test_add_global_middleware() {
    let stack = MiddlewareStack::new();

    stack.add_global("cors");
    stack.add_global("logging");
    stack.add_global("compression");

    let global = stack.global();
    assert_eq!(global.len(), 3);
    assert_eq!(global[0], "cors");
    assert_eq!(global[1], "logging");
    assert_eq!(global[2], "compression");
}

#[test]
fn test_add_global_middleware_batch() {
    let stack = MiddlewareStack::new();

    stack.add_global_middleware(vec![
        "cors".to_string(),
        "logging".to_string(),
        "compression".to_string(),
    ]);

    let global = stack.global();
    assert_eq!(global.len(), 3);
}

#[test]
fn test_add_middleware_group() {
    let stack = MiddlewareStack::new();

    stack.add_group("api", vec![
        "auth".to_string(),
        "throttle".to_string(),
        "json".to_string(),
    ]);

    let group = stack.group("api");
    assert!(group.is_some());

    let middleware = group.unwrap();
    assert_eq!(middleware.len(), 3);
    assert_eq!(middleware[0], "auth");
    assert_eq!(middleware[1], "throttle");
    assert_eq!(middleware[2], "json");
}

#[test]
fn test_append_to_group() {
    let stack = MiddlewareStack::new();

    stack.add_group("api", vec!["auth".to_string()]);
    stack.append_to_group("api", "throttle");
    stack.append_to_group("api", "cors");

    let group = stack.group("api").unwrap();
    assert_eq!(group.len(), 3);
    assert_eq!(group[2], "cors");
}

#[test]
fn test_append_to_nonexistent_group() {
    let stack = MiddlewareStack::new();

    stack.append_to_group("new_group", "middleware1");
    stack.append_to_group("new_group", "middleware2");

    let group = stack.group("new_group").unwrap();
    assert_eq!(group.len(), 2);
}

#[test]
fn test_add_route_middleware() {
    let stack = MiddlewareStack::new();

    stack.add_route_middleware("users.create", vec![
        "validate".to_string(),
        "transform".to_string(),
    ]);

    let route_mw = stack.route("users.create");
    assert!(route_mw.is_some());

    let middleware = route_mw.unwrap();
    assert_eq!(middleware.len(), 2);
    assert_eq!(middleware[0], "validate");
}

#[test]
fn test_append_route_middleware() {
    let stack = MiddlewareStack::new();

    stack.add_route_middleware("test.route", vec!["mw1".to_string()]);
    stack.append_route_middleware("test.route", "mw2");
    stack.append_route_middleware("test.route", "mw3");

    let route_mw = stack.route("test.route").unwrap();
    assert_eq!(route_mw.len(), 3);
}

#[test]
fn test_resolve_middleware_order() {
    let stack = MiddlewareStack::new();

    // Set up middleware layers
    stack.add_global("cors");
    stack.add_global("logging");
    stack.add_group("api", vec!["auth".to_string(), "throttle".to_string()]);
    stack.add_route_middleware("users.create", vec!["validate".to_string()]);

    // Resolve middleware for route in group
    let resolved = stack.resolve("users.create", &vec!["api".to_string()]);

    // Should be in order: global -> group -> route
    assert_eq!(resolved.len(), 5);
    assert_eq!(resolved[0], "cors");
    assert_eq!(resolved[1], "logging");
    assert_eq!(resolved[2], "auth");
    assert_eq!(resolved[3], "throttle");
    assert_eq!(resolved[4], "validate");
}

#[test]
fn test_resolve_with_multiple_groups() {
    let stack = MiddlewareStack::new();

    stack.add_global("cors");
    stack.add_group("web", vec!["session".to_string(), "csrf".to_string()]);
    stack.add_group("admin", vec!["auth".to_string(), "admin".to_string()]);
    stack.add_route_middleware("admin.users", vec!["validate".to_string()]);

    let resolved = stack.resolve(
        "admin.users",
        &vec!["web".to_string(), "admin".to_string()],
    );

    assert_eq!(resolved.len(), 6);
    assert_eq!(resolved[0], "cors"); // global
    assert_eq!(resolved[1], "session"); // web group
    assert_eq!(resolved[2], "csrf"); // web group
    assert_eq!(resolved[3], "auth"); // admin group
    assert_eq!(resolved[4], "admin"); // admin group
    assert_eq!(resolved[5], "validate"); // route
}

#[test]
fn test_resolve_removes_duplicates() {
    let stack = MiddlewareStack::new();

    stack.add_global("auth");
    stack.add_global("logging");
    stack.add_group("api", vec!["auth".to_string(), "throttle".to_string()]);
    stack.add_route_middleware("test", vec!["auth".to_string()]);

    let resolved = stack.resolve("test", &vec!["api".to_string()]);

    // Should only have one "auth" middleware
    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0], "auth");
    assert_eq!(resolved[1], "logging");
    assert_eq!(resolved[2], "throttle");
}

#[test]
fn test_resolve_preserves_order_while_removing_duplicates() {
    let stack = MiddlewareStack::new();

    stack.add_global("mw1");
    stack.add_global("mw2");
    stack.add_global("mw3");
    stack.add_group("g1", vec!["mw2".to_string(), "mw4".to_string()]);

    let resolved = stack.resolve("test", &vec!["g1".to_string()]);

    // First occurrence should be kept
    assert_eq!(resolved, vec!["mw1", "mw2", "mw3", "mw4"]);
}

#[test]
fn test_resolve_with_no_groups() {
    let stack = MiddlewareStack::new();

    stack.add_global("cors");
    stack.add_route_middleware("test", vec!["validate".to_string()]);

    let resolved = stack.resolve("test", &vec![]);

    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0], "cors");
    assert_eq!(resolved[1], "validate");
}

#[test]
fn test_resolve_with_only_global() {
    let stack = MiddlewareStack::new();

    stack.add_global("cors");
    stack.add_global("logging");

    let resolved = stack.resolve("test", &vec![]);

    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved[0], "cors");
    assert_eq!(resolved[1], "logging");
}

#[test]
fn test_resolve_empty_stack() {
    let stack = MiddlewareStack::new();

    let resolved = stack.resolve("test", &vec![]);

    assert_eq!(resolved.len(), 0);
}

#[test]
fn test_remove_group() {
    let stack = MiddlewareStack::new();

    stack.add_group("api", vec!["auth".to_string()]);
    assert!(stack.group("api").is_some());

    assert!(stack.remove_group("api"));
    assert!(stack.group("api").is_none());

    // Removing non-existent group should return false
    assert!(!stack.remove_group("nonexistent"));
}

#[test]
fn test_remove_route_middleware() {
    let stack = MiddlewareStack::new();

    stack.add_route_middleware("test", vec!["validate".to_string()]);
    assert!(stack.route("test").is_some());

    assert!(stack.remove_route_middleware("test"));
    assert!(stack.route("test").is_none());

    // Removing non-existent route middleware should return false
    assert!(!stack.remove_route_middleware("nonexistent"));
}

#[test]
fn test_clear() {
    let stack = MiddlewareStack::new();

    stack.add_global("cors");
    stack.add_group("api", vec!["auth".to_string()]);
    stack.add_route_middleware("test", vec!["validate".to_string()]);

    assert_eq!(stack.global().len(), 1);
    assert_eq!(stack.groups().len(), 1);
    assert!(stack.route("test").is_some());

    stack.clear();

    assert_eq!(stack.global().len(), 0);
    assert_eq!(stack.groups().len(), 0);
    assert!(stack.route("test").is_none());
}

#[test]
fn test_builder_pattern() {
    let stack = MiddlewareStackBuilder::new()
        .global("cors")
        .global("logging")
        .group("api", vec!["auth".to_string(), "throttle".to_string()])
        .route("users.create", vec!["validate".to_string()])
        .build();

    assert_eq!(stack.global().len(), 2);
    assert!(stack.group("api").is_some());
    assert!(stack.route("users.create").is_some());
}

#[test]
fn test_builder_chaining() {
    let stack = MiddlewareStackBuilder::new()
        .global("mw1")
        .global("mw2")
        .global("mw3")
        .group("g1", vec!["mw4".to_string()])
        .group("g2", vec!["mw5".to_string()])
        .route("r1", vec!["mw6".to_string()])
        .route("r2", vec!["mw7".to_string()])
        .build();

    assert_eq!(stack.global().len(), 3);
    assert_eq!(stack.groups().len(), 2);
}

#[test]
fn test_get_all_groups() {
    let stack = MiddlewareStack::new();

    stack.add_group("api", vec!["auth".to_string()]);
    stack.add_group("web", vec!["session".to_string()]);
    stack.add_group("admin", vec!["admin".to_string()]);

    let groups = stack.groups();
    assert_eq!(groups.len(), 3);
    assert!(groups.contains_key("api"));
    assert!(groups.contains_key("web"));
    assert!(groups.contains_key("admin"));
}

#[test]
fn test_nonexistent_group_in_resolve() {
    let stack = MiddlewareStack::new();

    stack.add_global("cors");

    // Resolving with non-existent group should not panic
    let resolved = stack.resolve("test", &vec!["nonexistent".to_string()]);

    assert_eq!(resolved.len(), 1);
    assert_eq!(resolved[0], "cors");
}

#[test]
fn test_complex_middleware_stack() {
    let stack = MiddlewareStack::new();

    // Set up complex stack
    stack.add_global("cors");
    stack.add_global("logging");
    stack.add_global("compression");

    stack.add_group("web", vec![
        "session".to_string(),
        "csrf".to_string(),
        "cookie".to_string(),
    ]);

    stack.add_group("api", vec![
        "auth".to_string(),
        "throttle".to_string(),
        "json".to_string(),
    ]);

    stack.add_group("admin", vec![
        "admin_auth".to_string(),
        "admin_log".to_string(),
    ]);

    stack.add_route_middleware("admin.users.create", vec![
        "validate_user".to_string(),
        "check_permissions".to_string(),
    ]);

    // Resolve for a complex route
    let resolved = stack.resolve(
        "admin.users.create",
        &vec!["web".to_string(), "api".to_string(), "admin".to_string()],
    );

    // Should have: 3 global + 3 web + 3 api + 2 admin + 2 route = 13
    assert_eq!(resolved.len(), 13);

    // Verify order
    assert_eq!(resolved[0], "cors"); // global
    assert_eq!(resolved[3], "session"); // web
    assert_eq!(resolved[6], "auth"); // api
    assert_eq!(resolved[9], "admin_auth"); // admin
    assert_eq!(resolved[11], "validate_user"); // route
}

#[test]
fn test_stack_clone() {
    let stack1 = MiddlewareStack::new();
    stack1.add_global("cors");

    let stack2 = stack1.clone();
    assert_eq!(stack2.global().len(), 1);

    // Modifications to stack2 should not affect stack1
    stack2.add_global("logging");
    assert_eq!(stack1.global().len(), 2); // Both share the same Arc
}
