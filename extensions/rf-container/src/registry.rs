//! Service registry implementation

use crate::{ContainerError, ContainerResult, Scope};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Factory function that creates service instances
type Factory = Box<dyn Fn() -> Arc<dyn Any + Send + Sync> + Send + Sync>;

/// Service registration entry
struct ServiceEntry {
    scope: Scope,
    factory: Factory,
    singleton_instance: Option<Arc<dyn Any + Send + Sync>>,
}

/// Thread-safe dependency injection container
///
/// Provides type-safe service registration and resolution with
/// configurable lifecycle scopes.
///
/// # Thread Safety
///
/// All operations are thread-safe via internal `Mutex`. Multiple threads
/// can safely register and resolve services concurrently.
///
/// # Example
///
/// ```rust
/// use rf_container::{ServiceRegistry, Scope};
/// use std::sync::Arc;
///
/// #[derive(Clone, Debug)]
/// struct DatabaseConfig {
///     url: String,
///     max_connections: u32,
/// }
///
/// let mut registry = ServiceRegistry::new();
///
/// // Register singleton
/// registry.register(
///     Scope::Singleton,
///     || Arc::new(DatabaseConfig {
///         url: "postgres://localhost".to_string(),
///         max_connections: 10,
///     })
/// );
///
/// // Resolve
/// let config: Arc<DatabaseConfig> = registry.resolve().unwrap();
/// assert_eq!(config.max_connections, 10);
/// ```
pub struct ServiceRegistry {
    services: Arc<Mutex<HashMap<TypeId, ServiceEntry>>>,
}

impl ServiceRegistry {
    /// Create a new empty service registry
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::ServiceRegistry;
    ///
    /// let registry = ServiceRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            services: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a service with a factory function
    ///
    /// # Type Parameters
    ///
    /// - `T`: Service type (must be `'static + Send + Sync`)
    ///
    /// # Parameters
    ///
    /// - `scope`: Lifecycle scope for the service
    /// - `factory`: Function that creates `Arc<T>` instances
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// struct Logger;
    ///
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(
    ///     Scope::Singleton,
    ///     || Arc::new(Logger)
    /// );
    /// ```
    pub fn register<T, F>(&mut self, scope: Scope, factory: F)
    where
        T: 'static + Send + Sync,
        F: Fn() -> Arc<T> + 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();

        let type_erased_factory = Box::new(move || {
            let instance = factory();
            instance as Arc<dyn Any + Send + Sync>
        });

        let entry = ServiceEntry {
            scope,
            factory: type_erased_factory,
            singleton_instance: None,
        };

        let mut services = self.services.lock().unwrap();
        services.insert(type_id, entry);
    }

    /// Resolve a service instance
    ///
    /// Returns an `Arc<T>` to the service instance. Behavior depends on scope:
    /// - **Singleton**: Returns cached instance, or creates and caches on first call
    /// - **Scoped**: Not yet implemented (returns error)
    /// - **Transient**: Creates new instance on every call
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
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// #[derive(Clone)]
    /// struct Config { port: u16 }
    ///
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(Scope::Singleton, || Arc::new(Config { port: 8080 }));
    ///
    /// let config: Arc<Config> = registry.resolve().unwrap();
    /// assert_eq!(config.port, 8080);
    /// ```
    pub fn resolve<T>(&self) -> ContainerResult<Arc<T>>
    where
        T: 'static + Send + Sync,
    {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        let mut services = self.services.lock().unwrap();

        let entry = services
            .get_mut(&type_id)
            .ok_or_else(|| ContainerError::ServiceNotFound {
                type_name: type_name.to_string(),
            })?;

        let instance: Arc<dyn Any + Send + Sync> = match entry.scope {
            Scope::Singleton => {
                // Return cached instance or create and cache
                if let Some(cached) = &entry.singleton_instance {
                    cached.clone()
                } else {
                    let new_instance = (entry.factory)();
                    entry.singleton_instance = Some(new_instance.clone());
                    new_instance
                }
            }
            Scope::Scoped => {
                // Scoped services are resolved through ScopedContainer
                // When called directly on registry, create new instance each time
                (entry.factory)()
            }
            Scope::Transient => {
                // Always create new instance
                (entry.factory)()
            }
        };

        // Downcast to concrete type
        instance
            .downcast::<T>()
            .map_err(|_| ContainerError::DowncastFailed {
                type_name: type_name.to_string(),
            })
    }

