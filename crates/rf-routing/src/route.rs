//! Route definition and configuration

use std::collections::HashMap;

/// HTTP method for routes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Options,
    Head,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::Get => write!(f, "GET"),
            HttpMethod::Post => write!(f, "POST"),
            HttpMethod::Put => write!(f, "PUT"),
            HttpMethod::Patch => write!(f, "PATCH"),
            HttpMethod::Delete => write!(f, "DELETE"),
            HttpMethod::Options => write!(f, "OPTIONS"),
            HttpMethod::Head => write!(f, "HEAD"),
        }
    }
}

/// Route definition with metadata
#[derive(Debug, Clone)]
pub struct Route {
    /// Route name (for named routes)
    pub name: Option<String>,
    /// URI pattern (e.g., "/users/{id}")
    pub uri: String,
    /// HTTP methods
    pub methods: Vec<HttpMethod>,
    /// Middleware names to apply
    pub middleware: Vec<String>,
    /// Group names this route belongs to
    pub groups: Vec<String>,
    /// Route metadata
    pub metadata: HashMap<String, String>,
}

impl Route {
    /// Create a new route
    pub fn new(uri: impl Into<String>, methods: Vec<HttpMethod>) -> Self {
        Self {
            name: None,
            uri: uri.into(),
            methods,
            middleware: Vec::new(),
            groups: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Set the route name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add middleware to the route
    pub fn middleware(mut self, middleware: Vec<&str>) -> Self {
        self.middleware.extend(middleware.iter().map(|s| s.to_string()));
        self
    }

    /// Add a single middleware
    pub fn add_middleware(mut self, middleware: impl Into<String>) -> Self {
        self.middleware.push(middleware.into());
        self
    }

    /// Set the groups this route belongs to
    pub fn groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }

    /// Add a group to this route
    pub fn add_group(mut self, group: impl Into<String>) -> Self {
        self.groups.push(group.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get metadata value
    pub fn metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Check if route has a specific middleware
    pub fn has_middleware(&self, middleware: &str) -> bool {
        self.middleware.iter().any(|m| m == middleware)
    }

    /// Check if route belongs to a specific group
    pub fn in_group(&self, group: &str) -> bool {
        self.groups.iter().any(|g| g == group)
    }

    /// Get all middleware including from groups
    pub fn all_middleware(&self, stack: &crate::middleware_stack::MiddlewareStack) -> Vec<String> {
        let mut middleware = stack.resolve(self.name.as_deref().unwrap_or(""), &self.groups);
        middleware.extend(self.middleware.clone());
        middleware
    }
}

/// Builder for creating routes with a fluent API
pub struct RouteBuilder {
    route: Route,
}

impl RouteBuilder {
    /// Create a new route builder
    pub fn new(uri: impl Into<String>, methods: Vec<HttpMethod>) -> Self {
        Self {
            route: Route::new(uri, methods),
        }
    }

    /// Create a GET route
    pub fn get(uri: impl Into<String>) -> Self {
        Self::new(uri, vec![HttpMethod::Get])
    }

    /// Create a POST route
    pub fn post(uri: impl Into<String>) -> Self {
        Self::new(uri, vec![HttpMethod::Post])
    }

    /// Create a PUT route
    pub fn put(uri: impl Into<String>) -> Self {
        Self::new(uri, vec![HttpMethod::Put])
    }

    /// Create a PATCH route
    pub fn patch(uri: impl Into<String>) -> Self {
        Self::new(uri, vec![HttpMethod::Patch])
    }

    /// Create a DELETE route
    pub fn delete(uri: impl Into<String>) -> Self {
        Self::new(uri, vec![HttpMethod::Delete])
    }

    /// Set the route name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.route = self.route.name(name);
        self
    }

    /// Add middleware
    pub fn middleware(mut self, middleware: Vec<&str>) -> Self {
        self.route = self.route.middleware(middleware);
        self
    }

    /// Add a single middleware
    pub fn add_middleware(mut self, middleware: impl Into<String>) -> Self {
        self.route = self.route.add_middleware(middleware);
        self
    }

    /// Set groups
    pub fn groups(mut self, groups: Vec<String>) -> Self {
        self.route = self.route.groups(groups);
        self
    }

    /// Add metadata
    pub fn metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.route = self.route.with_metadata(key, value);
        self
    }

    /// Build the route
    pub fn build(self) -> Route {
        self.route
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::middleware_stack::MiddlewareStack;

    #[test]
    fn test_route_creation() {
        let route = Route::new("/users", vec![HttpMethod::Get]);
        assert_eq!(route.uri, "/users");
        assert_eq!(route.methods.len(), 1);
        assert_eq!(route.methods[0], HttpMethod::Get);
    }

    #[test]
    fn test_route_with_name() {
        let route = Route::new("/users", vec![HttpMethod::Get]).name("users.index");
        assert_eq!(route.name, Some("users.index".to_string()));
    }

    #[test]
    fn test_route_with_middleware() {
        let route = Route::new("/users", vec![HttpMethod::Get])
            .middleware(vec!["auth", "throttle"]);

        assert_eq!(route.middleware.len(), 2);
        assert!(route.has_middleware("auth"));
        assert!(route.has_middleware("throttle"));
    }

    #[test]
    fn test_route_with_groups() {
        let route = Route::new("/users", vec![HttpMethod::Get])
            .add_group("api")
            .add_group("v1");

        assert_eq!(route.groups.len(), 2);
        assert!(route.in_group("api"));
        assert!(route.in_group("v1"));
    }

    #[test]
    fn test_route_with_metadata() {
        let route = Route::new("/users", vec![HttpMethod::Get])
            .with_metadata("description", "List all users")
            .with_metadata("version", "1.0");

        assert_eq!(route.metadata("description"), Some(&"List all users".to_string()));
        assert_eq!(route.metadata("version"), Some(&"1.0".to_string()));
    }

    #[test]
    fn test_route_builder_get() {
        let route = RouteBuilder::get("/users")
            .name("users.index")
            .add_middleware("auth")
            .build();

        assert_eq!(route.uri, "/users");
        assert_eq!(route.methods[0], HttpMethod::Get);
        assert_eq!(route.name, Some("users.index".to_string()));
        assert!(route.has_middleware("auth"));
    }

    #[test]
    fn test_route_builder_post() {
        let route = RouteBuilder::post("/users")
            .name("users.store")
            .middleware(vec!["auth", "validate"])
            .build();

        assert_eq!(route.methods[0], HttpMethod::Post);
        assert_eq!(route.middleware.len(), 2);
    }

    #[test]
    fn test_route_all_middleware() {
        let stack = MiddlewareStack::new();
        stack.add_global("cors");
        stack.add_group("api", vec!["auth".to_string()]);

        let route = Route::new("/users", vec![HttpMethod::Get])
            .name("users.index")
            .add_group("api")
            .add_middleware("throttle");

        let all_mw = route.all_middleware(&stack);
        assert_eq!(all_mw.len(), 3);
        assert!(all_mw.contains(&"cors".to_string()));
        assert!(all_mw.contains(&"auth".to_string()));
        assert!(all_mw.contains(&"throttle".to_string()));
    }

    #[test]
    fn test_http_method_display() {
        assert_eq!(format!("{}", HttpMethod::Get), "GET");
        assert_eq!(format!("{}", HttpMethod::Post), "POST");
        assert_eq!(format!("{}", HttpMethod::Put), "PUT");
        assert_eq!(format!("{}", HttpMethod::Patch), "PATCH");
        assert_eq!(format!("{}", HttpMethod::Delete), "DELETE");
    }
}
