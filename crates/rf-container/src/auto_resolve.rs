//! Auto-resolution for dependency injection
//!
//! Provides automatic dependency resolution with constructor injection.
//! Types can implement the `Resolvable` trait to enable automatic resolution
//! of their dependencies from the container.
//!
//! # Features
//!
//! - Automatic constructor injection
//! - Circular dependency detection
//! - Support for all lifecycle scopes (Singleton, Scoped, Transient)
//! - Thread-safe resolution
//!
//! # Example
//!
//! ```rust
//! use rf_container::{ServiceRegistry, Scope, Resolvable, ContainerError};
//! use std::sync::Arc;
//!
//! #[derive(Clone)]
//! struct Database;
//! #[derive(Clone)]
//! struct Cache;
//!
//! #[derive(Clone)]
//! struct UserRepository {
//!     db: Arc<Database>,
//!     cache: Arc<Cache>,
//! }
//!
//! impl Resolvable for Database {
//!     fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
//!         Ok(Database)
//!     }
//! }
//!
//! impl Resolvable for Cache {
//!     fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
//!         Ok(Cache)
//!     }
//! }
//!
//! impl Resolvable for UserRepository {
//!     fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
//!         let db = registry.resolve::<Database>()?;
//!         let cache = registry.resolve::<Cache>()?;
//!         Ok(UserRepository { db, cache })
//!     }
//! }
//!
//! let mut registry = ServiceRegistry::new();
//!
//! // Register dependencies manually
//! registry.register(Scope::Singleton, || Arc::new(Database));
//! registry.register(Scope::Singleton, || Arc::new(Cache));
//!
//! // Auto-resolve with dependencies injected
//! let repo = UserRepository::resolve(&registry).unwrap();
//! ```

use crate::{ContainerError, ContainerResult, Scope, ServiceRegistry};
use std::any::TypeId;
use std::sync::{Arc, Mutex};

/// Trait for types that can be automatically resolved from the container
///
/// Types implementing this trait can be automatically constructed with
/// their dependencies resolved from the service registry.
///
/// # Example
///
/// ```rust
/// use rf_container::{Resolvable, ServiceRegistry, ContainerError};
/// use std::sync::Arc;
///
/// struct Config {
///     database_url: String,
/// }
///
/// impl Resolvable for Config {
///     fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
///         Ok(Config {
///             database_url: "postgres://localhost".to_string(),
///         })
///     }
/// }
///
/// struct DatabasePool {
///     config: Arc<Config>,
/// }
///
/// impl Resolvable for DatabasePool {
///     fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
///         let config = registry.resolve::<Config>()?;
///         Ok(DatabasePool { config })
///     }
/// }
/// ```
pub trait Resolvable: Send + Sync + 'static {
    /// Resolve this type from the service registry
    ///
    /// This method should construct an instance of `Self` by resolving
    /// all required dependencies from the provided registry.
    ///
    /// # Parameters
    ///
    /// - `registry`: Service registry to resolve dependencies from
    ///
    /// # Errors
    ///
    /// - `ServiceNotFound`: Required dependency not registered
    /// - `CircularDependency`: Circular dependency detected
    /// - `DowncastFailed`: Type mismatch during resolution
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{Resolvable, ServiceRegistry, ContainerError};
    /// # struct Logger;
    /// impl Resolvable for Logger {
    ///     fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
    ///         Ok(Logger)
    ///     }
    /// }
    /// ```
    fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError>
    where
        Self: Sized;
}

/// Auto-resolver for handling dependency resolution with circular dependency detection
///
/// This is used internally by the container to track the resolution stack
/// and detect circular dependencies.
pub struct AutoResolver {
    /// Resolution stack for circular dependency detection
    resolution_stack: Mutex<Vec<TypeId>>,
}

impl AutoResolver {
    /// Create a new auto-resolver
    pub fn new() -> Self {
        Self {
            resolution_stack: Mutex::new(Vec::new()),
        }
    }

