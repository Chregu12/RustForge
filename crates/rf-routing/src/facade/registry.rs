//! Global router registry for managing routes across the application.
//!
//! This module provides a thread-safe global router that can be accessed
//! from anywhere in the application.

use axum::Router;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use crate::{Route as RfRoute, RouteRegistry, NamedRoute};
use std::collections::HashMap;

/// Information about a registered route
#[derive(Debug, Clone)]
pub struct RouteInfo {
    /// The route definition
    pub route: RfRoute,
    /// Whether this route has been registered with Axum
    pub registered: bool,
}

/// Global router registry that maintains all routes in the application.
#[derive(Debug)]
pub struct GlobalRouter {
    /// All registered routes
    routes: RwLock<Vec<RouteInfo>>,
    /// Named route registry
    named_routes: RwLock<RouteRegistry>,
    /// Route groups
    groups: RwLock<Vec<String>>,
    /// Middleware registry
    middleware: RwLock<HashMap<String, Vec<String>>>,
}

impl GlobalRouter {
    /// Create a new global router.
    pub fn new() -> Self {
        Self {
            routes: RwLock::new(Vec::new()),
            named_routes: RwLock::new(RouteRegistry::new()),
            groups: RwLock::new(Vec::new()),
            middleware: RwLock::new(HashMap::new()),
        }
    }

    /// Register a new route.
    pub fn register_route(&self, route: RfRoute) {
        let mut routes = self.routes.write();

        // Register named route if it has a name
        if let Some(name) = &route.name {
            let named_route = NamedRoute::new(name, &route.uri);
            self.named_routes.write().register(named_route);
        }

        routes.push(RouteInfo {
            route,
            registered: false,
        });
    }

    /// Get all registered routes.
    pub fn routes(&self) -> Vec<RfRoute> {
        self.routes
            .read()
            .iter()
            .map(|info| info.route.clone())
            .collect()
    }

    /// Get a route by name.
    pub fn get_route(&self, name: &str) -> Option<RfRoute> {
        self.routes
            .read()
            .iter()
            .find(|info| info.route.name.as_deref() == Some(name))
            .map(|info| info.route.clone())
    }

    /// Generate a URL for a named route.
    pub fn url(&self, name: &str, params: &HashMap<String, String>) -> Option<String> {
        let params_converted: HashMap<String, crate::ParamValue> = params
            .iter()
            .map(|(k, v)| (k.clone(), crate::ParamValue::String(v.clone())))
            .collect();

        self.named_routes.read().url(name, &params_converted)
    }

    /// Register a route group.
    pub fn register_group(&self, group: String) {
        let mut groups = self.groups.write();
        if !groups.contains(&group) {
            groups.push(group);
        }
    }

    /// Register middleware for a group.
    pub fn register_middleware(&self, group: String, middleware: Vec<String>) {
        self.middleware.write().insert(group, middleware);
    }

    /// Get middleware for a group.
    pub fn get_middleware(&self, group: &str) -> Vec<String> {
        self.middleware
            .read()
            .get(group)
            .cloned()
            .unwrap_or_default()
    }

    /// Clear all routes (useful for testing).
    pub fn clear(&self) {
        self.routes.write().clear();
        *self.named_routes.write() = RouteRegistry::new();
        self.groups.write().clear();
        self.middleware.write().clear();
    }

    /// Build an Axum router from all registered routes.
    ///
    /// Note: This is a placeholder that returns an empty router.
    /// In a real implementation, you would convert RouteInfo into actual Axum routes.
    pub fn build_router<S>(&self) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        // This would need actual handler storage and conversion
        // For now, return an empty router
        Router::new()
    }
}

impl Default for GlobalRouter {
    fn default() -> Self {
        Self::new()
    }
}

/// Global router instance accessible from anywhere in the application.
pub static GLOBAL_ROUTER: Lazy<GlobalRouter> = Lazy::new(GlobalRouter::new);

/// Get a reference to the global router.
///
/// # Examples
///
/// ```rust
/// use rf_routing::global_router;
///
/// let router = global_router();
/// let routes = router.routes();
/// ```
pub fn global_router() -> &'static GlobalRouter {
    &GLOBAL_ROUTER
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HttpMethod;

    #[test]
    fn test_global_router_register() {
        let router = GlobalRouter::new();

        let route = RfRoute::new("/test", vec![HttpMethod::Get])
            .name("test.route");

        router.register_route(route.clone());

        let routes = router.routes();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].uri, "/test");
    }

    #[test]
    fn test_global_router_get_route() {
        let router = GlobalRouter::new();

        let route = RfRoute::new("/users", vec![HttpMethod::Get])
            .name("users.index");

        router.register_route(route);

        let found = router.get_route("users.index");
        assert!(found.is_some());
        assert_eq!(found.unwrap().uri, "/users");
    }

    #[test]
    fn test_global_router_url_generation() {
        let router = GlobalRouter::new();

        let route = RfRoute::new("/users/{id}", vec![HttpMethod::Get])
            .name("users.show");

        router.register_route(route);

        let mut params = HashMap::new();
        params.insert("id".to_string(), "123".to_string());

        let url = router.url("users.show", &params);
        assert_eq!(url, Some("/users/123".to_string()));
    }

    #[test]
    fn test_global_router_groups() {
        let router = GlobalRouter::new();

        router.register_group("api".to_string());
        router.register_group("admin".to_string());

        let groups = router.groups.read();
        assert_eq!(groups.len(), 2);
        assert!(groups.contains(&"api".to_string()));
        assert!(groups.contains(&"admin".to_string()));
    }

    #[test]
    fn test_global_router_middleware() {
        let router = GlobalRouter::new();

        router.register_middleware(
            "api".to_string(),
            vec!["auth".to_string(), "throttle".to_string()],
        );

        let middleware = router.get_middleware("api");
        assert_eq!(middleware.len(), 2);
        assert_eq!(middleware[0], "auth");
        assert_eq!(middleware[1], "throttle");
    }

    #[test]
    fn test_global_router_clear() {
        let router = GlobalRouter::new();

        let route = RfRoute::new("/test", vec![HttpMethod::Get]);
        router.register_route(route);
        router.register_group("api".to_string());

        router.clear();

        assert_eq!(router.routes().len(), 0);
        assert_eq!(router.groups.read().len(), 0);
    }

    #[test]
    fn test_global_router_singleton() {
        let router1 = global_router();
        let router2 = global_router();

        // Both should point to the same instance
        assert!(std::ptr::eq(router1, router2));
    }
}