    /// Check if a service type is registered
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// struct Logger;
    ///
    /// let mut registry = ServiceRegistry::new();
    /// assert!(!registry.has::<Logger>());
    ///
    /// registry.register(Scope::Singleton, || Arc::new(Logger));
    /// assert!(registry.has::<Logger>());
    /// ```
    pub fn has<T>(&self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let services = self.services.lock().unwrap();
        services.contains_key(&type_id)
    }

    /// Remove a service registration
    ///
    /// Returns `true` if the service was registered and removed.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// struct Logger;
    ///
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(Scope::Singleton, || Arc::new(Logger));
    ///
    /// assert!(registry.remove::<Logger>());
    /// assert!(!registry.has::<Logger>());
    /// ```
    pub fn remove<T>(&mut self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let mut services = self.services.lock().unwrap();
        services.remove(&type_id).is_some()
    }

    /// Clear all service registrations
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::ServiceRegistry;
    ///
    /// let mut registry = ServiceRegistry::new();
    /// // ... register services ...
    /// registry.clear();
    /// ```
    pub fn clear(&mut self) {
        let mut services = self.services.lock().unwrap();
        services.clear();
    }

    /// Resolve a service for use in a scoped container
    ///
    /// This is used internally by `ScopedContainer` to resolve services.
    /// For scoped services, always creates a new instance.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Service type to resolve
    ///
    /// # Errors
    ///
    /// - `ServiceNotFound`: Service type not registered
    /// - `DowncastFailed`: Type mismatch during resolution
    pub fn resolve_for_scope<T>(&self) -> ContainerResult<Arc<T>>
    where
        T: 'static + Send + Sync,
    {
        // For scoped containers, we always create new instances
        // The scoped container handles caching
        self.resolve::<T>()
    }

    /// Check if a service is registered with Scoped lifetime
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// struct Logger;
    ///
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(Scope::Scoped, || Arc::new(Logger));
    ///
    /// assert!(registry.is_scoped::<Logger>());
    /// ```
    pub fn is_scoped<T>(&self) -> bool
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let services = self.services.lock().unwrap();
        services
            .get(&type_id)
            .map(|entry| entry.scope == Scope::Scoped)
            .unwrap_or(false)
    }

    /// Get the scope of a registered service
    ///
    /// Returns `None` if the service is not registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use rf_container::{ServiceRegistry, Scope};
    /// use std::sync::Arc;
    ///
    /// struct Logger;
    ///
    /// let mut registry = ServiceRegistry::new();
    /// registry.register(Scope::Singleton, || Arc::new(Logger));
    ///
    /// assert_eq!(registry.get_scope::<Logger>(), Some(Scope::Singleton));
    /// ```
    pub fn get_scope<T>(&self) -> Option<Scope>
    where
        T: 'static,
    {
        let type_id = TypeId::of::<T>();
        let services = self.services.lock().unwrap();
        services.get(&type_id).map(|entry| entry.scope)
    }
}

