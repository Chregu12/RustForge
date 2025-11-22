//! Cache configuration and backend selection
//!
//! Provides configuration structs and factory functions for creating cache backends.

use crate::{CacheError, MemoryCache};

#[cfg(feature = "redis-backend")]
use crate::RedisCache;

/// Cache result type
pub type CacheConfigResult<T> = Result<T, CacheError>;

/// Cache backend enum
///
/// Wraps concrete cache implementations.
#[derive(Clone)]
pub enum CacheBackend {
    /// In-memory cache
    Memory(MemoryCache),

    /// Redis cache
    #[cfg(feature = "redis-backend")]
    Redis(RedisCache),
}

/// Cache backend configuration
#[derive(Debug, Clone)]
pub enum CacheConfig {
    /// In-memory cache (for development/testing)
    Memory,

    /// Redis cache (for production)
    #[cfg(feature = "redis-backend")]
    Redis {
        /// Redis connection URL
        url: String,
        /// Cache key prefix for namespacing
        prefix: String,
    },
}

impl CacheConfig {
    /// Create memory backend configuration
    pub fn memory() -> Self {
        Self::Memory
    }

    /// Create Redis backend configuration
    #[cfg(feature = "redis-backend")]
    pub fn redis(url: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self::Redis {
            url: url.into(),
            prefix: prefix.into(),
        }
    }

    /// Create Redis backend from environment variables
    ///
    /// Looks for:
    /// - `REDIS_URL` or defaults to "redis://localhost:6379"
    /// - `CACHE_PREFIX` or defaults to "cache"
    #[cfg(feature = "redis-backend")]
    pub fn redis_from_env() -> Self {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let prefix = std::env::var("CACHE_PREFIX").unwrap_or_else(|_| "cache".to_string());

        Self::Redis { url, prefix }
    }

    /// Create cache backend from configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_cache::CacheConfig;
    ///
    /// # async fn example() -> Result<(), rf_cache::CacheError> {
    /// // Memory backend
    /// let cache = CacheConfig::memory().build().await?;
    ///
    /// // Redis backend (requires redis-backend feature)
    /// #[cfg(feature = "redis-backend")]
    /// let cache = CacheConfig::redis("redis://localhost", "myapp").build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> CacheConfigResult<CacheBackend> {
        match self {
            Self::Memory => Ok(CacheBackend::Memory(MemoryCache::new())),

            #[cfg(feature = "redis-backend")]
            Self::Redis { url, prefix } => {
                let cache = RedisCache::new(&url, &prefix).await?;
                Ok(CacheBackend::Redis(cache))
            }
        }
    }
}

/// Cache configuration builder
///
/// Provides a fluent interface for building cache configurations.
///
/// # Example
///
/// ```no_run
/// use rf_cache::CacheConfigBuilder;
///
/// # async fn example() -> Result<(), rf_cache::CacheError> {
/// let cache = CacheConfigBuilder::new()
///     .backend("redis")
///     .redis_url("redis://localhost:6379")
///     .prefix("myapp")
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct CacheConfigBuilder {
    backend: String,
    redis_url: Option<String>,
    prefix: Option<String>,
}

impl CacheConfigBuilder {
    /// Create new builder
    pub fn new() -> Self {
        Self {
            backend: "memory".to_string(),
            redis_url: None,
            prefix: None,
        }
    }

    /// Set backend type ("memory" or "redis")
    pub fn backend(mut self, backend: impl Into<String>) -> Self {
        self.backend = backend.into();
        self
    }

    /// Set Redis URL
    pub fn redis_url(mut self, url: impl Into<String>) -> Self {
        self.redis_url = Some(url.into());
        self
    }

    /// Set cache prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Build cache from configuration
    pub async fn build(self) -> CacheConfigResult<CacheBackend> {
        let config = match self.backend.as_str() {
            "memory" => CacheConfig::Memory,

            #[cfg(feature = "redis-backend")]
            "redis" => {
                let url = self
                    .redis_url
                    .or_else(|| std::env::var("REDIS_URL").ok())
                    .unwrap_or_else(|| "redis://localhost:6379".to_string());

                let prefix = self
                    .prefix
                    .or_else(|| std::env::var("CACHE_PREFIX").ok())
                    .unwrap_or_else(|| "cache".to_string());

                CacheConfig::Redis { url, prefix }
            }

            #[cfg(not(feature = "redis-backend"))]
            "redis" => {
                return Err(CacheError::Backend(
                    "Redis backend not enabled. Enable 'redis-backend' feature".to_string(),
                ))
            }

            _ => {
                return Err(CacheError::Backend(format!(
                    "Unknown backend: {}",
                    self.backend
                )))
            }
        };

        config.build().await
    }
}

impl Default for CacheConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    async fn redis_available() -> bool {
        use redis::AsyncCommands;
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        match redis::Client::open(redis_url.as_str()) {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(mut conn) => conn.ping::<_, String>().await.is_ok(),
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    use super::*;
    use crate::Cache;
    use std::time::Duration;

    #[tokio::test]
    async fn test_memory_config() {
        let cache_backend = CacheConfig::memory().build().await.unwrap();
        let cache = match cache_backend {
            CacheBackend::Memory(ref c) => c,
            #[cfg(feature = "redis-backend")]
            _ => panic!("Expected memory backend"),
        };
        cache
            .set("key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_builder_memory() {
        let cache_backend = CacheConfigBuilder::new()
            .backend("memory")
            .build()
            .await
            .unwrap();
        let cache = match cache_backend {
            CacheBackend::Memory(ref c) => c,
            #[cfg(feature = "redis-backend")]
            _ => panic!("Expected memory backend"),
        };
        cache
            .set("key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    #[cfg(feature = "redis-backend")]
    async fn test_redis_config() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_config: Redis not available");
            return;
        }
        let cache_backend = CacheConfig::redis("redis://localhost:6379", "test")
            .build()
            .await
            .unwrap();
        let cache = match cache_backend {
            CacheBackend::Redis(ref c) => c,
            _ => panic!("Expected redis backend"),
        };
        cache.flush().await.unwrap();
        cache
            .set("key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    #[cfg(feature = "redis-backend")]
    async fn test_builder_redis() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_builder_redis: Redis not available");
            return;
        }
        let cache_backend = CacheConfigBuilder::new()
            .backend("redis")
            .redis_url("redis://localhost:6379")
            .prefix("test")
            .build()
            .await
            .unwrap();
        let cache = match cache_backend {
            CacheBackend::Redis(ref c) => c,
            _ => panic!("Expected redis backend"),
        };
        cache.flush().await.unwrap();
        cache
            .set("key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }
}
