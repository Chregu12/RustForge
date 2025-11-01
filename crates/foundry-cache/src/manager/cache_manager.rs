use crate::manager::config::{CacheConfig, CacheDriver};
use crate::store::{CacheError, CacheStats, CacheStore, CacheValue};
use crate::stores::{FileStore, MemoryStore, RedisStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// High-level cache manager
pub struct CacheManager {
    store: Arc<dyn CacheStore>,
    config: CacheConfig,
}

impl CacheManager {
    pub async fn new(config: CacheConfig) -> Result<Self, CacheError> {
        let store: Arc<dyn CacheStore> = match config.driver {
            CacheDriver::Redis => {
                let redis_config = config.redis.as_ref()
                    .ok_or_else(|| CacheError::Other("Redis config required".to_string()))?;
                Arc::new(RedisStore::with_prefix(&redis_config.url, &config.prefix)?)
            }
            CacheDriver::File => {
                let file_config = config.file.as_ref()
                    .ok_or_else(|| CacheError::Other("File config required".to_string()))?;
                Arc::new(FileStore::new(&file_config.path).await?)
            }
            CacheDriver::Memory => {
                let memory_config = config.memory.as_ref()
                    .ok_or_else(|| CacheError::Other("Memory config required".to_string()))?;
                Arc::new(MemoryStore::with_config(
                    memory_config.max_capacity,
                    config.default_ttl,
                ))
            }
        };

        Ok(Self { store, config })
    }

    pub fn from_env() -> Result<Self, CacheError> {
        let config = CacheConfig::from_env();

        // Since we can't use async in this method, we'll use blocking
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::new(config))
    }

    /// Get a value from cache
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: for<'de> Deserialize<'de>,
    {
        if let Some(value) = self.store.get(key).await? {
            Ok(Some(value.to_json()?))
        } else {
            Ok(None)
        }
    }

    /// Get a string value from cache
    pub async fn get_string(&self, key: &str) -> Result<Option<String>, CacheError> {
        if let Some(value) = self.store.get(key).await? {
            Ok(Some(value.to_string()?))
        } else {
            Ok(None)
        }
    }

    /// Set a value in cache
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let cache_value = CacheValue::from_json(value)?;
        self.store.set(key, cache_value, ttl).await
    }

    /// Set a string value in cache
    pub async fn set_string(&self, key: &str, value: impl Into<String>, ttl: Option<Duration>) -> Result<(), CacheError> {
        let cache_value = CacheValue::from_string(value);
        self.store.set(key, cache_value, ttl).await
    }

    /// Set with default TTL
    pub async fn put<T>(&self, key: &str, value: &T) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        self.set(key, value, Some(self.config.default_ttl)).await
    }

    /// Get or set (if not exists)
    pub async fn remember<T, F>(&self, key: &str, ttl: Duration, f: F) -> Result<T, CacheError>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        F: FnOnce() -> T,
    {
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        let value = f();
        self.set(key, &value, Some(ttl)).await?;
        Ok(value)
    }

    /// Get or set async
    pub async fn remember_async<T, F, Fut>(&self, key: &str, ttl: Duration, f: F) -> Result<T, CacheError>
    where
        T: Serialize + for<'de> Deserialize<'de>,
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = T>,
    {
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        let value = f().await;
        self.set(key, &value, Some(ttl)).await?;
        Ok(value)
    }

    /// Delete a key
    pub async fn forget(&self, key: &str) -> Result<bool, CacheError> {
        self.store.delete(key).await
    }

    /// Check if key exists
    pub async fn has(&self, key: &str) -> Result<bool, CacheError> {
        self.store.exists(key).await
    }

    /// Clear all cache
    pub async fn flush(&self) -> Result<(), CacheError> {
        self.store.flush().await
    }

    /// Increment a counter
    pub async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        self.store.increment(key, amount).await
    }

    /// Decrement a counter
    pub async fn decrement(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        self.store.decrement(key, amount).await
    }

    /// Get cache statistics
    pub async fn stats(&self) -> Result<CacheStats, CacheError> {
        self.store.stats().await
    }

    /// Get many values at once
    pub async fn get_many<T>(&self, keys: &[String]) -> Result<Vec<Option<T>>, CacheError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let values = self.store.get_many(keys).await?;
        values
            .into_iter()
            .map(|opt| {
                opt.map(|v| v.to_json())
                    .transpose()
            })
            .collect()
    }

    /// Set many values at once
    pub async fn set_many<T>(&self, items: Vec<(String, T, Option<Duration>)>) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let cache_items: Result<Vec<_>, _> = items
            .into_iter()
            .map(|(key, value, ttl)| {
                CacheValue::from_json(&value).map(|v| (key, v, ttl))
            })
            .collect();

        self.store.set_many(cache_items?).await
    }

    /// Pull (get and delete)
    pub async fn pull<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let value = self.get(key).await?;
        if value.is_some() {
            self.forget(key).await?;
        }
        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestData {
        name: String,
        age: u32,
    }

    #[tokio::test]
    async fn test_cache_manager_memory() {
        let config = CacheConfig {
            driver: CacheDriver::Memory,
            default_ttl: Duration::from_secs(60),
            prefix: "test:".to_string(),
            redis: None,
            file: None,
            memory: Some(crate::manager::config::MemoryConfig {
                max_capacity: 1000,
            }),
        };

        let manager = CacheManager::new(config).await.unwrap();

        // Test set/get
        let data = TestData {
            name: "John".to_string(),
            age: 30,
        };

        manager.set("user:1", &data, None).await.unwrap();
        let result: TestData = manager.get("user:1").await.unwrap().unwrap();
        assert_eq!(result, data);

        // Test forget
        assert!(manager.forget("user:1").await.unwrap());
        assert!(manager.get::<TestData>("user:1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_cache_manager_remember() {
        let config = CacheConfig {
            driver: CacheDriver::Memory,
            default_ttl: Duration::from_secs(60),
            prefix: "test:".to_string(),
            redis: None,
            file: None,
            memory: Some(crate::manager::config::MemoryConfig {
                max_capacity: 1000,
            }),
        };

        let manager = CacheManager::new(config).await.unwrap();

        let value = manager
            .remember("counter", Duration::from_secs(60), || 42)
            .await
            .unwrap();

        assert_eq!(value, 42);

        // Should return cached value
        let value = manager
            .remember("counter", Duration::from_secs(60), || 100)
            .await
            .unwrap();

        assert_eq!(value, 42); // Still the cached value
    }

    #[tokio::test]
    async fn test_cache_manager_increment() {
        let config = CacheConfig {
            driver: CacheDriver::Memory,
            default_ttl: Duration::from_secs(60),
            prefix: "test:".to_string(),
            redis: None,
            file: None,
            memory: Some(crate::manager::config::MemoryConfig {
                max_capacity: 1000,
            }),
        };

        let manager = CacheManager::new(config).await.unwrap();

        let result = manager.increment("views", 1).await.unwrap();
        assert_eq!(result, 1);

        let result = manager.increment("views", 5).await.unwrap();
        assert_eq!(result, 6);
    }
}
