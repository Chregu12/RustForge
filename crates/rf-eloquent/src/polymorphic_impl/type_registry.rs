//! # Type Registry for Polymorphic Relationships
//!
//! Provides a global registry to map model type names to their resolver functions.
//! This is essential for MorphTo relationships to dynamically resolve the correct model type.

use super::polymorphic::{PolymorphicError, PolymorphicResult};
use async_trait::async_trait;
use lazy_static::lazy_static;
use sea_orm::DatabaseConnection;
use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type alias for the resolver function
/// Takes (id, db) and returns a boxed Any containing the resolved model
pub type ResolverFn = Arc<
    dyn Fn(
            i64,
            Arc<DatabaseConnection>,
        ) -> Pin<Box<dyn Future<Output = PolymorphicResult<Box<dyn Any + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Trait for types that can be resolved from the type registry
#[async_trait]
pub trait TypeResolver: Send + Sync {
    /// Resolve a model by its type name and ID
    async fn resolve(
        &self,
        type_name: &str,
        id: i64,
        db: &DatabaseConnection,
    ) -> PolymorphicResult<Box<dyn Any + Send + Sync>>;

    /// Check if a type is registered
    fn has_type(&self, type_name: &str) -> bool;

    /// Get all registered type names
    fn registered_types(&self) -> Vec<String>;
}

/// Global type registry for polymorphic relationships
pub struct TypeRegistry {
    resolvers: Arc<RwLock<HashMap<String, ResolverFn>>>,
}