    /// Resolve a type with circular dependency detection
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type to resolve (must implement `Resolvable`)
    ///
    /// # Parameters
    ///
    /// - `registry`: Service registry to resolve from
    ///
    /// # Errors
    ///
    /// - `CircularDependency`: If `T` is already being resolved (circular dependency)
    /// - Other errors from the `Resolvable::resolve` implementation
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, AutoResolver, Resolvable, ContainerError};
    /// # struct Database;
    /// # impl Resolvable for Database {
    /// #     fn resolve(_: &ServiceRegistry) -> Result<Self, ContainerError> { Ok(Database) }
    /// # }
    /// let registry = ServiceRegistry::new();
    /// let resolver = AutoResolver::new();
    ///
    /// let db = resolver.resolve::<Database>(&registry).unwrap();
    /// ```
    pub fn resolve<T: Resolvable>(&self, registry: &ServiceRegistry) -> ContainerResult<Arc<T>> {
        let type_id = TypeId::of::<T>();
        let type_name = std::any::type_name::<T>();

        // Check for circular dependency
        {
            let stack = self.resolution_stack.lock().unwrap();
            if stack.contains(&type_id) {
                return Err(ContainerError::CircularDependency {
                    type_name: type_name.to_string(),
                });
            }
        }

        // Add to resolution stack
        {
            let mut stack = self.resolution_stack.lock().unwrap();
            stack.push(type_id);
        }

        // Resolve the type
        let result = T::resolve(registry);

        // Remove from resolution stack
        {
            let mut stack = self.resolution_stack.lock().unwrap();
            stack.pop();
        }

        result.map(Arc::new)
    }

    /// Check if a type is currently being resolved (for circular dependency detection)
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type to check
    ///
    /// # Returns
    ///
    /// `true` if the type is in the resolution stack, `false` otherwise
    pub fn is_resolving<T: 'static>(&self) -> bool {
        let type_id = TypeId::of::<T>();
        let stack = self.resolution_stack.lock().unwrap();
        stack.contains(&type_id)
    }

    /// Get the current resolution depth (number of types in the resolution stack)
    pub fn resolution_depth(&self) -> usize {
        let stack = self.resolution_stack.lock().unwrap();
        stack.len()
    }

    /// Clear the resolution stack (mainly for testing)
    pub fn clear(&self) {
        let mut stack = self.resolution_stack.lock().unwrap();
        stack.clear();
    }
}

impl Default for AutoResolver {
    fn default() -> Self {
        Self::new()
    }
}

// Extension trait for ServiceRegistry to support auto-binding
impl ServiceRegistry {
    /// Bind a resolvable type with default (Singleton) scope
    ///
    /// This is a convenience method for registering types that implement `Resolvable`.
    /// The type will be registered as a Singleton by default.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type to bind (must implement `Resolvable`)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, Resolvable, ContainerError};
    /// # struct Database;
    /// # impl Resolvable for Database {
    /// #     fn resolve(_: &ServiceRegistry) -> Result<Self, ContainerError> { Ok(Database) }
    /// # }
    /// let mut registry = ServiceRegistry::new();
    /// registry.bind::<Database>();
    ///
    /// let db = registry.resolve::<Database>().unwrap();
    /// ```
    pub fn bind<T: Resolvable>(&mut self) {
        self.bind_with_scope::<T>(Scope::Singleton);
    }

