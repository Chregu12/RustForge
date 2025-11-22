//! Middleware stack management for organizing and applying middleware to routes
//!
//! This module provides a comprehensive middleware management system with:
//! - Global middleware (applied to all routes)
//! - Route group middleware
//! - Route-specific middleware
//! - Middleware resolution and ordering

use crate::middleware_pipeline::{MiddlewareHandler, MiddlewareRegistry};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Middleware stack for managing all middleware layers
#[derive(Clone)]
pub struct MiddlewareStack {
    /// Global middleware applied to all routes
    global: Arc<RwLock<Vec<String>>>,
    /// Middleware groups (name -> middleware names)
    groups: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Route-specific middleware (route name -> middleware names)
    route_middleware: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Registry for middleware handlers
    registry: Arc<MiddlewareRegistry>,
}

impl MiddlewareStack {
    /// Create a new middleware stack
    pub fn new() -> Self {
        Self {
            global: Arc::new(RwLock::new(Vec::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            route_middleware: Arc::new(RwLock::new(HashMap::new())),
            registry: Arc::new(MiddlewareRegistry::new()),
        }
    }

    /// Create a middleware stack with an existing registry
    pub fn with_registry(registry: Arc<MiddlewareRegistry>) -> Self {
        Self {
            global: Arc::new(RwLock::new(Vec::new())),
            groups: Arc::new(RwLock::new(HashMap::new())),
            route_middleware: Arc::new(RwLock::new(HashMap::new())),
            registry,
        }
    }

    /// Add global middleware (applied to all routes)
    pub fn add_global(&self, middleware: impl Into<String>) -> &Self {
        self.global.write().push(middleware.into());
        self
    }

    /// Add multiple global middleware
    pub fn add_global_middleware(&self, middleware: Vec<String>) -> &Self {
        self.global.write().extend(middleware);
        self
    }

    /// Add a middleware group
    pub fn add_group(&self, name: impl Into<String>, middleware: Vec<String>) -> &Self {
        self.groups.write().insert(name.into(), middleware);
        self
    }

    /// Add middleware to an existing group
    pub fn append_to_group(&self, group_name: &str, middleware: impl Into<String>) -> &Self {
        let mut groups = self.groups.write();
        groups
            .entry(group_name.to_string())
            .or_insert_with(Vec::new)
            .push(middleware.into());
        self
    }

    /// Add route-specific middleware
    pub fn add_route_middleware(
        &self,
        route_name: impl Into<String>,
        middleware: Vec<String>,
    ) -> &Self {
        self.route_middleware
            .write()
            .insert(route_name.into(), middleware);
        self
    }

    /// Append middleware to a route
    pub fn append_route_middleware(
        &self,
        route_name: &str,
        middleware: impl Into<String>,
    ) -> &Self {
        let mut route_mw = self.route_middleware.write();
        route_mw
            .entry(route_name.to_string())
            .or_insert_with(Vec::new)
            .push(middleware.into());
        self
    }

    /// Get global middleware
    pub fn global(&self) -> Vec<String> {
        self.global.read().clone()
    }

    /// Get middleware for a group
    pub fn group(&self, name: &str) -> Option<Vec<String>> {
        self.groups.read().get(name).cloned()
    }

    /// Get all groups
    pub fn groups(&self) -> HashMap<String, Vec<String>> {
        self.groups.read().clone()
    }

    /// Get middleware for a route
    pub fn route(&self, route_name: &str) -> Option<Vec<String>> {
        self.route_middleware.read().get(route_name).cloned()
    }

    /// Resolve middleware for a route with groups
    ///
    /// Returns middleware in order: global -> group -> route
    pub fn resolve(&self, route_name: &str, groups: &[String]) -> Vec<String> {
        let mut middleware = Vec::new();

        // Add global middleware
        middleware.extend(self.global.read().clone());

        // Add group middleware
        let groups_lock = self.groups.read();
        for group_name in groups {
            if let Some(group_mw) = groups_lock.get(group_name) {
                middleware.extend(group_mw.clone());
            }
        }

        // Add route-specific middleware
        if let Some(route_mw) = self.route_middleware.read().get(route_name) {
            middleware.extend(route_mw.clone());
        }

        // Remove duplicates while preserving order
        let mut seen = std::collections::HashSet::new();
        middleware.retain(|mw| seen.insert(mw.clone()));

        middleware
    }

    /// Get the middleware registry
    pub fn registry(&self) -> &Arc<MiddlewareRegistry> {
        &self.registry
    }

    /// Get middleware handlers for a route
    pub fn resolve_handlers(&self, route_name: &str, groups: &[String]) -> Vec<MiddlewareHandler> {
        let middleware_names = self.resolve(route_name, groups);

        middleware_names
            .iter()
            .filter_map(|name| self.registry.get(name))
            .collect()
    }

    /// Clear all middleware
    pub fn clear(&self) {
        self.global.write().clear();
        self.groups.write().clear();
        self.route_middleware.write().clear();
    }

    /// Remove a middleware group
    pub fn remove_group(&self, name: &str) -> bool {
        self.groups.write().remove(name).is_some()
    }

    /// Remove route middleware
    pub fn remove_route_middleware(&self, route_name: &str) -> bool {
        self.route_middleware.write().remove(route_name).is_some()
    }
}

impl Default for MiddlewareStack {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MiddlewareStack {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiddlewareStack")
            .field("global_count", &self.global.read().len())
            .field("groups_count", &self.groups.read().len())
            .field("routes_count", &self.route_middleware.read().len())
            .finish()
    }
}

/// Builder for creating middleware stacks with a fluent API
pub struct MiddlewareStackBuilder {
    stack: MiddlewareStack,
}

impl MiddlewareStackBuilder {
    /// Create a new middleware stack builder
    pub fn new() -> Self {
        Self {
            stack: MiddlewareStack::new(),
        }
    }

    /// Create with an existing registry
    pub fn with_registry(registry: Arc<MiddlewareRegistry>) -> Self {
        Self {
            stack: MiddlewareStack::with_registry(registry),
        }
    }

    /// Add global middleware
    pub fn global(self, middleware: impl Into<String>) -> Self {
        self.stack.add_global(middleware);
        self
    }

    /// Add a middleware group
    pub fn group(self, name: impl Into<String>, middleware: Vec<String>) -> Self {
        self.stack.add_group(name, middleware);
        self
    }

    /// Add route middleware
    pub fn route(self, route_name: impl Into<String>, middleware: Vec<String>) -> Self {
        self.stack.add_route_middleware(route_name, middleware);
        self
    }

    /// Build the middleware stack
    pub fn build(self) -> MiddlewareStack {
        self.stack
    }
}

impl Default for MiddlewareStackBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        stack.add_global("auth");

        let global = stack.global();
        assert_eq!(global.len(), 2);
        assert_eq!(global[0], "cors");
        assert_eq!(global[1], "auth");
    }

