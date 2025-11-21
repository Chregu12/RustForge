//! Scoped service container implementation
//!
//! Provides request-scoped and tenant-scoped service lifetimes, where instances
//! are created once per scope and shared within that scope.
//!
//! # Example
//!
//! ```rust
//! use rf_container::{ServiceRegistry, Scope, ScopeManager};
//! use std::sync::{Arc, Mutex};
//!
//! #[derive(Clone)]
//! struct RequestLogger {
//!     request_id: u32,
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut registry = ServiceRegistry::new();
//!     let counter = Arc::new(Mutex::new(0u32));
//!     let counter_clone = counter.clone();
//!
//!     registry.register(Scope::Scoped, move || {
//!         let mut count = counter_clone.lock().unwrap();
//!         *count += 1;
//!         Arc::new(RequestLogger {
//!             request_id: *count,
//!         })
//!     });
//!
//!     let registry = Arc::new(registry);
//!     let manager = ScopeManager::new(registry);
//!
//!     manager.with_scope("request-1".to_string(), async {
//!         // Resolve scoped service within scope
//!     }).await;
//! }
//! ```

use crate::{ContainerResult, ServiceRegistry};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::task_local;

/// Scoped container that lives for the duration of a scope (e.g., HTTP request, tenant session)
///
/// A scoped container maintains its own cache of scoped service instances while
/// delegating to the parent registry for singleton and transient services.
///
/// # Thread Safety
///
/// All operations are thread-safe via internal `Mutex`. The scoped container
/// can be safely used across async tasks within the same scope.
///
/// # Lifecycle
///
/// - Created at the start of a scope (e.g., incoming HTTP request)
/// - Maintains instance cache for the scope duration
/// - Automatically cleaned up when the scope ends
pub struct ScopedContainer {
    /// Parent service registry
    parent: Arc<ServiceRegistry>,
    /// Cached scoped instances for this scope
    instances: Arc<Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
    /// Unique identifier for this scope
    scope_id: String,
}

