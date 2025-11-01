use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::binding::{Binding, Factory};
use crate::context::ContextualBindingStore;
use crate::error::{ContainerError, Result};

/// The main service container for dependency injection
#[derive(Clone)]
pub struct Container {
    bindings: Arc<RwLock<HashMap<String, Binding>>>,
    aliases: Arc<RwLock<HashMap<String, String>>>,
    tags: Arc<RwLock<HashMap<String, Vec<String>>>>,
    contextual: Arc<RwLock<ContextualBindingStore>>,
    deferred: Arc<RwLock<Vec<String>>>,
}

impl Container {
    /// Create a new container instance
    pub fn new() -> Self {
        Self {
            bindings: Arc::new(RwLock::new(HashMap::new())),
            aliases: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
            contextual: Arc::new(RwLock::new(ContextualBindingStore::new())),
            deferred: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Bind a transient service (new instance each time)
    pub async fn bind<T: Send + Sync + 'static>(
        &self,
        key: impl Into<String>,
        factory: impl Fn() -> Result<T> + Send + Sync + 'static,
    ) -> Result<()> {
        let key = key.into();
        let wrapped_factory: Factory = Arc::new(move || {
            let instance = factory()?;
            Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
        });

        let binding = Binding::transient(wrapped_factory);
        let mut bindings = self.bindings.write().await;
        bindings.insert(key, binding);
        Ok(())
    }

    /// Bind a singleton service (single shared instance)
    pub async fn singleton<T: Send + Sync + 'static>(
        &self,
        key: impl Into<String>,
        factory: impl Fn() -> Result<T> + Send + Sync + 'static,
    ) -> Result<()> {
        let key = key.into();
        let wrapped_factory: Factory = Arc::new(move || {
            let instance = factory()?;
            Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
        });

        let binding = Binding::singleton(wrapped_factory);
        let mut bindings = self.bindings.write().await;
        bindings.insert(key, binding);
        Ok(())
    }