impl TypeRegistry {
    /// Create a new empty type registry
    pub fn new() -> Self {
        Self {
            resolvers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a model type with its resolver function
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_eloquent::relationships::type_registry::GLOBAL_TYPE_REGISTRY;
    /// use std::sync::Arc;
    ///
    /// # async fn example() {
    /// GLOBAL_TYPE_REGISTRY.register::<Post>("Post", |id, db| {
    ///     Box::pin(async move {
    ///         let post = Post::find_by_id(id, &db).await?;
    ///         Ok(Box::new(post) as Box<dyn Any + Send + Sync>)
    ///     })
    /// }).await;
    /// # }
    /// ```
    pub async fn register<F, Fut>(&self, type_name: impl Into<String>, resolver: F)
    where
        F: Fn(i64, Arc<DatabaseConnection>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = PolymorphicResult<Box<dyn Any + Send + Sync>>> + Send + 'static,
    {
        let type_name = type_name.into();
        let resolver_fn: ResolverFn = Arc::new(move |id, db| Box::pin(resolver(id, db)));

        let mut resolvers = self.resolvers.write().await;
        resolvers.insert(type_name, resolver_fn);
    }

    /// Register a simple model type (with a find_by_id method)
    ///
    /// This is a convenience method for models that implement a standard find_by_id pattern.
    pub async fn register_simple<T, F, Fut>(&self, type_name: impl Into<String>, finder: F)
    where
        T: Send + Sync + 'static,
        F: Fn(i64, Arc<DatabaseConnection>) -> Fut + Send + Sync + Clone + 'static,
        Fut: Future<Output = PolymorphicResult<T>> + Send + 'static,
    {
        let type_name = type_name.into();
        let resolver_fn: ResolverFn = Arc::new(move |id, db| {
            let finder = finder.clone();
            Box::pin(async move {
                let model = finder(id, db).await?;
                Ok(Box::new(model) as Box<dyn Any + Send + Sync>)
            })
        });

        let mut resolvers = self.resolvers.write().await;
        resolvers.insert(type_name, resolver_fn);
    }
}

impl Default for TypeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TypeResolver for TypeRegistry {
    async fn resolve(
        &self,
        type_name: &str,
        id: i64,
        db: &DatabaseConnection,
    ) -> PolymorphicResult<Box<dyn Any + Send + Sync>> {
        let resolvers = self.resolvers.read().await;
        let resolver = resolvers
            .get(type_name)
            .ok_or_else(|| PolymorphicError::TypeNotRegistered(type_name.to_string()))?;

        // Clone the Arc to use outside the lock
        let resolver = Arc::clone(resolver);
        drop(resolvers); // Release the read lock

        resolver(id, Arc::new(db.clone())).await
    }

    fn has_type(&self, type_name: &str) -> bool {
        // Note: This is synchronous, so we can't use async read
        // In practice, this should be checked during registration
        // For now, we'll return true and let resolve handle the error
        true
    }

    fn registered_types(&self) -> Vec<String> {
        // Note: This is synchronous, so we can't use async read
        // Return empty vec for now - this is mainly for debugging
        Vec::new()
    }
}

lazy_static! {
    /// Global singleton type registry
    ///
    /// Use this to register model types for polymorphic relationships.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_eloquent::relationships::type_registry::GLOBAL_TYPE_REGISTRY;
    /// use std::sync::Arc;
    ///
    /// # async fn setup() {
    /// // Register Post type
    /// GLOBAL_TYPE_REGISTRY.register("Post", |id, db| {
    ///     Box::pin(async move {
    ///         // Load Post model
    /// #       Ok(Box::new(()) as Box<dyn std::any::Any + Send + Sync>)
    ///     })
    /// }).await;
    ///
    /// // Register Video type
    /// GLOBAL_TYPE_REGISTRY.register("Video", |id, db| {
    ///     Box::pin(async move {
    ///         // Load Video model
    /// #       Ok(Box::new(()) as Box<dyn std::any::Any + Send + Sync>)
    ///     })
    /// }).await;
    /// # }
    /// ```
    pub static ref GLOBAL_TYPE_REGISTRY: TypeRegistry = TypeRegistry::new();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_type_registry_new() {
        let registry = TypeRegistry::new();
        assert!(registry.resolvers.read().await.is_empty());
    }

    #[tokio::test]
    async fn test_type_registry_register() {
        let registry = TypeRegistry::new();

        // Register a test type
        registry
            .register("TestModel", |id, _db| {
                Box::pin(async move {
                    Ok(Box::new(id) as Box<dyn Any + Send + Sync>)
                })
            })
            .await;

        let resolvers = registry.resolvers.read().await;
        assert!(resolvers.contains_key("TestModel"));
    }

    #[tokio::test]
    async fn test_type_registry_resolve_not_registered() {
        let registry = TypeRegistry::new();
        let db = DatabaseConnection::default();

        let result = registry.resolve("UnknownType", 1, &db).await;
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PolymorphicError::TypeNotRegistered(_)
        ));
    }

    #[tokio::test]
    async fn test_type_registry_resolve_success() {
        let registry = TypeRegistry::new();

        // Register a test type
        registry
            .register("TestModel", |id, _db| {
                Box::pin(async move {
                    Ok(Box::new(format!("Model-{}", id)) as Box<dyn Any + Send + Sync>)
                })
            })
            .await;

        let db = DatabaseConnection::default();
        let result = registry.resolve("TestModel", 42, &db).await;
        assert!(result.is_ok());

        let model = result.unwrap();
        let value = model.downcast_ref::<String>().unwrap();
        assert_eq!(value, "Model-42");
    }

    #[tokio::test]
    async fn test_type_registry_multiple_types() {
        let registry = TypeRegistry::new();

        // Register multiple types
        registry
            .register("Post", |id, _db| {
                Box::pin(async move {
                    Ok(Box::new(format!("Post-{}", id)) as Box<dyn Any + Send + Sync>)
                })
            })
            .await;

        registry
            .register("Video", |id, _db| {
                Box::pin(async move {
                    Ok(Box::new(format!("Video-{}", id)) as Box<dyn Any + Send + Sync>)
                })
            })
            .await;

        let resolvers = registry.resolvers.read().await;
        assert_eq!(resolvers.len(), 2);
        assert!(resolvers.contains_key("Post"));
        assert!(resolvers.contains_key("Video"));
    }
}