impl ScopedContainer {
    /// Create a new scoped container
    ///
    /// # Parameters
    ///
    /// - `parent`: Parent service registry containing service definitions
    /// - `scope_id`: Unique identifier for this scope (e.g., request ID)
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, ScopedContainer};
    /// use std::sync::Arc;
    ///
    /// let registry = Arc::new(ServiceRegistry::new());
    /// let scoped = ScopedContainer::new(registry, "request-123".to_string());
    /// ```
    pub fn new(parent: Arc<ServiceRegistry>, scope_id: String) -> Self {
        Self {
            parent,
            instances: Arc::new(Mutex::new(HashMap::new())),
            scope_id,
        }
    }

    /// Get the scope identifier
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, ScopedContainer};
    /// # use std::sync::Arc;
    /// let registry = Arc::new(ServiceRegistry::new());
    /// let scoped = ScopedContainer::new(registry, "request-123".to_string());
    /// assert_eq!(scoped.scope_id(), "request-123");
    /// ```
    pub fn scope_id(&self) -> &str {
        &self.scope_id
    }

    /// Resolve a service within this scope
    ///
    /// Resolution strategy:
    /// 1. Check if scoped instance exists in cache (for Scoped services)
    /// 2. If not cached, create and cache the instance
    /// 3. For Singleton/Transient services, delegate to parent registry
    ///
    /// # Type Parameters
    ///
    /// - `T`: Service type to resolve
    ///
    /// # Errors
    ///
    /// - `ServiceNotFound`: Service type not registered
    /// - `DowncastFailed`: Type mismatch during resolution
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, ScopedContainer, Scope};
    /// # use std::sync::Arc;
    /// # #[derive(Clone)]
    /// # struct Logger;
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(Scope::Scoped, || Arc::new(Logger));
    ///
    /// let registry = Arc::new(registry);
    /// let scoped = ScopedContainer::new(registry, "scope-1".to_string());
    ///
    /// let logger: Arc<Logger> = scoped.resolve().unwrap();
    /// ```
    pub fn resolve<T>(&self) -> ContainerResult<Arc<T>>
    where
        T: 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();

        // Check if we have a cached scoped instance
        {
            let instances = self.instances.lock().unwrap();
            if let Some(instance) = instances.get(&type_id) {
                // Try to downcast the cached instance
                if let Ok(arc) = instance.clone().downcast::<T>() {
                    return Ok(arc);
                }
            }
        }

        // Not in cache, resolve from parent
        let instance = self.parent.resolve_for_scope::<T>()?;

        // If this is a scoped service, cache it
        if self.parent.is_scoped::<T>() {
            let mut instances = self.instances.lock().unwrap();
            instances.insert(type_id, instance.clone() as Arc<dyn Any + Send + Sync>);
        }

        Ok(instance)
    }

    /// Check if a scoped instance exists in cache
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, ScopedContainer, Scope};
    /// # use std::sync::Arc;
    /// # #[derive(Clone)]
    /// # struct Logger;
    /// # let mut registry = ServiceRegistry::new();
    /// # registry.register(Scope::Scoped, || Arc::new(Logger));
    /// # let registry = Arc::new(registry);
    /// let scoped = ScopedContainer::new(registry, "scope-1".to_string());
    ///
    /// assert!(!scoped.has_cached::<Logger>());
    /// let _logger: Arc<Logger> = scoped.resolve().unwrap();
    /// assert!(scoped.has_cached::<Logger>());
    /// ```
    pub fn has_cached<T>(&self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let instances = self.instances.lock().unwrap();
        instances.contains_key(&type_id)
    }

    /// Clear all cached instances in this scope
    ///
    /// Useful for testing or manual scope cleanup.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, ScopedContainer};
    /// # use std::sync::Arc;
    /// # let registry = Arc::new(ServiceRegistry::new());
    /// let scoped = ScopedContainer::new(registry, "scope-1".to_string());
    /// scoped.clear();
    /// ```
    pub fn clear(&self) {
        let mut instances = self.instances.lock().unwrap();
        instances.clear();
    }

    /// Get the number of cached instances
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, ScopedContainer};
    /// # use std::sync::Arc;
    /// # let registry = Arc::new(ServiceRegistry::new());
    /// let scoped = ScopedContainer::new(registry, "scope-1".to_string());
    /// assert_eq!(scoped.cached_count(), 0);
    /// ```
    pub fn cached_count(&self) -> usize {
        let instances = self.instances.lock().unwrap();
        instances.len()
    }

    /// Get a reference to the current scope (if any)
    ///
    /// Returns `None` if called outside of a scope.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ScopedContainer, ScopeManager, ServiceRegistry};
    /// # use std::sync::Arc;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let registry = Arc::new(ServiceRegistry::new());
    /// # let manager = ScopeManager::new(registry);
    /// manager.with_scope("test".to_string(), async {
    ///     let scope = ScopedContainer::current();
    ///     assert!(scope.is_some());
    /// }).await;
    /// # }
    /// ```
    pub fn current() -> Option<ScopedContainer> {
        CURRENT_SCOPE.try_with(|scope| scope.clone()).ok()
    }
}

impl Clone for ScopedContainer {
    fn clone(&self) -> Self {
        Self {
            parent: Arc::clone(&self.parent),
            instances: Arc::clone(&self.instances),
            scope_id: self.scope_id.clone(),
        }
    }
}

// Task-local storage for current scope
task_local! {
    /// Current scoped container for the async task
    pub static CURRENT_SCOPE: ScopedContainer;
}

/// Scope manager for creating and managing service scopes
///
/// The scope manager creates new scopes and executes async closures within
/// those scopes. It uses tokio's task-local storage to make the scope
/// available to all code within the async context.
///
/// # Example
///
/// ```rust
/// use rf_container::{ServiceRegistry, ScopeManager};
/// use std::sync::Arc;
///
/// #[tokio::main]
/// async fn main() {
///     let registry = Arc::new(ServiceRegistry::new());
///     let manager = ScopeManager::new(registry);
///
///     manager.with_scope("request-1".to_string(), async {
///         // Your request handling code here
///         // Scoped services will be created once per request
///     }).await;
/// }
/// ```
pub struct ScopeManager {
    registry: Arc<ServiceRegistry>,
}

