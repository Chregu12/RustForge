//! Global cache manager

use once_cell::sync::Lazy;
use rf_cache::{Cache as CacheTrait, CacheError, CacheResult, MemoryCache};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Global cache manager instance
pub static GLOBAL_CACHE: Lazy<Arc<RwLock<CacheManager>>> = Lazy::new(|| {
    Arc::new(RwLock::new(CacheManager::new()))
});

/// Cache manager that holds the cache backend
pub struct CacheManager {
    backend: MemoryCache,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            backend: MemoryCache::new(),
        }
    }

    /// Get a value from cache
    pub async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        self.backend.get(key).await
    }

    /// Put a value in cache
    pub async fn put<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        self.backend.set(key, value, ttl).await
    }

    /// Store a value forever (very long TTL)
    pub async fn forever<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> CacheResult<()> {
        // Use a very long TTL (1 year)
        self.backend.set(key, value, Duration::from_secs(365 * 24 * 3600)).await
    }

    /// Remove a value from cache
    pub async fn forget(&self, key: &str) -> CacheResult<()> {
        self.backend.delete(key).await
    }

    /// Check if a key exists in cache
    pub async fn has(&self, key: &str) -> CacheResult<bool> {
        self.backend.exists(key).await
    }

    /// Flush all cache entries
    pub async fn flush(&self) -> CacheResult<()> {
        self.backend.flush().await
    }

    /// Remember pattern: get from cache or compute and store
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
        self.backend.remember(key, ttl, f).await
    }

    /// Remember forever: get from cache or compute and store forever
    pub async fn remember_forever<T, F, Fut>(
        &self,
        key: &str,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        self.remember(key, Duration::from_secs(365 * 24 * 3600), f).await
    }

    /// Pull: get and delete a value from cache
    pub async fn pull<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        if let Some(value) = self.get(key).await? {
            self.forget(key).await?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Add: store only if key doesn't exist
    pub async fn add<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<bool> {
        if !self.has(key).await? {
            self.put(key, value, ttl).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get the underlying cache backend
    pub fn backend(&self) -> &MemoryCache {
        &self.backend
    }

    /// Create a tagged cache
    pub fn tags(&self, tags: &[&str]) -> rf_cache::TaggedCache {
        self.backend.tags(tags)
    }
}

impl Default for CacheManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_manager_new() {
        let manager = CacheManager::new();
        assert!(!manager.has("test").await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_manager_put_get() {
        let manager = CacheManager::new();

        manager.put("key", &"value", Duration::from_secs(60)).await.unwrap();

        let value: Option<String> = manager.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_cache_manager_forget() {
        let manager = CacheManager::new();

        manager.put("key", &"value", Duration::from_secs(60)).await.unwrap();
        assert!(manager.has("key").await.unwrap());

        manager.forget("key").await.unwrap();
        assert!(!manager.has("key").await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_manager_pull() {
        let manager = CacheManager::new();

        manager.put("key", &"value", Duration::from_secs(60)).await.unwrap();

        let value: Option<String> = manager.pull("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));

        // Should be removed after pull
        assert!(!manager.has("key").await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_manager_add() {
        let manager = CacheManager::new();

        // First add should succeed
        let added = manager.add("key", &"value1", Duration::from_secs(60)).await.unwrap();
        assert!(added);

        // Second add should fail
        let added = manager.add("key", &"value2", Duration::from_secs(60)).await.unwrap();
        assert!(!added);

        // Value should still be the first one
        let value: Option<String> = manager.get("key").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }
}
