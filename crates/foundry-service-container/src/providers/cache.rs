use async_trait::async_trait;
use crate::container::Container;
use crate::error::Result;
use crate::provider::ServiceProvider;

/// Cache service provider for caching services
pub struct CacheServiceProvider;

impl CacheServiceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceProvider for CacheServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        // Register cache driver
        container
            .singleton("cache.driver", || {
                Ok(std::env::var("CACHE_DRIVER").unwrap_or_else(|_| "memory".to_string()))
            })
            .await?;

        // Register Redis connection
        container
            .singleton("cache.redis.host", || {
                Ok(std::env::var("REDIS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()))
            })
            .await?;

        container
            .singleton("cache.redis.port", || {
                Ok(std::env::var("REDIS_PORT")
                    .unwrap_or_else(|_| "6379".to_string())
                    .parse::<u16>()
                    .unwrap_or(6379))
            })
            .await?;

        container
            .singleton("cache.redis.password", || {
                Ok(std::env::var("REDIS_PASSWORD").ok())
            })
            .await?;

        container
            .singleton("cache.redis.db", || {
                Ok(std::env::var("REDIS_DB")
                    .unwrap_or_else(|_| "0".to_string())
                    .parse::<u8>()
                    .unwrap_or(0))
            })
            .await?;

        // Register cache TTL
        container
            .singleton("cache.ttl", || {
                Ok(std::env::var("CACHE_TTL")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse::<u64>()
                    .unwrap_or(3600))
            })
            .await?;

        // Register cache prefix
        container
            .singleton("cache.prefix", || {
                Ok(std::env::var("CACHE_PREFIX")
                    .unwrap_or_else(|_| {
                        std::env::var("APP_NAME")
                            .unwrap_or_else(|_| "foundry".to_string())
                            .to_lowercase()
                    }))
            })
            .await?;

        Ok(())
    }

    async fn boot(&self, _container: &Container) -> Result<()> {
        // Could initialize cache connections here
        Ok(())
    }

    fn name(&self) -> &str {
        "CacheServiceProvider"
    }

    fn defer(&self) -> Vec<String> {
        // Cache services can be lazy loaded
        vec![
            "cache.driver".to_string(),
            "cache.redis.host".to_string(),
            "cache.redis.port".to_string(),
            "cache.redis.password".to_string(),
            "cache.redis.db".to_string(),
            "cache.ttl".to_string(),
            "cache.prefix".to_string(),
        ]
    }
}

impl Default for CacheServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