impl ScopeManager {
    /// Create a new scope manager
    ///
    /// # Parameters
    ///
    /// - `registry`: Service registry containing service definitions
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, ScopeManager};
    /// use std::sync::Arc;
    ///
    /// let registry = Arc::new(ServiceRegistry::new());
    /// let manager = ScopeManager::new(registry);
    /// ```
    pub fn new(registry: Arc<ServiceRegistry>) -> Self {
        Self { registry }
    }

    /// Execute a future within a new scope
    ///
    /// Creates a scoped container and sets it as the current scope for the
    /// duration of the async closure execution.
    ///
    /// # Parameters
    ///
    /// - `scope_id`: Unique identifier for this scope
    /// - `f`: Async closure to execute within the scope
    ///
    /// # Returns
    ///
    /// The result of the async closure
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, ScopeManager, ScopedContainer};
    /// # use std::sync::Arc;
    /// # #[derive(Clone)]
    /// # struct Logger;
    /// # #[tokio::main]
    /// # async fn main() {
    /// # let registry = Arc::new(ServiceRegistry::new());
    /// let manager = ScopeManager::new(registry);
    ///
    /// let result = manager.with_scope("request-123".to_string(), async {
    ///     // Access current scope
    ///     if let Some(scope) = ScopedContainer::current() {
    ///         println!("Scope ID: {}", scope.scope_id());
    ///     }
    ///
    ///     42 // Return value
    /// }).await;
    ///
    /// assert_eq!(result, 42);
    /// # }
    /// ```
    pub async fn with_scope<F, R>(&self, scope_id: String, f: F) -> R
    where
        F: std::future::Future<Output = R>,
    {
        let scoped = ScopedContainer::new(Arc::clone(&self.registry), scope_id);

        CURRENT_SCOPE.scope(scoped, f).await
    }
}

impl Clone for ScopeManager {
    fn clone(&self) -> Self {
        Self {
            registry: Arc::clone(&self.registry),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scope;

    #[derive(Clone, Debug, PartialEq)]
    struct RequestLogger {
        request_id: String,
        instance_id: u32,
    }

    #[derive(Clone, Debug)]
    struct TenantDatabase {
        tenant_id: String,
        connection_id: u32,
    }

    #[derive(Clone)]
    struct Counter {
        count: Arc<Mutex<u32>>,
    }

    impl Counter {
        fn new() -> Self {
            Self {
                count: Arc::new(Mutex::new(0)),
            }
        }

        fn increment(&self) -> u32 {
            let mut count = self.count.lock().unwrap();
            *count += 1;
            *count
        }

        fn get(&self) -> u32 {
            *self.count.lock().unwrap()
        }
    }

    #[tokio::test]
    async fn test_scoped_instance_created_once_per_scope() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Scoped, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            let instance_id = *count;
            Arc::new(RequestLogger {
                request_id: "test".to_string(),
                instance_id,
            })
        });

        let registry = Arc::new(registry);
        let manager = ScopeManager::new(registry);

        // First scope
        manager
            .with_scope("scope-1".to_string(), async {
                let logger1: Arc<RequestLogger> =
                    ScopedContainer::current().unwrap().resolve().unwrap();
                let logger2: Arc<RequestLogger> =
                    ScopedContainer::current().unwrap().resolve().unwrap();

                // Same instance within scope
                assert_eq!(logger1.instance_id, logger2.instance_id);
                assert_eq!(logger1.instance_id, 1);
            })
            .await;

        // Second scope
        manager
            .with_scope("scope-2".to_string(), async {
                let logger: Arc<RequestLogger> =
                    ScopedContainer::current().unwrap().resolve().unwrap();

                // New instance for new scope
                assert_eq!(logger.instance_id, 2);
            })
            .await;

