use async_trait::async_trait;
use crate::container::Container;
use crate::error::Result;
use crate::provider::ServiceProvider;

/// Application service provider for core application services
pub struct ApplicationServiceProvider;

impl ApplicationServiceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceProvider for ApplicationServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        // Register application key service
        container
            .singleton("app.key", || {
                Ok(std::env::var("APP_KEY").unwrap_or_else(|_| String::new()))
            })
            .await?;

        // Register application environment
        container
            .singleton("app.env", || {
                Ok(std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string()))
            })
            .await?;

        // Register application debug mode
        container
            .singleton("app.debug", || {
                Ok(std::env::var("APP_DEBUG")
                    .unwrap_or_else(|_| "true".to_string())
                    .parse::<bool>()
                    .unwrap_or(true))
            })
            .await?;

        // Register application name
        container
            .singleton("app.name", || {
                Ok(std::env::var("APP_NAME").unwrap_or_else(|_| "Foundry".to_string()))
            })
            .await?;

        // Register application URL
        container
            .singleton("app.url", || {
                Ok(std::env::var("APP_URL")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string()))
            })
            .await?;

        Ok(())
    }

    async fn boot(&self, _container: &Container) -> Result<()> {
        // Validate APP_KEY is set in production
        let env = std::env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
        let key = std::env::var("APP_KEY").unwrap_or_default();

        if env == "production" && key.is_empty() {
            tracing::warn!("APP_KEY is not set in production environment!");
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "ApplicationServiceProvider"
    }
}

impl Default for ApplicationServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
