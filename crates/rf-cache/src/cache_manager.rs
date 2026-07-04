//! Global cache manager

use once_cell::sync::Lazy;
use crate::{Cache as CacheTrait, CacheResult, MemoryCache};
use serde::{de::DeserializeOwned, Serialize};
use std::sync::RwLock;
use std::time::Duration;

/// Global cache manager instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_CACHE: Lazy<RwLock<CacheManager>> = Lazy::new(|| {
    RwLock::new(CacheManager::new())
});

/// Cache manager that holds the cache backend.
///
/// The default (and always-available) driver is a process-global in-memory
/// [`MemoryCache`] with real, `Instant`-based TTL expiry. The manager owns a
/// private Tokio runtime purely to drive the async backend behind the
/// synchronous facade API; it is never exposed to callers.
pub struct CacheManager {
    backend: MemoryCache,
    runtime: tokio::runtime::Runtime,
}

impl CacheManager {
    /// Create a new cache manager
    pub fn new() -> Self {
        Self {
            backend: MemoryCache::new(),
            runtime: tokio::runtime::Runtime::new().expect("Failed to create tokio runtime"),
        }
    }

    /// Drive a backend future to completion synchronously.
    ///
    /// The facade is a blocking API, but it is frequently called from *inside*
    /// an async request handler. Calling `Runtime::block_on` while already on a
    /// Tokio worker thread panics with "Cannot start a runtime from within a
    /// runtime". To stay correct in both contexts we detect an active runtime
    /// and, if present, drive the future on a short-lived scoped thread that has
    /// no ambient runtime — avoiding both the panic and any sync-over-async
    /// deadlock. Outside a runtime we block directly on our own runtime.
    fn run<F>(&self, fut: F) -> F::Output
    where
        F: std::future::Future + Send,
        F::Output: Send,
    {
        if tokio::runtime::Handle::try_current().is_ok() {
            std::thread::scope(|s| {
                s.spawn(|| self.runtime.block_on(fut))
                    .join()
                    .expect("cache backend thread panicked")
            })
        } else {
            self.runtime.block_on(fut)
        }
    }

    /// Get a value from cache
    pub fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        self.run(self.backend.get(key))
    }

    /// Put a value in cache
    pub fn put<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        self.run(self.backend.set(key, value, ttl))
    }

    /// Store a value forever (very long TTL)
    pub fn forever<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
    ) -> CacheResult<()> {
        // Use a very long TTL (1 year)
        self.run(self.backend.set(key, value, Duration::from_secs(365 * 24 * 3600)))
    }

    /// Remove a value from cache
    pub fn forget(&self, key: &str) -> CacheResult<()> {
        self.run(self.backend.delete(key))
    }

    /// Check if a key exists in cache
    pub fn has(&self, key: &str) -> CacheResult<bool> {
        self.run(self.backend.exists(key))
    }

    /// Extend an existing entry's expiration to `now + ttl` without re-reading
    /// or rewriting its value. Returns `true` if the key existed and was touched.
    pub fn touch(&self, key: &str, ttl: Duration) -> CacheResult<bool> {
        self.run(self.backend.touch(key, ttl))
    }

    /// Flush all cache entries
    pub fn flush(&self) -> CacheResult<()> {
        self.run(self.backend.flush())
    }

    /// Remember pattern: get from cache or compute and store
    pub fn remember<T, F, Fut>(
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
        self.run(self.backend.remember(key, ttl, f))
    }

    /// Remember forever: get from cache or compute and store forever
    pub fn remember_forever<T, F, Fut>(
        &self,
        key: &str,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        self.remember(key, Duration::from_secs(365 * 24 * 3600), f)
    }

    /// Pull: get and delete a value from cache
    pub fn pull<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        if let Some(value) = self.get(key)? {
            self.forget(key)?;
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Add: store only if key doesn't exist
    pub fn add<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<bool> {
        if !self.has(key)? {
            self.put(key, value, ttl)?;
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
    pub fn tags(&self, tags: &[&str]) -> crate::TaggedCache {
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

    #[test]
    fn test_cache_manager_new() {
        let manager = CacheManager::new();
        assert!(!manager.has("test").unwrap());
    }

    #[test]
    fn test_cache_manager_put_get() {
        let manager = CacheManager::new();

        manager.put("key", &"value", Duration::from_secs(60)).unwrap();

        let value: Option<String> = manager.get("key").unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[test]
    fn test_cache_manager_forget() {
        let manager = CacheManager::new();

        manager.put("key", &"value", Duration::from_secs(60)).unwrap();
        assert!(manager.has("key").unwrap());

        manager.forget("key").unwrap();
        assert!(!manager.has("key").unwrap());
    }

    #[test]
    fn test_cache_manager_pull() {
        let manager = CacheManager::new();

        manager.put("key", &"value", Duration::from_secs(60)).unwrap();

        let value: Option<String> = manager.pull("key").unwrap();
        assert_eq!(value, Some("value".to_string()));

        // Should be removed after pull
        assert!(!manager.has("key").unwrap());
    }

    #[test]
    fn test_cache_manager_add() {
        let manager = CacheManager::new();

        // First add should succeed
        let added = manager.add("key", &"value1", Duration::from_secs(60)).unwrap();
        assert!(added);

        // Second add should fail
        let added = manager.add("key", &"value2", Duration::from_secs(60)).unwrap();
        assert!(!added);

        // Value should still be the first one
        let value: Option<String> = manager.get("key").unwrap();
        assert_eq!(value, Some("value1".to_string()));
    }
}
