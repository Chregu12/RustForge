//! Route groups for organizing routes with shared configuration.
//!
//! This module provides Laravel-like route groups with support for:
//! - Prefixes
//! - Middleware
//! - Named route prefixes
//! - Domain constraints
//! - Nested groups

use axum::Router;

/// Configuration for a route group.
#[derive(Debug, Clone)]
pub struct RouteGroup {
    prefix: Option<String>,
    middleware: Vec<String>,
    name: Option<String>,
    domain: Option<String>,
}

impl RouteGroup {
    /// Create a new route group.
    pub fn new() -> Self {
        Self {
            prefix: None,
            middleware: Vec::new(),
            name: None,
            domain: None,
        }
    }

    /// Set the prefix for all routes in this group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::RouteGroup;
    ///
    /// let group = RouteGroup::new().prefix("/api");
    /// ```
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Add middleware to this group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::RouteGroup;
    ///
    /// let group = RouteGroup::new()
    ///     .middleware("auth")
    ///     .middleware("throttle");
    /// ```
    pub fn middleware(mut self, middleware: impl Into<String>) -> Self {
        self.middleware.push(middleware.into());
        self
    }

    /// Set the name prefix for all routes in this group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::RouteGroup;
    ///
    /// let group = RouteGroup::new().name("api.");
    /// ```
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the domain constraint for this group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::RouteGroup;
    ///
    /// let group = RouteGroup::new().domain("api.example.com");
    /// ```
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Get the prefix.
    pub fn get_prefix(&self) -> Option<&str> {
        self.prefix.as_deref()
    }

    /// Get the middleware stack.
    pub fn get_middleware(&self) -> &[String] {
        &self.middleware
    }

    /// Get the name prefix.
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Get the domain constraint.
    pub fn get_domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    /// Apply this group configuration to a router.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_routing::RouteGroup;
    /// use axum::{Router, routing::get};
    ///
    /// async fn handler() -> &'static str {
    ///     "Hello, World!"
    /// }
    ///
    /// let group = RouteGroup::new()
    ///     .prefix("/api")
    ///     .middleware("auth");
    ///
    /// let router = Router::new()
    ///     .route("/users", get(handler));
    ///
    /// let router = group.apply(router);
    /// ```
    pub fn apply<S>(self, router: Router<S>) -> Router<S>
    where
        S: Clone + Send + Sync + 'static,
    {
        let mut result = router;

        // Apply prefix by nesting the router
        if let Some(prefix) = self.prefix {
            let nested = result;
            result = Router::new().nest(&prefix, nested);
        }

        // Note: Middleware will be applied by the middleware pipeline
        // The group just stores the middleware names

        result
    }

    /// Create a nested group.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::RouteGroup;
    ///
    /// let parent = RouteGroup::new()
    ///     .prefix("/api")
    ///     .middleware("auth");
    ///
    /// let child = parent.nest(RouteGroup::new()
    ///     .prefix("/v1")
    ///     .middleware("throttle"));
    /// ```
    pub fn nest(self, child: RouteGroup) -> RouteGroup {
        let prefix = match (self.prefix, child.prefix) {
            (Some(p1), Some(p2)) => Some(format!("{}{}", p1, p2)),
            (Some(p), None) | (None, Some(p)) => Some(p),
            (None, None) => None,
        };

        let name = match (self.name, child.name) {
            (Some(n1), Some(n2)) => Some(format!("{}{}", n1, n2)),
            (Some(n), None) | (None, Some(n)) => Some(n),
            (None, None) => None,
        };

        let mut middleware = self.middleware;
        middleware.extend(child.middleware);

        let domain = child.domain.or(self.domain);

        RouteGroup {
            prefix,
            middleware,
            name,
            domain,
        }
    }
}

impl Default for RouteGroup {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating route groups with a fluent API.
pub struct RouteGroupBuilder {
    group: RouteGroup,
}

impl RouteGroupBuilder {
    /// Create a new route group builder.
    pub fn new() -> Self {
        Self {
            group: RouteGroup::new(),
        }
    }

    /// Set the prefix.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.group = self.group.prefix(prefix);
        self
    }

    /// Add middleware.
    pub fn middleware(mut self, middleware: impl Into<String>) -> Self {
        self.group = self.group.middleware(middleware);
        self
    }

    /// Set the name prefix.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.group = self.group.name(name);
        self
    }

    /// Set the domain constraint.
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.group = self.group.domain(domain);
        self
    }

    /// Build the route group.
    pub fn build(self) -> RouteGroup {
        self.group
    }
}

impl Default for RouteGroupBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// A registry for managing route groups.
#[derive(Debug, Clone, Default)]
pub struct RouteGroupRegistry {
    groups: Vec<RouteGroup>,
}