impl Default for ServiceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// Clone creates a new registry with shared service definitions
impl Clone for ServiceRegistry {
    fn clone(&self) -> Self {
        Self {
            services: Arc::clone(&self.services),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct TestService {
        value: String,
    }

    #[derive(Clone, Debug)]
    struct Counter {
        count: Arc<Mutex<u32>>,
    }

    #[test]
    fn test_register_and_resolve_singleton() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "test".to_string(),
            })
        });

        let service: Arc<TestService> = registry.resolve().unwrap();
        assert_eq!(service.value, "test");
    }

    #[test]
    fn test_singleton_returns_same_instance() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Singleton, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            Arc::new(Counter {
                count: counter_clone.clone(),
            })
        });

        let _service1: Arc<Counter> = registry.resolve().unwrap();
        let _service2: Arc<Counter> = registry.resolve().unwrap();

        // Factory should only be called once for singleton
        let count = counter.lock().unwrap();
        assert_eq!(*count, 1);
    }

    #[test]
    fn test_transient_creates_new_instance() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Transient, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            Arc::new(Counter {
                count: counter_clone.clone(),
            })
        });

        let _service1: Arc<Counter> = registry.resolve().unwrap();
        let _service2: Arc<Counter> = registry.resolve().unwrap();

        // Factory should be called twice for transient
        let count = counter.lock().unwrap();
        assert_eq!(*count, 2);
    }

    #[test]
    fn test_resolve_unregistered_service() {
        let registry = ServiceRegistry::new();

        let result: ContainerResult<Arc<TestService>> = registry.resolve();
        assert!(result.is_err());

        match result.unwrap_err() {
            ContainerError::ServiceNotFound { .. } => {}
            _ => panic!("Expected ServiceNotFound error"),
        }
    }

    #[test]
    fn test_has_service() {
        let mut registry = ServiceRegistry::new();
        assert!(!registry.has::<TestService>());

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "test".to_string(),
            })
        });

        assert!(registry.has::<TestService>());
    }

    #[test]
    fn test_remove_service() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "test".to_string(),
            })
        });

        assert!(registry.has::<TestService>());
        assert!(registry.remove::<TestService>());
        assert!(!registry.has::<TestService>());
    }

    #[test]
    fn test_clear_services() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "test1".to_string(),
            })
        });

        #[derive(Clone)]
        struct OtherService;
        registry.register(Scope::Singleton, || Arc::new(OtherService));

        registry.clear();

        assert!(!registry.has::<TestService>());
        assert!(!registry.has::<OtherService>());
    }

    #[test]
    fn test_clone_registry() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "shared".to_string(),
            })
        });

        let cloned = registry.clone();

        // Both registries share the same services
        assert!(cloned.has::<TestService>());

        let service: Arc<TestService> = cloned.resolve().unwrap();
        assert_eq!(service.value, "shared");
    }

    #[test]
    fn test_scoped_service_registration() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Scoped, || {
            Arc::new(TestService {
                value: "scoped".to_string(),
            })
        });

        assert!(registry.has::<TestService>());
        assert!(registry.is_scoped::<TestService>());
    }

    #[test]
    fn test_scoped_service_creates_new_instance() {
        let mut registry = ServiceRegistry::new();
        let counter = Arc::new(Mutex::new(0u32));
        let counter_clone = counter.clone();

        registry.register(Scope::Scoped, move || {
            let mut count = counter_clone.lock().unwrap();
            *count += 1;
            Arc::new(Counter {
                count: counter_clone.clone(),
            })
        });

        // Each resolve creates a new instance when called on registry directly
        let _service1: Arc<Counter> = registry.resolve().unwrap();
        let _service2: Arc<Counter> = registry.resolve().unwrap();

        let count = counter.lock().unwrap();
        assert_eq!(*count, 2); // Called twice
    }

    #[test]
    fn test_is_scoped() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "singleton".to_string(),
            })
        });

        #[derive(Clone)]
        struct ScopedService;
        registry.register(Scope::Scoped, || Arc::new(ScopedService));

        assert!(!registry.is_scoped::<TestService>());
        assert!(registry.is_scoped::<ScopedService>());
    }

    #[test]
    fn test_get_scope() {
        let mut registry = ServiceRegistry::new();

        registry.register(Scope::Singleton, || {
            Arc::new(TestService {
                value: "singleton".to_string(),
            })
        });

        #[derive(Clone)]
        struct ScopedService;
        registry.register(Scope::Scoped, || Arc::new(ScopedService));

        #[derive(Clone)]
        struct TransientService;
        registry.register(Scope::Transient, || Arc::new(TransientService));

        assert_eq!(registry.get_scope::<TestService>(), Some(Scope::Singleton));
        assert_eq!(registry.get_scope::<ScopedService>(), Some(Scope::Scoped));
        assert_eq!(
            registry.get_scope::<TransientService>(),
            Some(Scope::Transient)
        );

        #[derive(Clone)]
        struct UnregisteredService;
        assert_eq!(registry.get_scope::<UnregisteredService>(), None);
    }
}
