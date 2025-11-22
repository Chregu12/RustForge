//! Application container and service management

use crate::ServiceProvider;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

type BoxedService = Box<dyn Any + Send + Sync>;
type Factory = Arc<dyn Fn() -> BoxedService + Send + Sync>;

/// Service container for dependency injection
#[derive(Clone)]
pub struct Container {
    bindings: Arc<RwLock<HashMap<String, Factory>>>,
    singletons: Arc<RwLock<HashMap<String, Arc<BoxedService>>>>,
    type_bindings: Arc<RwLock<HashMap<TypeId, Factory>>>,
    type_singletons: Arc<RwLock<HashMap<TypeId, Arc<BoxedService>>>>,
}

impl Container {
    /// Create a new container
    pub fn new() -> Self {
        Self {
            bindings: Arc::new(RwLock::new(HashMap::new())),
            singletons: Arc::new(RwLock::new(HashMap::new())),
            type_bindings: Arc::new(RwLock::new(HashMap::new())),
            type_singletons: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Bind a service to the container
    pub async fn bind<F>(&self, key: impl Into<String>, factory: F)
    where
        F: Fn() -> BoxedService + Send + Sync + 'static,
    {
        let mut bindings = self.bindings.write().await;
        bindings.insert(key.into(), Arc::new(factory));
    }

    /// Bind a singleton to the container
    pub async fn singleton<F>(&self, key: impl Into<String>, factory: F)
    where
        F: Fn() -> BoxedService + Send + Sync + 'static,
    {
        let key = key.into();
        let service = Arc::new(factory());
        let mut singletons = self.singletons.write().await;
        singletons.insert(key, service);
    }

    /// Bind a service by type
    pub async fn bind_type<T, F>(&self, factory: F)
    where
        T: Any + Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        let wrapped_factory = Arc::new(move || Box::new(factory()) as BoxedService);
        let mut bindings = self.type_bindings.write().await;
        bindings.insert(TypeId::of::<T>(), wrapped_factory);
    }

    /// Bind a singleton by type
    pub async fn singleton_type<T, F>(&self, factory: F)
    where
        T: Any + Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        let service = Arc::new(Box::new(factory()) as BoxedService);
        let mut singletons = self.type_singletons.write().await;
        singletons.insert(TypeId::of::<T>(), service);
    }

    /// Resolve a service from the container
    pub async fn make(&self, key: &str) -> Option<Arc<BoxedService>> {
        // Check singletons first
        {
            let singletons = self.singletons.read().await;
            if let Some(service) = singletons.get(key) {
                return Some(Arc::clone(service));
            }
        }

        // Check bindings
        let bindings = self.bindings.read().await;
        if let Some(factory) = bindings.get(key) {
            return Some(Arc::new(factory()));
        }

        None
    }

    /// Resolve a service by type
    pub async fn make_type<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        let type_id = TypeId::of::<T>();

        // Check singletons first
        {
            let singletons = self.type_singletons.read().await;
            if let Some(service) = singletons.get(&type_id) {
                let boxed = Arc::clone(service);
                if let Some(concrete) =
                    Arc::downcast::<T>(unsafe { Arc::from_raw(Arc::into_raw(boxed) as *const T) })
                        .ok()
                {
                    return Some(concrete);
                }
            }
        }

        // Check bindings
        let bindings = self.type_bindings.read().await;
        if let Some(factory) = bindings.get(&type_id) {
            let boxed = factory();
            if let Ok(concrete) = boxed.downcast::<T>() {
                return Some(Arc::new(*concrete));
            }
        }

        None
    }

    /// Check if a binding exists
    pub async fn has(&self, key: &str) -> bool {
        let singletons = self.singletons.read().await;
        if singletons.contains_key(key) {
            return true;
        }

        let bindings = self.bindings.read().await;
        bindings.contains_key(key)
    }
}

impl Default for Container {
    fn default() -> Self {
        Self::new()
    }
}

/// Application instance
pub struct Application {
    container: Container,
    providers: Vec<Box<dyn ServiceProvider>>,
    booted: bool,
}

impl Application {
    /// Create a new application
    pub fn new() -> Self {
        Self {
            container: Container::new(),
            providers: Vec::new(),
            booted: false,
        }
    }

    /// Register a service provider
    pub fn provider<P: ServiceProvider + 'static>(mut self, provider: P) -> Self {
        self.providers.push(Box::new(provider));
        self
    }

    /// Get the container
    pub fn container(&self) -> &Container {
        &self.container
    }

    /// Bind a service
    pub async fn bind<F>(&mut self, key: impl Into<String>, factory: F)
    where
        F: Fn() -> BoxedService + Send + Sync + 'static,
    {
        self.container.bind(key, factory).await;
    }

    /// Bind a singleton
    pub async fn singleton<F>(&mut self, key: impl Into<String>, factory: F)
    where
        F: Fn() -> BoxedService + Send + Sync + 'static,
    {
        self.container.singleton(key, factory).await;
    }

    /// Resolve a service
    pub async fn make(&self, key: &str) -> Option<Arc<BoxedService>> {
        self.container.make(key).await
    }

    /// Boot the application and all providers
    pub async fn boot(mut self) -> anyhow::Result<Self> {
        if self.booted {
            return Ok(self);
        }

        // Take ownership of providers temporarily
        let providers = std::mem::take(&mut self.providers);

        // Register all providers
        for provider in &providers {
            provider.register(&mut self).await?;
        }

        // Restore providers
        self.providers = providers;

        // Boot all providers
        for provider in &self.providers {
            provider.boot(&self).await?;
        }

        self.booted = true;
        Ok(self)
    }

    /// Check if the application has been booted
    pub fn is_booted(&self) -> bool {
        self.booted
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_container_bind() {
        let container = Container::new();
        container.bind("test", || Box::new(42i32)).await;

        let service = container.make("test").await.unwrap();
        let value = service.downcast_ref::<i32>().unwrap();
        assert_eq!(*value, 42);
    }

    #[tokio::test]
    async fn test_container_singleton() {
        let container = Container::new();
        container.singleton("counter", || Box::new(0i32)).await;

        let service1 = container.make("counter").await.unwrap();
        let service2 = container.make("counter").await.unwrap();

        // Should be the same instance
        assert!(Arc::ptr_eq(&service1, &service2));
    }

    #[tokio::test]
    async fn test_container_has() {
        let container = Container::new();
        assert!(!container.has("test").await);

        container.bind("test", || Box::new(42i32)).await;
        assert!(container.has("test").await);
    }

    #[tokio::test]
    async fn test_application_boot() {
        use async_trait::async_trait;

        struct TestProvider;

        #[async_trait]
        impl ServiceProvider for TestProvider {
            async fn register(&self, app: &mut Application) -> anyhow::Result<()> {
                app.bind("test", || Box::new(42i32)).await;
                Ok(())
            }
        }

        let app = Application::new()
            .provider(TestProvider)
            .boot()
            .await
            .unwrap();

        assert!(app.is_booted());
        assert!(app.container().has("test").await);
    }
}
