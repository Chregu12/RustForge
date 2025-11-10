//! Advanced caching strategies

use crate::{Cache, CacheError, CacheResult, MemoryCache};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Cache warmer for preloading cache
pub struct CacheWarmer {
    cache: MemoryCache,
    tasks: Vec<WarmingTask>,
}

struct WarmingTask {
    key: String,
    ttl: Duration,
    task: Box<dyn Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<Vec<u8>>> + Send>> + Send + Sync>,
}

impl CacheWarmer {
    /// Create new cache warmer
    pub fn new(cache: MemoryCache) -> Self {
        Self {
            cache,
            tasks: Vec::new(),
        }
    }

    /// Add warming task
    pub fn warm<T, F, Fut>(mut self, key: &str, ttl: Duration, f: F) -> Self
    where
        T: Serialize + Send + 'static,
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = CacheResult<T>> + Send + 'static,
    {
        let key = key.to_string();
        let task = Box::new(move || {
            let fut = f();
            Box::pin(async move {
                let value = fut.await?;
                serde_json::to_vec(&value).map_err(|e| CacheError::Serialization(e.to_string()))
            }) as std::pin::Pin<Box<dyn std::future::Future<Output = CacheResult<Vec<u8>>> + Send>>
        });

        self.tasks.push(WarmingTask { key, ttl, task });
        self
    }

    /// Start warming (runs once)
    pub async fn start(self) -> CacheResult<()> {
        for task in self.tasks {
            let data = (task.task)().await?;
            let value: serde_json::Value = serde_json::from_slice(&data)
                .map_err(|e| CacheError::Deserialization(e.to_string()))?;
            self.cache.set(&task.key, &value, task.ttl).await?;
        }
        Ok(())
    }
}

/// Probabilistic early expiration cache wrapper
pub struct ProbabilisticCache {
    cache: MemoryCache,
    beta: f64,
}

impl ProbabilisticCache {
    /// Create new probabilistic cache
    /// beta: probability factor (typically 0.1 = 10% chance)
    pub fn new(cache: MemoryCache, beta: f64) -> Self {
        Self { cache, beta }
    }

    /// Remember with probabilistic early expiration
    pub async fn remember<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        // Simple implementation: use random chance to regenerate
        if self.cache.exists(key).await? {
            let random: f64 = rand::random();
            if random > self.beta {
                if let Some(value) = self.cache.get(key).await? {
                    return Ok(value);
                }
            }
        }

        let value = f().await?;
        self.cache.set(key, &value, ttl).await?;
        Ok(value)
    }
}

/// Multi-level cache (L1 memory, L2 could be Redis)
pub struct MultiLevelCache {
    l1: MemoryCache,
    l2: Option<MemoryCache>, // In real impl, this would be Redis
}

impl MultiLevelCache {
    /// Create new multi-level cache
    pub fn new(l1: MemoryCache, l2: Option<MemoryCache>) -> Self {
        Self { l1, l2 }
    }
}

#[async_trait]
impl Cache for MultiLevelCache {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        // Try L1 first
        if let Some(value) = self.l1.get(key).await? {
            return Ok(Some(value));
        }

        // Try L2 (without populating L1 to avoid trait bound issues)
        if let Some(l2) = &self.l2 {
            return l2.get::<T>(key).await;
        }

        Ok(None)
    }

    async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        // Set in both levels
        self.l1.set(key, value, ttl).await?;
        if let Some(l2) = &self.l2 {
            l2.set(key, value, ttl).await?;
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        self.l1.delete(key).await?;
        if let Some(l2) = &self.l2 {
            l2.delete(key).await?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        if self.l1.exists(key).await? {
            return Ok(true);
        }
        if let Some(l2) = &self.l2 {
            return l2.exists(key).await;
        }
        Ok(false)
    }

    async fn flush(&self) -> CacheResult<()> {
        self.l1.flush().await?;
        if let Some(l2) = &self.l2 {
            l2.flush().await?;
        }
        Ok(())
    }
}

/// Simple random number generator (replace with rand crate in production)
mod rand {
    use std::cell::Cell;

    thread_local! {
        static STATE: Cell<u64> = Cell::new(1);
    }

    pub fn random() -> f64 {
        STATE.with(|state| {
            let mut x = state.get();
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            state.set(x);
            (x as f64) / (u64::MAX as f64)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_multi_level_cache() {
        let l1 = MemoryCache::new();
        let l2 = MemoryCache::new();
        let cache = MultiLevelCache::new(l1.clone(), Some(l2.clone()));

        // Set in multi-level
        cache
            .set("key", &"value", Duration::from_secs(60))
            .await
            .unwrap();

        // Should be in both levels
        assert!(l1.exists("key").await.unwrap());
        assert!(l2.exists("key").await.unwrap());

        // Get from L1
        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_cache_warmer() {
        let cache = MemoryCache::new();

        CacheWarmer::new(cache.clone())
            .warm("key1", Duration::from_secs(60), || async {
                Ok::<_, CacheError>("value1".to_string())
            })
            .warm("key2", Duration::from_secs(60), || async {
                Ok::<_, CacheError>("value2".to_string())
            })
            .start()
            .await
            .unwrap();

        let value1: Option<String> = cache.get("key1").await.unwrap();
        let value2: Option<String> = cache.get("key2").await.unwrap();

        assert_eq!(value1, Some("value1".to_string()));
        assert_eq!(value2, Some("value2".to_string()));
    }

    #[tokio::test]
    async fn test_probabilistic_cache() {
        let cache = MemoryCache::new();
        let prob_cache = ProbabilisticCache::new(cache, 0.5);

        let value: String = prob_cache
            .remember("key", Duration::from_secs(60), || async {
                Ok("computed".to_string())
            })
            .await
            .unwrap();

        assert_eq!(value, "computed");
    }
}
