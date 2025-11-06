use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::container::Container;
use crate::error::{ContainerError, Result};

/// Service provider trait - similar to Laravel's ServiceProvider
#[async_trait]
pub trait ServiceProvider: Send + Sync {
    /// Register services in the container
    async fn register(&self, container: &Container) -> Result<()>;

    /// Bootstrap services (called after all providers are registered)
    async fn boot(&self, _container: &Container) -> Result<()> {
        Ok(())
    }

    /// Return list of services that should be deferred (lazy loaded)
    fn defer(&self) -> Vec<String> {
        vec![]
    }

    /// Provider name for debugging
    fn name(&self) -> &str {
        std::any::type_name::<Self>()
    }

    /// Return list of providers this provider depends on
    fn dependencies(&self) -> Vec<String> {
        vec![]
    }
}

/// Registry for managing service providers
#[derive(Clone)]
pub struct ProviderRegistry {
    providers: Arc<RwLock<Vec<Arc<dyn ServiceProvider>>>>,
    registered: Arc<RwLock<HashMap<String, bool>>>,
    booted: Arc<RwLock<HashMap<String, bool>>>,
}

impl ProviderRegistry {
    /// Create a new provider registry
    pub fn new() -> Self {
        Self {
            providers: Arc::new(RwLock::new(Vec::new())),
            registered: Arc::new(RwLock::new(HashMap::new())),
            booted: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a provider to the registry
    pub async fn add(&self, provider: Arc<dyn ServiceProvider>) -> Result<()> {
        let mut providers = self.providers.write().await;
        providers.push(provider);
        Ok(())
    }

    /// Register all providers with the container
    pub async fn register_all(&self, container: &Container) -> Result<()> {
        let providers = self.providers.read().await;

        for provider in providers.iter() {
            let name = provider.name().to_string();

            // Check if already registered
            {
                let registered = self.registered.read().await;
                if registered.get(&name).copied().unwrap_or(false) {
                    continue;
                }
            }

            // Register the provider
            provider.register(container).await.map_err(|e| {
                ContainerError::ProviderError(format!(
                    "Failed to register provider {}: {}",
                    name, e
                ))
            })?;

            // Mark as registered
            let mut registered = self.registered.write().await;
            registered.insert(name.clone(), true);

            // Handle deferred services
            for service in provider.defer() {
                container.defer(service).await?;
            }
        }

        Ok(())
    }

    /// Boot all providers
    pub async fn boot_all(&self, container: &Container) -> Result<()> {
        let providers = self.providers.read().await;

        for provider in providers.iter() {
            let name = provider.name().to_string();

            // Check if already booted
            {
                let booted = self.booted.read().await;
                if booted.get(&name).copied().unwrap_or(false) {
                    continue;
                }
            }

            // Boot the provider
            provider.boot(container).await.map_err(|e| {
                ContainerError::ProviderError(format!(
                    "Failed to boot provider {}: {}",
                    name, e
                ))
            })?;

            // Mark as booted
            let mut booted = self.booted.write().await;
            booted.insert(name, true);
        }

        Ok(())
    }

    /// Get all registered provider names
    pub async fn registered_providers(&self) -> Vec<String> {
        let registered = self.registered.read().await;
        registered.keys().cloned().collect()
    }

    /// Get all booted provider names
    pub async fn booted_providers(&self) -> Vec<String> {
        let booted = self.booted.read().await;
        booted.keys().cloned().collect()
    }

    /// Check if a provider is registered
    pub async fn is_registered(&self, name: &str) -> bool {
        let registered = self.registered.read().await;
        registered.get(name).copied().unwrap_or(false)
    }

    /// Check if a provider is booted
    pub async fn is_booted(&self, name: &str) -> bool {
        let booted = self.booted.read().await;
        booted.get(name).copied().unwrap_or(false)
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider;

    #[async_trait]
    impl ServiceProvider for TestProvider {
        async fn register(&self, container: &Container) -> Result<()> {
            container
                .singleton("test_service", || Ok("test".to_string()))
                .await
        }

        fn name(&self) -> &str {
            "TestProvider"
        }
    }

    #[tokio::test]
    async fn test_provider_registration() {
        let container = Container::new();
        let registry = ProviderRegistry::new();

        registry.add(Arc::new(TestProvider)).await.unwrap();
        registry.register_all(&container).await.unwrap();

        assert!(registry.is_registered("TestProvider").await);
        assert!(container.has("test_service").await);
    }

    #[tokio::test]
    async fn test_provider_boot() {
        let container = Container::new();
        let registry = ProviderRegistry::new();

        registry.add(Arc::new(TestProvider)).await.unwrap();
        registry.register_all(&container).await.unwrap();
        registry.boot_all(&container).await.unwrap();

        assert!(registry.is_booted("TestProvider").await);
    }
}
