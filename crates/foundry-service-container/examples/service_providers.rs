use foundry_service_container::{
    async_trait, Container, ProviderRegistry, ServiceProvider, Result,
};
use std::sync::Arc;

// Custom service provider
struct AppServiceProvider;

#[async_trait]
impl ServiceProvider for AppServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        // Register app config
        container
            .singleton("app.name", || Ok("MyApp".to_string()))
            .await?;

        container
            .singleton("app.version", || Ok("1.0.0".to_string()))
            .await?;

        Ok(())
    }

    async fn boot(&self, container: &Container) -> Result<()> {
        let name: Arc<String> = container.resolve("app.name").await?;
        let version: Arc<String> = container.resolve("app.version").await?;
        println!("Booting {} v{}", name, version);
        Ok(())
    }

    fn name(&self) -> &str {
        "AppServiceProvider"
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Create container and provider registry
    let container = Container::new();
    let registry = ProviderRegistry::new();

    // Add providers
    registry.add(Arc::new(AppServiceProvider)).await?;

    // Register all providers
    registry.register_all(&container).await?;

    // Boot all providers
    registry.boot_all(&container).await?;

    // Use registered services
    let name: Arc<String> = container.resolve("app.name").await?;
    println!("Application name: {}", name);

    Ok(())
}