        // Factory called twice (once per scope)
        let count = counter.lock().unwrap();
        assert_eq!(*count, 2);
    }

    #[tokio::test]
    async fn test_different_instances_across_scopes() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Scoped, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            Arc::new(TenantDatabase {
                tenant_id: format!("tenant-{}", *count),
                connection_id: *count,
            })
        });

        let registry = Arc::new(registry);
        let manager = ScopeManager::new(registry);

        let id1 = manager
            .with_scope("request-1".to_string(), async {
                let db: Arc<TenantDatabase> =
                    ScopedContainer::current().unwrap().resolve().unwrap();
                db.connection_id
            })
            .await;

        let id2 = manager
            .with_scope("request-2".to_string(), async {
                let db: Arc<TenantDatabase> =
                    ScopedContainer::current().unwrap().resolve().unwrap();
                db.connection_id
            })
            .await;

        assert_ne!(id1, id2);
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
    }

    #[tokio::test]
    async fn test_nested_scopes() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Scoped, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            Arc::new(RequestLogger {
                request_id: "test".to_string(),
                instance_id: *count,
            })
        });

        let registry = Arc::new(registry);
        let manager = ScopeManager::new(Arc::clone(&registry));

        manager
            .with_scope("outer".to_string(), async {
                let outer_logger: Arc<RequestLogger> =
                    ScopedContainer::current().unwrap().resolve().unwrap();

                let inner_manager = ScopeManager::new(registry);
                inner_manager
                    .with_scope("inner".to_string(), async {
                        let inner_logger: Arc<RequestLogger> =
                            ScopedContainer::current().unwrap().resolve().unwrap();

                        // Different scope = different instance
                        assert_ne!(outer_logger.instance_id, inner_logger.instance_id);
                    })
                    .await;
            })
            .await;
    }

    #[tokio::test]
    async fn test_scoped_container_scope_id() {
        let registry = Arc::new(ServiceRegistry::new());
        let scoped = ScopedContainer::new(registry, "test-scope".to_string());

        assert_eq!(scoped.scope_id(), "test-scope");
    }

    #[tokio::test]
    async fn test_has_cached() {
        let mut registry = ServiceRegistry::new();
        registry.register(Scope::Scoped, || {
            Arc::new(RequestLogger {
                request_id: "test".to_string(),
                instance_id: 1,
            })
        });

        let registry = Arc::new(registry);
        let scoped = ScopedContainer::new(registry, "scope-1".to_string());

        assert!(!scoped.has_cached::<RequestLogger>());

        let _logger: Arc<RequestLogger> = scoped.resolve().unwrap();

        assert!(scoped.has_cached::<RequestLogger>());
    }

    #[tokio::test]
    async fn test_clear_cache() {
        let mut registry = ServiceRegistry::new();
        registry.register(Scope::Scoped, || {
            Arc::new(RequestLogger {
                request_id: "test".to_string(),
                instance_id: 1,
            })
        });

        let registry = Arc::new(registry);
        let scoped = ScopedContainer::new(registry, "scope-1".to_string());

        let _logger: Arc<RequestLogger> = scoped.resolve().unwrap();
        assert!(scoped.has_cached::<RequestLogger>());

        scoped.clear();
        assert!(!scoped.has_cached::<RequestLogger>());
    }

    #[tokio::test]
    async fn test_cached_count() {
        let mut registry = ServiceRegistry::new();
        registry.register(Scope::Scoped, || {
            Arc::new(RequestLogger {
                request_id: "test".to_string(),
                instance_id: 1,
            })
        });
        registry.register(Scope::Scoped, || {
            Arc::new(TenantDatabase {
                tenant_id: "test".to_string(),
                connection_id: 1,
            })
        });

        let registry = Arc::new(registry);
        let scoped = ScopedContainer::new(registry, "scope-1".to_string());

        assert_eq!(scoped.cached_count(), 0);

        let _logger: Arc<RequestLogger> = scoped.resolve().unwrap();
        assert_eq!(scoped.cached_count(), 1);

        let _db: Arc<TenantDatabase> = scoped.resolve().unwrap();
        assert_eq!(scoped.cached_count(), 2);
    }

    #[tokio::test]
    async fn test_current_scope_available_within_scope() {
        let registry = Arc::new(ServiceRegistry::new());
        let manager = ScopeManager::new(registry);

        manager
            .with_scope("test-scope".to_string(), async {
                let scope = ScopedContainer::current();
                assert!(scope.is_some());
                assert_eq!(scope.unwrap().scope_id(), "test-scope");
            })
            .await;
    }

    #[tokio::test]
    async fn test_current_scope_none_outside_scope() {
        let scope = ScopedContainer::current();
        assert!(scope.is_none());
    }

    #[tokio::test]
    async fn test_scope_manager_clone() {
        let registry = Arc::new(ServiceRegistry::new());
        let manager1 = ScopeManager::new(registry);
        let manager2 = manager1.clone();

        manager2
            .with_scope("test".to_string(), async {
                let scope = ScopedContainer::current();
                assert!(scope.is_some());
            })
            .await;
    }

    #[tokio::test]
    async fn test_concurrent_scopes() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Scoped, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            Arc::new(RequestLogger {
                request_id: "test".to_string(),
                instance_id: *count,
            })
        });

        let registry = Arc::new(registry);
        let manager = ScopeManager::new(registry);

        // Spawn multiple concurrent scopes
        let mut handles = vec![];

        for i in 0..5 {
            let manager_clone = manager.clone();
            let handle = tokio::spawn(async move {
                manager_clone
                    .with_scope(format!("scope-{}", i), async {
                        let logger: Arc<RequestLogger> =
                            ScopedContainer::current().unwrap().resolve().unwrap();
                        logger.instance_id
                    })
                    .await
            });
            handles.push(handle);
        }

        // Collect results
        let mut instance_ids = vec![];
        for handle in handles {
            instance_ids.push(handle.await.unwrap());
        }

        // Each scope should have a unique instance
        instance_ids.sort();
        instance_ids.dedup();
        assert_eq!(instance_ids.len(), 5);
    }
}