    #[test]
    fn test_add_middleware_group() {
        let stack = MiddlewareStack::new();
        stack.add_group("api", vec!["auth".to_string(), "throttle".to_string()]);

        let group = stack.group("api");
        assert!(group.is_some());
        assert_eq!(group.unwrap().len(), 2);
    }

    #[test]
    fn test_append_to_group() {
        let stack = MiddlewareStack::new();
        stack.add_group("api", vec!["auth".to_string()]);
        stack.append_to_group("api", "throttle");

        let group = stack.group("api").unwrap();
        assert_eq!(group.len(), 2);
        assert_eq!(group[1], "throttle");
    }

    #[test]
    fn test_add_route_middleware() {
        let stack = MiddlewareStack::new();
        stack.add_route_middleware("users.create", vec!["validate".to_string()]);

        let route_mw = stack.route("users.create");
        assert!(route_mw.is_some());
        assert_eq!(route_mw.unwrap()[0], "validate");
    }

    #[test]
    fn test_resolve_middleware() {
        let stack = MiddlewareStack::new();

        // Set up global, group, and route middleware
        stack.add_global("cors");
        stack.add_group("api", vec!["auth".to_string(), "throttle".to_string()]);
        stack.add_route_middleware("users.create", vec!["validate".to_string()]);

        // Resolve for a route with group
        let resolved = stack.resolve("users.create", &vec!["api".to_string()]);

        assert_eq!(resolved.len(), 4);
        assert_eq!(resolved[0], "cors"); // global
        assert_eq!(resolved[1], "auth"); // group
        assert_eq!(resolved[2], "throttle"); // group
        assert_eq!(resolved[3], "validate"); // route
    }

    #[test]
    fn test_resolve_removes_duplicates() {
        let stack = MiddlewareStack::new();

        stack.add_global("auth");
        stack.add_group("api", vec!["auth".to_string(), "throttle".to_string()]);

        let resolved = stack.resolve("test.route", &vec!["api".to_string()]);

        assert_eq!(resolved.len(), 2);
        assert_eq!(resolved[0], "auth");
        assert_eq!(resolved[1], "throttle");
    }

    #[test]
    fn test_remove_group() {
        let stack = MiddlewareStack::new();
        stack.add_group("api", vec!["auth".to_string()]);

        assert!(stack.group("api").is_some());
        assert!(stack.remove_group("api"));
        assert!(stack.group("api").is_none());
    }

    #[test]
    fn test_remove_route_middleware() {
        let stack = MiddlewareStack::new();
        stack.add_route_middleware("test", vec!["auth".to_string()]);

        assert!(stack.route("test").is_some());
        assert!(stack.remove_route_middleware("test"));
        assert!(stack.route("test").is_none());
    }

    #[test]
    fn test_clear() {
        let stack = MiddlewareStack::new();

        stack.add_global("cors");
        stack.add_group("api", vec!["auth".to_string()]);
        stack.add_route_middleware("test", vec!["validate".to_string()]);

        stack.clear();

        assert_eq!(stack.global().len(), 0);
        assert_eq!(stack.groups().len(), 0);
        assert!(stack.route("test").is_none());
    }

    #[test]
    fn test_builder() {
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
    fn test_resolve_multiple_groups() {
        let stack = MiddlewareStack::new();

        stack.add_global("cors");
        stack.add_group("web", vec!["session".to_string()]);
        stack.add_group("admin", vec!["auth".to_string(), "admin".to_string()]);

        let resolved = stack.resolve("admin.users", &vec!["web".to_string(), "admin".to_string()]);

        assert_eq!(resolved.len(), 4);
        assert_eq!(resolved[0], "cors");
        assert_eq!(resolved[1], "session");
        assert_eq!(resolved[2], "auth");
        assert_eq!(resolved[3], "admin");
    }
}
