//! Route builder for the facade pattern.
//!
//! This module provides a fluent builder API for configuring routes
//! with middleware, names, and other options.

use rf_routing::{Route as RfRoute, HttpMethod};
use crate::registry::global_router;

/// Builder for configuring a route with a fluent API.
///
/// This builder allows chaining methods to configure a route before
/// it's registered with the global router.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_route_facade::FacadeRouteBuilder;
/// use rf_routing::HttpMethod;
///
/// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
///     .name("users.index")
///     .middleware("auth")
///     .middleware("throttle");
/// ```
pub struct FacadeRouteBuilder {
    pub(crate) route: RfRoute,
}

impl FacadeRouteBuilder {
    /// Create a new route builder.
    pub fn new(path: impl Into<String>, methods: Vec<HttpMethod>) -> Self {
        Self {
            route: RfRoute::new(path.into(), methods),
        }
    }

    /// Set the route name.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_route_facade::FacadeRouteBuilder;
    /// use rf_routing::HttpMethod;
    ///
    /// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
    ///     .name("users.index");
    /// ```
    pub fn name(mut self, name: impl Into<String>) -> Self {
        let mut route = std::mem::replace(&mut self.route, RfRoute::new("", vec![]));
        route = route.name(name);
        self.route = route;
        self
    }

    /// Add a middleware to the route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_route_facade::FacadeRouteBuilder;
    /// use rf_routing::HttpMethod;
    ///
    /// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
    ///     .middleware("auth")
    ///     .middleware("throttle");
    /// ```
    pub fn middleware(mut self, middleware: impl Into<String>) -> Self {
        let mut route = std::mem::replace(&mut self.route, RfRoute::new("", vec![]));
        route = route.add_middleware(middleware);
        self.route = route;
        self
    }

    /// Add multiple middleware to the route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_route_facade::FacadeRouteBuilder;
    /// use rf_routing::HttpMethod;
    ///
    /// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
    ///     .with_middleware(vec!["auth", "throttle"]);
    /// ```
    pub fn with_middleware(mut self, middleware: Vec<&str>) -> Self {
        let mut route = std::mem::replace(&mut self.route, RfRoute::new("", vec![]));
        route = route.middleware(middleware);
        self.route = route;
        self
    }

    /// Add the route to a group.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_route_facade::FacadeRouteBuilder;
    /// use rf_routing::HttpMethod;
    ///
    /// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
    ///     .group("api");
    /// ```
    pub fn group(mut self, group: impl Into<String>) -> Self {
        let mut route = std::mem::replace(&mut self.route, RfRoute::new("", vec![]));
        route = route.add_group(group);
        self.route = route;
        self
    }

    /// Add metadata to the route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_route_facade::FacadeRouteBuilder;
    /// use rf_routing::HttpMethod;
    ///
    /// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
    ///     .metadata("description", "List all users");
    /// ```
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let mut route = std::mem::replace(&mut self.route, RfRoute::new("", vec![]));
        route = route.with_metadata(key, value);
        self.route = route;
        self
    }

    /// Set the domain constraint for the route.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_route_facade::FacadeRouteBuilder;
    /// use rf_routing::HttpMethod;
    ///
    /// let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
    ///     .domain("api.example.com");
    /// ```
    pub fn domain(self, domain: impl Into<String>) -> Self {
        self.metadata("domain", domain)
    }

    /// Build and register the route with the global router.
    ///
    /// This method consumes the builder and registers the route.
    pub fn build(mut self) {
        let route = std::mem::replace(&mut self.route, RfRoute::new("", vec![]));
        global_router().register_route(route);
    }

    /// Get the internal route (for testing).
    #[doc(hidden)]
    pub fn into_route(mut self) -> RfRoute {
        std::mem::replace(&mut self.route, RfRoute::new("", vec![]))
    }
}

// Implement Drop to auto-register routes
impl Drop for FacadeRouteBuilder {
    fn drop(&mut self) {
        // Only register if the route hasn't been consumed yet
        // Check if route has a valid URI (not the dummy "" we use in mem::replace)
        if !self.route.uri.is_empty() {
            global_router().register_route(self.route.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_route_builder_new() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get]);
        let route = builder.into_route();

        assert_eq!(route.uri, "/users");
        assert_eq!(route.methods.len(), 1);
        assert_eq!(route.methods[0], HttpMethod::Get);
    }

    #[test]
    fn test_route_builder_name() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
            .name("users.index");

        let route = builder.into_route();
        assert_eq!(route.name, Some("users.index".to_string()));
    }

    #[test]
    fn test_route_builder_middleware() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
            .middleware("auth")
            .middleware("throttle");

        let route = builder.into_route();
        assert_eq!(route.middleware.len(), 2);
        assert!(route.has_middleware("auth"));
        assert!(route.has_middleware("throttle"));
    }

    #[test]
    fn test_route_builder_with_middleware() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
            .with_middleware(vec!["auth", "throttle", "cors"]);

        let route = builder.into_route();
        assert_eq!(route.middleware.len(), 3);
    }

    #[test]
    fn test_route_builder_group() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
            .group("api")
            .group("v1");

        let route = builder.into_route();
        assert_eq!(route.groups.len(), 2);
        assert!(route.in_group("api"));
        assert!(route.in_group("v1"));
    }

    #[test]
    fn test_route_builder_metadata() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
            .metadata("description", "List all users")
            .metadata("version", "1.0");

        let route = builder.into_route();
        assert_eq!(
            route.metadata("description"),
            Some(&"List all users".to_string())
        );
        assert_eq!(route.metadata("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_route_builder_domain() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Get])
            .domain("api.example.com");

        let route = builder.into_route();
        assert_eq!(
            route.metadata("domain"),
            Some(&"api.example.com".to_string())
        );
    }

    #[test]
    fn test_route_builder_chaining() {
        let builder = FacadeRouteBuilder::new("/users", vec![HttpMethod::Post])
            .name("users.store")
            .middleware("auth")
            .middleware("validate")
            .group("api")
            .metadata("rate_limit", "100")
            .domain("api.example.com");

        let route = builder.into_route();

        assert_eq!(route.name, Some("users.store".to_string()));
        assert_eq!(route.middleware.len(), 2);
        assert_eq!(route.groups.len(), 1);
        assert_eq!(route.metadata.len(), 2);
    }
}
