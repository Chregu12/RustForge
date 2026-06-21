//! Middleware pipeline for managing and applying middleware to routes.
//!
//! This module provides Laravel-like middleware management with:
//! - Global middleware registry
//! - Named middleware
//! - Middleware stacks
//! - Pipeline building

use axum::{extract::Request, middleware::Next, response::Response};
use futures::future::BoxFuture;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

/// Type alias for middleware handler functions.
pub type MiddlewareHandler =
    Arc<dyn Fn(Request, Next) -> BoxFuture<'static, Result<Response, Response>> + Send + Sync>;

/// Registry for managing named middleware.
#[derive(Clone)]
pub struct MiddlewareRegistry {
    middleware: Arc<RwLock<HashMap<String, MiddlewareHandler>>>,
}

impl MiddlewareRegistry {
    /// Create a new middleware registry.
    pub fn new() -> Self {
        Self {
            middleware: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a middleware with a name.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_routing::MiddlewareRegistry;
    /// use axum::{extract::Request, middleware::Next, response::Response};
    /// use futures::future::BoxFuture;
    ///
    /// let mut registry = MiddlewareRegistry::new();
    ///
    /// registry.register("auth", |req: Request, next: Next| {
    ///     Box::pin(async move {
    ///         // Authentication logic here
    ///         Ok(next.run(req).await)
    ///     })
    /// });
    /// ```
    pub fn register<F>(&self, name: impl Into<String>, handler: F)
    where
        F: Fn(Request, Next) -> BoxFuture<'static, Result<Response, Response>>
            + Send
            + Sync
            + 'static,
    {
        self.middleware
            .write()
            .insert(name.into(), Arc::new(handler));
    }

    /// Get a middleware by name.
    pub fn get(&self, name: &str) -> Option<MiddlewareHandler> {
        self.middleware.read().get(name).cloned()
    }

    /// Check if a middleware exists.
    pub fn has(&self, name: &str) -> bool {
        self.middleware.read().contains_key(name)
    }

    /// Get all registered middleware names.
    pub fn names(&self) -> Vec<String> {
        self.middleware.read().keys().cloned().collect()
    }

    /// Remove a middleware.
    pub fn remove(&self, name: &str) -> bool {
        self.middleware.write().remove(name).is_some()
    }

    /// Clear all middleware.
    pub fn clear(&self) {
        self.middleware.write().clear();
    }
}

impl Default for MiddlewareRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for MiddlewareRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiddlewareRegistry")
            .field("count", &self.middleware.read().len())
            .finish()
    }
}

/// Middleware pipeline for building and applying middleware stacks.
#[derive(Debug, Clone)]
pub struct MiddlewarePipeline {
    registry: Arc<MiddlewareRegistry>,
    stack: Vec<String>,
}

impl MiddlewarePipeline {
    /// Create a new middleware pipeline.
    pub fn new(registry: Arc<MiddlewareRegistry>) -> Self {
        Self {
            registry,
            stack: Vec::new(),
        }
    }

    /// Add middleware to the pipeline.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_routing::{MiddlewareRegistry, MiddlewarePipeline};
    /// use std::sync::Arc;
    ///
    /// let registry = Arc::new(MiddlewareRegistry::new());
    /// let pipeline = MiddlewarePipeline::new(registry)
    ///     .push("auth")
    ///     .push("throttle");
    /// ```
    pub fn push(mut self, name: impl Into<String>) -> Self {
        self.stack.push(name.into());
        self
    }

    /// Get the middleware stack.
    pub fn stack(&self) -> &[String] {
        &self.stack
    }

    /// Check if the pipeline is empty.
    pub fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    /// Get the number of middleware in the pipeline.
    pub fn len(&self) -> usize {
        self.stack.len()
    }

    /// Clear the pipeline.
    pub fn clear(mut self) -> Self {
        self.stack.clear();
        self
    }

    /// Get the registry.
    pub fn registry(&self) -> &Arc<MiddlewareRegistry> {
        &self.registry
    }
}

/// Global middleware registry instance.
static GLOBAL_REGISTRY: RwLock<Option<Arc<MiddlewareRegistry>>> = RwLock::new(None);

/// Get or create the global middleware registry.
pub fn global_registry() -> Arc<MiddlewareRegistry> {
    let read_guard = GLOBAL_REGISTRY.read();
    if let Some(registry) = read_guard.as_ref() {
        return Arc::clone(registry);
    }
    drop(read_guard);

    let mut write_guard = GLOBAL_REGISTRY.write();
    if let Some(registry) = write_guard.as_ref() {
        return Arc::clone(registry);
    }

    let registry = Arc::new(MiddlewareRegistry::new());
    *write_guard = Some(Arc::clone(&registry));
    registry
}

/// Register a middleware globally.
///
/// # Example
///
/// ```rust,no_run
/// use rf_routing::register_middleware;
/// use axum::{extract::Request, middleware::Next, response::Response};
/// use futures::future::BoxFuture;
///
/// register_middleware("auth", |req: Request, next: Next| {
///     Box::pin(async move {
///         // Authentication logic here
///         Ok(next.run(req).await)
///     })
/// });
/// ```
pub fn register_middleware<F>(name: impl Into<String>, handler: F)
where
    F: Fn(Request, Next) -> BoxFuture<'static, Result<Response, Response>> + Send + Sync + 'static,
{
    global_registry().register(name, handler);
}

/// Create a middleware pipeline from the global registry.
pub fn pipeline() -> MiddlewarePipeline {
    MiddlewarePipeline::new(global_registry())
}

/// Middleware group for common middleware combinations.
#[derive(Debug, Clone)]
pub struct MiddlewareGroup {
    name: String,
    middleware: Vec<String>,
}