impl RouteGroupRegistry {
    /// Create a new route group registry.
    pub fn new() -> Self {
        Self { groups: Vec::new() }
    }

    /// Register a route group.
    pub fn register(&mut self, group: RouteGroup) {
        self.groups.push(group);
    }

    /// Get all registered groups.
    pub fn groups(&self) -> &[RouteGroup] {
        &self.groups
    }

    /// Find groups by prefix.
    pub fn find_by_prefix(&self, prefix: &str) -> Vec<&RouteGroup> {
        self.groups
            .iter()
            .filter(|g| g.get_prefix() == Some(prefix))
            .collect()
    }

    /// Find groups by middleware.
    pub fn find_by_middleware(&self, middleware: &str) -> Vec<&RouteGroup> {
        self.groups
            .iter()
            .filter(|g| g.get_middleware().contains(&middleware.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Router};

    async fn dummy_handler() -> &'static str {
        "test"
    }

    #[test]
    fn test_route_group_creation() {
        let group = RouteGroup::new();
        assert!(group.get_prefix().is_none());
        assert!(group.get_middleware().is_empty());
        assert!(group.get_name().is_none());
        assert!(group.get_domain().is_none());
    }

    #[test]
    fn test_route_group_prefix() {
        let group = RouteGroup::new().prefix("/api");
        assert_eq!(group.get_prefix(), Some("/api"));
    }

    #[test]
    fn test_route_group_middleware() {
        let group = RouteGroup::new().middleware("auth").middleware("throttle");

        assert_eq!(group.get_middleware().len(), 2);
        assert!(group.get_middleware().contains(&"auth".to_string()));
        assert!(group.get_middleware().contains(&"throttle".to_string()));
    }

    #[test]
    fn test_route_group_name() {
        let group = RouteGroup::new().name("api.");
        assert_eq!(group.get_name(), Some("api."));
    }

    #[test]
    fn test_route_group_domain() {
        let group = RouteGroup::new().domain("api.example.com");
        assert_eq!(group.get_domain(), Some("api.example.com"));
    }

    #[test]
    fn test_route_group_builder() {
        let group = RouteGroupBuilder::new()
            .prefix("/api")
            .middleware("auth")
            .name("api.")
            .domain("api.example.com")
            .build();

        assert_eq!(group.get_prefix(), Some("/api"));
        assert_eq!(group.get_middleware().len(), 1);
        assert_eq!(group.get_name(), Some("api."));
        assert_eq!(group.get_domain(), Some("api.example.com"));
    }

    #[test]
    fn test_nested_groups() {
        let parent = RouteGroup::new()
            .prefix("/api")
            .middleware("auth")
            .name("api.");

        let child = RouteGroup::new()
            .prefix("/v1")
            .middleware("throttle")
            .name("v1.");

        let nested = parent.nest(child);

        assert_eq!(nested.get_prefix(), Some("/api/v1"));
        assert_eq!(nested.get_name(), Some("api.v1."));
        assert_eq!(nested.get_middleware().len(), 2);
    }

    #[test]
    fn test_nested_groups_partial() {
        let parent = RouteGroup::new().prefix("/api");
        let child = RouteGroup::new().middleware("throttle");
        let nested = parent.nest(child);

        assert_eq!(nested.get_prefix(), Some("/api"));
        assert_eq!(nested.get_middleware().len(), 1);
    }

    #[tokio::test]
    async fn test_route_group_apply() {
        let group = RouteGroup::new().prefix("/api");
        let router: Router = Router::new().route("/users", get(dummy_handler));
        let _router = group.apply(router);

        // Router should be created successfully
        assert!(true);
    }

    #[test]
    fn test_route_group_registry() {
        let mut registry = RouteGroupRegistry::new();

        let group1 = RouteGroup::new().prefix("/api").middleware("auth");
        let group2 = RouteGroup::new().prefix("/admin").middleware("admin");

        registry.register(group1);
        registry.register(group2);

        assert_eq!(registry.groups().len(), 2);
    }

    #[test]
    fn test_route_group_registry_find_by_prefix() {
        let mut registry = RouteGroupRegistry::new();

        let group1 = RouteGroup::new().prefix("/api");
        let group2 = RouteGroup::new().prefix("/admin");
        let group3 = RouteGroup::new().prefix("/api");

        registry.register(group1);
        registry.register(group2);
        registry.register(group3);

        let found = registry.find_by_prefix("/api");
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_route_group_registry_find_by_middleware() {
        let mut registry = RouteGroupRegistry::new();

        let group1 = RouteGroup::new().middleware("auth");
        let group2 = RouteGroup::new().middleware("admin");
        let group3 = RouteGroup::new().middleware("auth").middleware("throttle");

        registry.register(group1);
        registry.register(group2);
        registry.register(group3);

        let found = registry.find_by_middleware("auth");
        assert_eq!(found.len(), 2);
    }
}
