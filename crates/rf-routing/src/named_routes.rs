//! Named routes for URL generation.

use std::collections::HashMap;

/// Route parameter value.
#[derive(Debug, Clone)]
pub enum ParamValue {
    String(String),
    Number(i64),
}

impl From<&str> for ParamValue {
    fn from(s: &str) -> Self {
        ParamValue::String(s.to_string())
    }
}

impl From<String> for ParamValue {
    fn from(s: String) -> Self {
        ParamValue::String(s)
    }
}

impl From<i64> for ParamValue {
    fn from(n: i64) -> Self {
        ParamValue::Number(n)
    }
}

impl From<i32> for ParamValue {
    fn from(n: i32) -> Self {
        ParamValue::Number(n as i64)
    }
}

impl ParamValue {
    /// Convert to string representation.
    pub fn as_str(&self) -> String {
        match self {
            ParamValue::String(s) => s.clone(),
            ParamValue::Number(n) => n.to_string(),
        }
    }
}

/// A named route definition.
#[derive(Debug, Clone)]
pub struct NamedRoute {
    name: String,
    pattern: String,
}

impl NamedRoute {
    /// Create a new named route.
    pub fn new(name: impl Into<String>, pattern: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            pattern: pattern.into(),
        }
    }

    /// Get the route name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the route pattern.
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    /// Generate URL with parameters.
    pub fn url(&self, params: &HashMap<String, ParamValue>) -> String {
        let mut url = self.pattern.clone();

        for (key, value) in params {
            let placeholder = format!("{{{}}}", key);
            url = url.replace(&placeholder, &value.as_str());
        }

        url
    }
}

/// Route registry for managing named routes.
#[derive(Debug, Clone, Default)]
pub struct RouteRegistry {
    routes: HashMap<String, NamedRoute>,
}

impl RouteRegistry {
    /// Create a new route registry.
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// Register a named route.
    pub fn register(&mut self, route: NamedRoute) {
        self.routes.insert(route.name().to_string(), route);
    }

    /// Get a route by name.
    pub fn get(&self, name: &str) -> Option<&NamedRoute> {
        self.routes.get(name)
    }

    /// Generate URL for a named route.
    pub fn url(&self, name: &str, params: &HashMap<String, ParamValue>) -> Option<String> {
        self.routes.get(name).map(|route| route.url(params))
    }

    /// Check if a route exists.
    pub fn has(&self, name: &str) -> bool {
        self.routes.contains_key(name)
    }

    /// Get all route names.
    pub fn names(&self) -> Vec<String> {
        self.routes.keys().cloned().collect()
    }
}

/// Builder for creating route URLs.
pub struct RouteUrlBuilder {
    route: NamedRoute,
    params: HashMap<String, ParamValue>,
}

impl RouteUrlBuilder {
    /// Create a new route URL builder.
    pub fn new(route: NamedRoute) -> Self {
        Self {
            route,
            params: HashMap::new(),
        }
    }

    /// Add a parameter.
    pub fn param(mut self, key: impl Into<String>, value: impl Into<ParamValue>) -> Self {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Build the URL.
    pub fn build(self) -> String {
        self.route.url(&self.params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_named_route() {
        let route = NamedRoute::new("users.show", "/users/{id}");
        assert_eq!(route.name(), "users.show");
        assert_eq!(route.pattern(), "/users/{id}");
    }

    #[test]
    fn test_route_url_generation() {
        let route = NamedRoute::new("users.show", "/users/{id}");
        let mut params = HashMap::new();
        params.insert("id".to_string(), ParamValue::Number(123));

        let url = route.url(&params);
        assert_eq!(url, "/users/123");
    }

    #[test]
    fn test_route_registry() {
        let mut registry = RouteRegistry::new();

        let route1 = NamedRoute::new("users.index", "/users");
        let route2 = NamedRoute::new("users.show", "/users/{id}");

        registry.register(route1);
        registry.register(route2);

        assert!(registry.has("users.index"));
        assert!(registry.has("users.show"));
        assert!(!registry.has("posts.index"));

        let mut params = HashMap::new();
        params.insert("id".to_string(), ParamValue::Number(456));

        let url = registry.url("users.show", &params);
        assert_eq!(url, Some("/users/456".to_string()));
    }

    #[test]
    fn test_route_url_builder() {
        let route = NamedRoute::new("posts.show", "/posts/{id}/comments/{comment_id}");
        let url = RouteUrlBuilder::new(route)
            .param("id", 123)
            .param("comment_id", 456)
            .build();

        assert_eq!(url, "/posts/123/comments/456");
    }

    #[test]
    fn test_param_value_conversion() {
        let string_param: ParamValue = "test".into();
        assert_eq!(string_param.as_str(), "test");

        let number_param: ParamValue = 123i64.into();
        assert_eq!(number_param.as_str(), "123");

        let i32_param: ParamValue = 42i32.into();
        assert_eq!(i32_param.as_str(), "42");
    }

    #[test]
    fn test_route_registry_names() {
        let mut registry = RouteRegistry::new();
        registry.register(NamedRoute::new("users.index", "/users"));
        registry.register(NamedRoute::new("posts.index", "/posts"));

        let mut names = registry.names();
        names.sort();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"users.index".to_string()));
        assert!(names.contains(&"posts.index".to_string()));
    }

    #[test]
    fn test_route_registry_overwrite() {
        let mut registry = RouteRegistry::new();
        registry.register(NamedRoute::new("users.show", "/users/{id}"));
        registry.register(NamedRoute::new("users.show", "/accounts/{id}"));

        let mut params = HashMap::new();
        params.insert("id".to_string(), ParamValue::Number(5));
        let url = registry.url("users.show", &params);
        assert_eq!(url, Some("/accounts/5".to_string()));
    }

    #[test]
    fn test_route_registry_url_missing_param_leaves_placeholder() {
        let mut registry = RouteRegistry::new();
        registry.register(NamedRoute::new("users.show", "/users/{id}"));

        let url = registry.url("users.show", &HashMap::new());
        // With no params, the placeholder stays
        assert_eq!(url, Some("/users/{id}".to_string()));
    }

    #[test]
    fn test_route_registry_url_nonexistent_route() {
        let registry = RouteRegistry::new();
        let url = registry.url("nonexistent", &HashMap::new());
        assert!(url.is_none());
    }

    #[test]
    fn test_route_url_builder_string_param() {
        let route = NamedRoute::new("posts.show", "/posts/{slug}");
        let url = RouteUrlBuilder::new(route)
            .param("slug", "hello-world")
            .build();
        assert_eq!(url, "/posts/hello-world");
    }

    #[test]
    fn test_named_route_multiple_params_url() {
        let route = NamedRoute::new("orders.item", "/orders/{order_id}/items/{item_id}");
        let mut params = HashMap::new();
        params.insert("order_id".to_string(), ParamValue::Number(10));
        params.insert("item_id".to_string(), ParamValue::Number(20));
        let url = route.url(&params);
        assert_eq!(url, "/orders/10/items/20");
    }

    #[test]
    fn test_param_value_from_string_owned() {
        let s = String::from("owned-string");
        let param: ParamValue = s.into();
        assert_eq!(param.as_str(), "owned-string");
    }
}