    /// Bind a factory closure
    pub async fn factory<T: Send + Sync + 'static>(
        &self,
        key: impl Into<String>,
        factory: impl Fn() -> Result<T> + Send + Sync + 'static,
    ) -> Result<()> {
        let key = key.into();
        let wrapped_factory: Factory = Arc::new(move || {
            let instance = factory()?;
            Ok(Arc::new(instance) as Arc<dyn Any + Send + Sync>)
        });

        let binding = Binding::factory(wrapped_factory);
        let mut bindings = self.bindings.write().await;
        bindings.insert(key, binding);
        Ok(())
    }

    /// Bind an existing instance as a singleton
    pub async fn instance<T: Send + Sync + 'static>(
        &self,
        key: impl Into<String>,
        instance: T,
    ) -> Result<()> {
        let key = key.into();
        let arc_instance = Arc::new(instance);
        let wrapped_factory: Factory = Arc::new(move || {
            Ok(arc_instance.clone() as Arc<dyn Any + Send + Sync>)
        });

        let binding = Binding::singleton(wrapped_factory);
        let mut bindings = self.bindings.write().await;
        bindings.insert(key, binding);
        Ok(())
    }

    /// Resolve a service by key
    pub async fn resolve<T: Send + Sync + 'static>(&self, key: impl AsRef<str>) -> Result<Arc<T>> {
        let key = key.as_ref();

        // Check for alias
        let actual_key = {
            let aliases = self.aliases.read().await;
            aliases.get(key).cloned().unwrap_or_else(|| key.to_string())
        };

        let bindings = self.bindings.read().await;
        let binding = bindings
            .get(&actual_key)
            .ok_or_else(|| ContainerError::ServiceNotFound(actual_key.clone()))?;

        let instance = binding.resolve().await?;

        instance
            .downcast::<T>()
            .map_err(|_| ContainerError::TypeMismatch(actual_key.clone()))
    }

    /// Get a service by type (convenience method)
    pub async fn get<T: Send + Sync + 'static>(&self) -> Result<Arc<T>> {
        let type_name = std::any::type_name::<T>();
        self.resolve(type_name).await
    }

    /// Check if a service exists
    pub async fn has(&self, key: impl AsRef<str>) -> bool {
        let key = key.as_ref();
        let bindings = self.bindings.read().await;
        bindings.contains_key(key)
    }

    /// Make a service with custom parameters
    pub async fn make<T: Send + Sync + 'static>(
        &self,
        key: impl AsRef<str>,
        _params: HashMap<String, serde_json::Value>,
    ) -> Result<Arc<T>> {
        // For now, just resolve normally
        // In a more advanced implementation, params would be passed to the factory
        self.resolve(key).await
    }

    /// Tag services for grouped resolution
    pub async fn tag(&self, tags: Vec<String>, service_keys: Vec<String>) -> Result<()> {
        let mut tags_map = self.tags.write().await;
        for tag in tags {
            tags_map
                .entry(tag)
                .or_insert_with(Vec::new)
                .extend(service_keys.clone());
        }
        Ok(())
    }

    /// Get all services with a specific tag
    pub async fn tagged(&self, tag: impl AsRef<str>) -> Result<Vec<String>> {
        let tags = self.tags.read().await;
        Ok(tags
            .get(tag.as_ref())
            .cloned()
            .unwrap_or_default())
    }

    /// Create an alias for a service
    pub async fn alias(&self, alias: impl Into<String>, original: impl Into<String>) -> Result<()> {
        let mut aliases = self.aliases.write().await;
        aliases.insert(alias.into(), original.into());
        Ok(())
    }

    /// Mark a service as deferred (lazy loaded)
    pub async fn defer(&self, key: impl Into<String>) -> Result<()> {
        let mut deferred = self.deferred.write().await;
        deferred.push(key.into());
        Ok(())
    }

    /// Check if a service is deferred
    pub async fn is_deferred(&self, key: impl AsRef<str>) -> bool {
        let deferred = self.deferred.read().await;
        deferred.contains(&key.as_ref().to_string())
    }

    /// Flush all bindings (useful for testing)
    pub async fn flush(&self) {
        let mut bindings = self.bindings.write().await;
        bindings.clear();
        let mut aliases = self.aliases.write().await;
        aliases.clear();
        let mut tags = self.tags.write().await;
        tags.clear();
        let mut deferred = self.deferred.write().await;
        deferred.clear();
    }

    /// Get all registered service keys
    pub async fn keys(&self) -> Vec<String> {
        let bindings = self.bindings.read().await;
        bindings.keys().cloned().collect()
    }

    /// Extend container with another container's bindings
    pub async fn extend(&self, other: &Container) {
        let other_bindings = other.bindings.read().await;
        let mut bindings = self.bindings.write().await;

        for (key, binding) in other_bindings.iter() {
            // Create a new binding with the same factory
            let new_binding = match binding.binding_type {
                crate::binding::BindingType::Transient => {
                    Binding::transient(binding.factory.clone())
                }
                crate::binding::BindingType::Singleton => {
                    Binding::singleton(binding.factory.clone())
                }
                crate::binding::BindingType::Factory => {
                    Binding::factory(binding.factory.clone())
                }
            };
            bindings.insert(key.clone(), new_binding.with_tags(binding.tags.clone()));
        }
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestService {
        value: String,
    }

    #[tokio::test]
    async fn test_bind_and_resolve() {
        let container = Container::new();

        container
            .bind("test", || Ok(TestService {
                value: "test".to_string(),
            }))
            .await
            .unwrap();

        let service: Arc<TestService> = container.resolve("test").await.unwrap();
        assert_eq!(service.value, "test");
    }

    #[tokio::test]
    async fn test_singleton() {
        let container = Container::new();

        container
            .singleton("counter", || Ok(TestService {
                value: "singleton".to_string(),
            }))
            .await
            .unwrap();

        let service1: Arc<TestService> = container.resolve("counter").await.unwrap();
        let service2: Arc<TestService> = container.resolve("counter").await.unwrap();

        assert_eq!(service1.value, service2.value);
        assert!(Arc::ptr_eq(&service1, &service2));
    }

    #[tokio::test]
    async fn test_instance() {
        let container = Container::new();

        let instance = TestService {
            value: "instance".to_string(),
        };

        container.instance("test", instance).await.unwrap();

        let service: Arc<TestService> = container.resolve("test").await.unwrap();
        assert_eq!(service.value, "instance");
    }

    #[tokio::test]
    async fn test_has() {
        let container = Container::new();

        container
            .bind("exists", || Ok(TestService {
                value: "test".to_string(),
            }))
            .await
            .unwrap();

        assert!(container.has("exists").await);
        assert!(!container.has("not_exists").await);
    }

    #[tokio::test]
    async fn test_alias() {
        let container = Container::new();

        container
            .bind("original", || Ok(TestService {
                value: "test".to_string(),
            }))
            .await
            .unwrap();

        container.alias("alias", "original").await.unwrap();

        let service: Arc<TestService> = container.resolve("alias").await.unwrap();
        assert_eq!(service.value, "test");
    }

    #[tokio::test]
    async fn test_tags() {
        let container = Container::new();

        container
            .tag(vec!["cache".to_string()], vec!["redis".to_string(), "memcached".to_string()])
            .await
            .unwrap();

        let tagged = container.tagged("cache").await.unwrap();
        assert_eq!(tagged.len(), 2);
        assert!(tagged.contains(&"redis".to_string()));
        assert!(tagged.contains(&"memcached".to_string()));
    }

    #[tokio::test]
    async fn test_defer() {
        let container = Container::new();

        container.defer("lazy_service").await.unwrap();

        assert!(container.is_deferred("lazy_service").await);
        assert!(!container.is_deferred("regular_service").await);
    }
}