impl MiddlewareGroup {
    /// Create a new middleware group.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            middleware: Vec::new(),
        }
    }

    /// Add middleware to the group.
    #[allow(clippy::should_implement_trait)] // intentional builder method, not std::ops::Add
    pub fn add(mut self, middleware: impl Into<String>) -> Self {
        self.middleware.push(middleware.into());
        self
    }

    /// Get the group name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the middleware stack.
    pub fn middleware(&self) -> &[String] {
        &self.middleware
    }

    /// Apply this group to a pipeline.
    pub fn apply_to(&self, pipeline: MiddlewarePipeline) -> MiddlewarePipeline {
        let mut result = pipeline;
        for mw in &self.middleware {
            result = result.push(mw);
        }
        result
    }
}

/// Registry for middleware groups.
#[derive(Debug, Clone, Default)]
pub struct MiddlewareGroupRegistry {
    groups: HashMap<String, MiddlewareGroup>,
}

impl MiddlewareGroupRegistry {
    /// Create a new middleware group registry.
    pub fn new() -> Self {
        Self {
            groups: HashMap::new(),
        }
    }

    /// Register a middleware group.
    pub fn register(&mut self, group: MiddlewareGroup) {
        self.groups.insert(group.name().to_string(), group);
    }

    /// Get a middleware group by name.
    pub fn get(&self, name: &str) -> Option<&MiddlewareGroup> {
        self.groups.get(name)
    }

    /// Check if a group exists.
    pub fn has(&self, name: &str) -> bool {
        self.groups.contains_key(name)
    }

    /// Get all group names.
    pub fn names(&self) -> Vec<String> {
        self.groups.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_middleware_registry_creation() {
        let registry = MiddlewareRegistry::new();
        assert_eq!(registry.names().len(), 0);
    }

    #[test]
    fn test_middleware_registry_register() {
        let registry = MiddlewareRegistry::new();

        registry.register("auth", |req: Request, next: Next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });

        assert!(registry.has("auth"));
        assert_eq!(registry.names().len(), 1);
    }

    #[test]
    fn test_middleware_registry_get() {
        let registry = MiddlewareRegistry::new();

        registry.register("auth", |req: Request, next: Next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });

        let middleware = registry.get("auth");
        assert!(middleware.is_some());

        let missing = registry.get("nonexistent");
        assert!(missing.is_none());
    }

    #[test]
    fn test_middleware_registry_remove() {
        let registry = MiddlewareRegistry::new();

        registry.register("auth", |req: Request, next: Next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });

        assert!(registry.has("auth"));
        assert!(registry.remove("auth"));
        assert!(!registry.has("auth"));
    }

    #[test]
    fn test_middleware_registry_clear() {
        let registry = MiddlewareRegistry::new();

        registry.register("auth", |req: Request, next: Next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });
        registry.register("throttle", |req: Request, next: Next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });

        assert_eq!(registry.names().len(), 2);
        registry.clear();
        assert_eq!(registry.names().len(), 0);
    }

    #[test]
    fn test_middleware_pipeline_creation() {
        let registry = Arc::new(MiddlewareRegistry::new());
        let pipeline = MiddlewarePipeline::new(registry);

        assert!(pipeline.is_empty());
        assert_eq!(pipeline.len(), 0);
    }

    #[test]
    fn test_middleware_pipeline_push() {
        let registry = Arc::new(MiddlewareRegistry::new());
        let pipeline = MiddlewarePipeline::new(registry)
            .push("auth")
            .push("throttle");

        assert_eq!(pipeline.len(), 2);
        assert_eq!(pipeline.stack(), &["auth", "throttle"]);
    }

    #[test]
    fn test_middleware_pipeline_clear() {
        let registry = Arc::new(MiddlewareRegistry::new());
        let pipeline = MiddlewarePipeline::new(registry)
            .push("auth")
            .push("throttle")
            .clear();

        assert!(pipeline.is_empty());
    }

    #[test]
    fn test_global_registry() {
        let registry1 = global_registry();
        let registry2 = global_registry();

        // Should return the same instance
        assert!(Arc::ptr_eq(&registry1, &registry2));
    }

    #[test]
    fn test_register_middleware_global() {
        register_middleware("test", |req: Request, next: Next| {
            Box::pin(async move { Ok(next.run(req).await) })
        });

        let registry = global_registry();
        assert!(registry.has("test"));
    }

    #[test]
    fn test_pipeline_from_global() {
        let pipeline = pipeline().push("auth").push("throttle");
        assert_eq!(pipeline.len(), 2);
    }

    #[test]
    fn test_middleware_group() {
        let group = MiddlewareGroup::new("api")
            .add("auth")
            .add("throttle")
            .add("cors");

        assert_eq!(group.name(), "api");
        assert_eq!(group.middleware().len(), 3);
    }

    #[test]
    fn test_middleware_group_apply() {
        let group = MiddlewareGroup::new("api").add("auth").add("throttle");

        let registry = Arc::new(MiddlewareRegistry::new());
        let pipeline = MiddlewarePipeline::new(registry);
        let pipeline = group.apply_to(pipeline);

        assert_eq!(pipeline.len(), 2);
        assert_eq!(pipeline.stack(), &["auth", "throttle"]);
    }

    #[test]
    fn test_middleware_group_registry() {
        let mut registry = MiddlewareGroupRegistry::new();

        let group1 = MiddlewareGroup::new("api").add("auth");
        let group2 = MiddlewareGroup::new("web").add("session");

        registry.register(group1);
        registry.register(group2);

        assert!(registry.has("api"));
        assert!(registry.has("web"));
        assert_eq!(registry.names().len(), 2);
    }
}