    /// Bind a resolvable type with a specific scope
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type to bind (must implement `Resolvable`)
    ///
    /// # Parameters
    ///
    /// - `scope`: Lifecycle scope for the service
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, Scope, Resolvable, ContainerError};
    /// # struct RequestLogger;
    /// # impl Resolvable for RequestLogger {
    /// #     fn resolve(_: &ServiceRegistry) -> Result<Self, ContainerError> { Ok(RequestLogger) }
    /// # }
    /// let mut registry = ServiceRegistry::new();
    /// registry.bind_with_scope::<RequestLogger>(Scope::Scoped);
    ///
    /// let logger = registry.resolve::<RequestLogger>().unwrap();
    /// ```
    pub fn bind_with_scope<T: Resolvable>(&mut self, scope: Scope) {
        let registry_clone = self.clone();
        self.register(scope, move || {
            match T::resolve(&registry_clone) {
                Ok(instance) => Arc::new(instance),
                Err(e) => {
                    panic!("Failed to resolve type during factory call: {}", e);
                }
            }
        });
    }

    /// Bind a resolvable type as a transient service
    ///
    /// Creates a new instance on every resolution.
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type to bind (must implement `Resolvable`)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, Resolvable, ContainerError};
    /// # struct TempFile;
    /// # impl Resolvable for TempFile {
    /// #     fn resolve(_: &ServiceRegistry) -> Result<Self, ContainerError> { Ok(TempFile) }
    /// # }
    /// let mut registry = ServiceRegistry::new();
    /// registry.bind_transient::<TempFile>();
    ///
    /// let file1 = registry.resolve::<TempFile>().unwrap();
    /// let file2 = registry.resolve::<TempFile>().unwrap();
    /// // file1 and file2 are different instances
    /// ```
    pub fn bind_transient<T: Resolvable>(&mut self) {
        self.bind_with_scope::<T>(Scope::Transient);
    }

    /// Bind a resolvable type as a scoped service
    ///
    /// Creates one instance per scope (e.g., per HTTP request).
    ///
    /// # Type Parameters
    ///
    /// - `T`: Type to bind (must implement `Resolvable`)
    ///
    /// # Example
    ///
    /// ```rust
    /// # use rf_container::{ServiceRegistry, Resolvable, ContainerError};
    /// # struct RequestContext;
    /// # impl Resolvable for RequestContext {
    /// #     fn resolve(_: &ServiceRegistry) -> Result<Self, ContainerError> { Ok(RequestContext) }
    /// # }
    /// let mut registry = ServiceRegistry::new();
    /// registry.bind_scoped::<RequestContext>();
    /// ```
    pub fn bind_scoped<T: Resolvable>(&mut self) {
        self.bind_with_scope::<T>(Scope::Scoped);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct Database {
        connection_string: String,
    }

    impl Resolvable for Database {
        fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
            Ok(Database {
                connection_string: "postgres://localhost".to_string(),
            })
        }
    }

    #[derive(Debug, Clone)]
    struct Cache {
        host: String,
    }

    impl Resolvable for Cache {
        fn resolve(_registry: &ServiceRegistry) -> Result<Self, ContainerError> {
            Ok(Cache {
                host: "redis://localhost".to_string(),
            })
        }
    }

    #[derive(Clone)]
    struct UserRepository {
        db: Arc<Database>,
        cache: Arc<Cache>,
    }

    impl Resolvable for UserRepository {
        fn resolve(registry: &ServiceRegistry) -> Result<Self, ContainerError> {
            let db = registry.resolve::<Database>()?;
            let cache = registry.resolve::<Cache>()?;
            Ok(UserRepository { db, cache })
        }
    }

    #[test]
    fn test_auto_resolver_creates_instance() {
        let registry = ServiceRegistry::new();
        let resolver = AutoResolver::new();

        let db = resolver.resolve::<Database>(&registry).unwrap();
        assert_eq!(db.connection_string, "postgres://localhost");
    }

    #[test]
    fn test_auto_resolver_resolution_depth() {
        let resolver = AutoResolver::new();
        assert_eq!(resolver.resolution_depth(), 0);

        // Simulate adding to stack
        {
            let mut stack = resolver.resolution_stack.lock().unwrap();
            stack.push(TypeId::of::<Database>());
        }
        assert_eq!(resolver.resolution_depth(), 1);

        resolver.clear();
        assert_eq!(resolver.resolution_depth(), 0);
    }

    #[test]
    fn test_auto_resolver_is_resolving() {
        let resolver = AutoResolver::new();
        assert!(!resolver.is_resolving::<Database>());

        {
            let mut stack = resolver.resolution_stack.lock().unwrap();
            stack.push(TypeId::of::<Database>());
        }
        assert!(resolver.is_resolving::<Database>());

        resolver.clear();
        assert!(!resolver.is_resolving::<Database>());
    }

    #[test]
    fn test_resolvable_trait_basic() {
        let registry = ServiceRegistry::new();
        let db = Database::resolve(&registry).unwrap();
        assert_eq!(db.connection_string, "postgres://localhost");
    }

    #[test]
    fn test_resolvable_trait_with_dependencies() {
        let mut registry = ServiceRegistry::new();

        // Register dependencies first
        registry.register(Scope::Singleton, || {
            Arc::new(Database {
                connection_string: "postgres://localhost".to_string(),
            })
        });

        registry.register(Scope::Singleton, || {
            Arc::new(Cache {
                host: "redis://localhost".to_string(),
            })
        });

        // Resolve with dependencies
        let repo = UserRepository::resolve(&registry).unwrap();
        assert_eq!(repo.db.connection_string, "postgres://localhost");
        assert_eq!(repo.cache.host, "redis://localhost");
    }
}
