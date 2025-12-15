//! Queue configuration and backend selection
//!
//! Provides configuration structs and factory functions for creating queue backends.

use crate::error::{QueueError, QueueResult};
use crate::queue::Queue;
use crate::MemoryQueue;
use std::sync::Arc;

#[cfg(feature = "redis-backend")]
use crate::RedisQueue;

/// Queue backend configuration
#[derive(Debug, Clone)]
pub enum QueueConfig {
    /// In-memory queue (for development/testing)
    Memory,

    /// Redis queue (for production)
    #[cfg(feature = "redis-backend")]
    Redis {
        /// Redis connection URL
        url: String,
        /// Queue prefix for namespacing
        prefix: String,
    },
}

impl QueueConfig {
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
    /// - `QUEUE_PREFIX` or defaults to "queue"
    #[cfg(feature = "redis-backend")]
    pub fn redis_from_env() -> Self {
        let url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        let prefix = std::env::var("QUEUE_PREFIX").unwrap_or_else(|_| "queue".to_string());

        Self::Redis { url, prefix }
    }

    /// Create queue backend from configuration
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_queue::{QueueConfig, Queue};
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Memory backend
    /// let queue = QueueConfig::memory().build().await?;
    ///
    /// // Redis backend (requires redis-backend feature)
    /// #[cfg(feature = "redis-backend")]
    /// let queue = QueueConfig::redis("redis://localhost", "myapp").build().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn build(self) -> QueueResult<Arc<dyn Queue>> {
        match self {
            Self::Memory => Ok(Arc::new(MemoryQueue::new())),

            #[cfg(feature = "redis-backend")]
            Self::Redis { url, prefix } => {
                let queue = RedisQueue::new(&url, &prefix).await?;
                Ok(Arc::new(queue))
            }
        }
    }
}

/// Queue configuration builder
///
/// Provides a fluent interface for building queue configurations.
///
/// # Example
///
/// ```no_run
/// use rf_queue::QueueConfigBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let queue = QueueConfigBuilder::new()
///     .backend("redis")
///     .redis_url("redis://localhost:6379")
///     .prefix("myapp")
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct QueueConfigBuilder {
    backend: String,
    redis_url: Option<String>,
    prefix: Option<String>,
}

impl QueueConfigBuilder {
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

    /// Set queue prefix
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Build queue from configuration
    pub async fn build(self) -> QueueResult<Arc<dyn Queue>> {
        let config = match self.backend.as_str() {
            "memory" => QueueConfig::Memory,

            #[cfg(feature = "redis-backend")]
            "redis" => {
                let url = self
                    .redis_url
                    .or_else(|| std::env::var("REDIS_URL").ok())
                    .unwrap_or_else(|| "redis://localhost:6379".to_string());

                let prefix = self
                    .prefix
                    .or_else(|| std::env::var("QUEUE_PREFIX").ok())
                    .unwrap_or_else(|| "queue".to_string());

                QueueConfig::Redis { url, prefix }
            }

            #[cfg(not(feature = "redis-backend"))]
            "redis" => {
                return Err(QueueError::ConfigError(
                    "Redis backend not enabled. Enable 'redis-backend' feature".to_string(),
                ))
            }

            _ => {
                return Err(QueueError::ConfigError(format!(
                    "Unknown backend: {}",
                    self.backend
                )))
            }
        };

        config.build().await
    }
}

impl Default for QueueConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {

    #[cfg(test)]
    #[cfg(feature = "redis-backend")]
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

    #[tokio::test]
    async fn test_memory_config() {
        let queue = QueueConfig::memory().build().await.unwrap();
        assert_eq!(queue.size("default").await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_builder_memory() {
        let queue = QueueConfigBuilder::new()
            .backend("memory")
            .build()
            .await
            .unwrap();
        assert_eq!(queue.size("default").await.unwrap(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "redis-backend")]
    async fn test_redis_config() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_config: Redis not available");
            return;
        }
        let queue = QueueConfig::redis("redis://localhost:6379", "test")
            .build()
            .await
            .unwrap();
        queue.clear("default").await.unwrap();
        assert_eq!(queue.size("default").await.unwrap(), 0);
    }

    #[tokio::test]
    #[cfg(feature = "redis-backend")]
    async fn test_builder_redis() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_builder_redis: Redis not available");
            return;
        }
        let queue = QueueConfigBuilder::new()
            .backend("redis")
            .redis_url("redis://localhost:6379")
            .prefix("test")
            .build()
            .await
            .unwrap();
        queue.clear("default").await.unwrap();
        assert_eq!(queue.size("default").await.unwrap(), 0);
    }
}
