//! # rf-cache: Advanced Caching for RustForge
//!
//! Provides comprehensive caching support with basic and advanced strategies.
//!
//! ## Features
//!
//! - **Basic Caching**: Get, Set, Delete operations
//! - **Cache Tags**: Group related cache entries
//! - **Tag Invalidation**: Flush all entries with a tag
//! - **Stampede Prevention**: Prevent cache stampedes with locking
//! - **TTL Support**: Time-to-live for cache entries
//! - **Memory Backend**: In-memory caching for development
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_cache::*;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), CacheError> {
//! let cache = MemoryCache::new();
//!
//! // Basic operations
//! cache.set("key", &"value".to_string(), Duration::from_secs(60)).await?;
//! let value: Option<String> = cache.get("key").await?;
//! cache.delete("key").await?;
//!
//! // With tags
//! cache.tags(&["users", "user:123"])
//!     .set("user:123:profile", &"data".to_string(), Duration::from_secs(3600))
//!     .await?;
//!
//! // Invalidate by tag
//! cache.tags(&["users"]).flush().await?;
//!
//! // Stampede prevention
//! let value = cache.remember_with_lock("expensive", Duration::from_secs(60), || async {
//!     // Expensive computation
//!     Ok::<_, CacheError>("result".to_string())
//! }).await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use thiserror::Error;
use tokio::sync::{Mutex, RwLock};

pub mod advanced;
pub mod config;
pub mod drivers;

#[cfg(feature = "redis-backend")]
pub mod redis;

/// Redis Pub/Sub for event-driven inter-service communication.
#[cfg(feature = "redis-backend")]
pub mod pubsub;

// Cache facade (Laravel-style static API)
pub mod cache_manager;
pub mod facade;

pub use config::{CacheBackend, CacheConfig, CacheConfigBuilder};

// Re-export facade types (Laravel-style static API)
pub use cache_manager::{CacheManager, GLOBAL_CACHE};
pub use facade::{Cache as CacheFacade, IntoTtl};

// Re-export drivers
#[cfg(feature = "memcached")]
pub use drivers::memcached::{MemcachedDriver, MemcachedOps};

#[cfg(feature = "database")]
pub use drivers::database::DatabaseDriver;

#[cfg(feature = "file")]
pub use drivers::file::FileDriver;

/// Cache errors
#[derive(Debug, Error)]
pub enum CacheError {
    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Cache backend error: {0}")]
    Backend(String),

    #[error("Lock acquisition failed")]
    LockFailed,
}

/// Result type for cache operations
pub type CacheResult<T> = Result<T, CacheError>;

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub sets: u64,
    pub deletes: u64,
}

impl CacheStats {
    /// Cache hit rate (0.0 - 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

/// Cache trait
#[async_trait]
pub trait Cache: Send + Sync {
    /// Get value from cache
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>>;

    /// Set value in cache with TTL
    async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()>;

    /// Delete value from cache
    async fn delete(&self, key: &str) -> CacheResult<()>;

    /// Check if key exists
    async fn exists(&self, key: &str) -> CacheResult<bool>;

    /// Clear all cache entries
    async fn flush(&self) -> CacheResult<()>;

    /// Get or set (remember pattern)
    async fn remember<T, F, Fut>(&self, key: &str, ttl: Duration, f: F) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        let value = f().await?;
        self.set(key, &value, ttl).await?;
        Ok(value)
    }

    /// Get multiple values at once
    async fn get_many<T: DeserializeOwned + Send>(
        &self,
        keys: &[&str],
    ) -> CacheResult<HashMap<String, T>> {
        let mut results = HashMap::new();
        for key in keys {
            if let Ok(Some(value)) = self.get(key).await {
                results.insert(key.to_string(), value);
            }
        }
        Ok(results)
    }

    /// Set multiple values at once
    async fn set_many<T: Serialize + Sync>(
        &self,
        items: &[(&str, &T)],
        ttl: Duration,
    ) -> CacheResult<()> {
        for (key, value) in items {
            self.set(key, value, ttl).await?;
        }
        Ok(())
    }

    /// Increment a numeric value (returns new value)
    async fn increment(&self, key: &str, amount: i64) -> CacheResult<i64> {
        let current: i64 = self.get(key).await?.unwrap_or(0);
        let new_val = current + amount;
        // Default TTL of 1 day; concrete impls can override to preserve original TTL
        self.set(key, &new_val, Duration::from_secs(86400)).await?;
        Ok(new_val)
    }

    /// Decrement a numeric value (returns new value)
    async fn decrement(&self, key: &str, amount: i64) -> CacheResult<i64> {
        self.increment(key, -amount).await
    }

    /// Extend an existing entry's expiration to `now + ttl` without re-reading or
    /// rewriting its value. Returns `true` if the key existed and was touched.
    ///
    /// The default implementation is a no-op returning `Ok(false)`; backends that
    /// support in-place expiry (memory, redis) override it.
    ///
    /// ```rust,no_run
    /// use rf_cache::{Cache, MemoryCache};
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), rf_cache::CacheError> {
    /// let cache = MemoryCache::new();
    /// cache.set("key", &"value", Duration::from_secs(10)).await?;
    /// let touched = cache.touch("key", Duration::from_secs(60)).await?;
    /// assert!(touched);
    /// # Ok(())
    /// # }
    /// ```
    async fn touch(&self, key: &str, ttl: Duration) -> CacheResult<bool> {
        let _ = (key, ttl);
        Ok(false)
    }
}

/// Cache entry with TTL
#[derive(Clone)]
struct CacheEntry {
    data: Vec<u8>,
    expires_at: std::time::Instant,
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: std::time::Instant::now() + ttl,
        }
    }

    fn is_expired(&self) -> bool {
        std::time::Instant::now() > self.expires_at
    }

    /// Returns the remaining TTL; returns zero if already expired
    fn remaining_ttl(&self) -> Duration {
        let now = std::time::Instant::now();
        if self.expires_at > now {
            self.expires_at - now
        } else {
            Duration::ZERO
        }
    }
}

/// In-memory cache implementation
#[derive(Clone)]
pub struct MemoryCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    tags: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
    stats: Arc<RwLock<CacheStats>>,
}

impl MemoryCache {
    /// Create new memory cache with background eviction
    pub fn new() -> Self {
        let entries: Arc<RwLock<HashMap<String, CacheEntry>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let tags: Arc<RwLock<HashMap<String, HashSet<String>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let stats = Arc::new(RwLock::new(CacheStats::default()));

        // Spawn a background task to evict expired entries every 60 seconds.
        // Only runs when called from within a Tokio runtime context.
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let entries_bg = Arc::clone(&entries);
            let tags_bg = Arc::clone(&tags);
            handle.spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    // Remove expired entries
                    let mut entries = entries_bg.write().await;
                    entries.retain(|_, v| !v.is_expired());
                    // Clean up tag sets — remove keys that were evicted
                    let active_keys: HashSet<String> = entries.keys().cloned().collect();
                    drop(entries);
                    let mut tags = tags_bg.write().await;
                    for key_set in tags.values_mut() {
                        key_set.retain(|k| active_keys.contains(k));
                    }
                    tags.retain(|_, v| !v.is_empty());
                }
            });
        }

        Self { entries, tags, locks, stats }
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Reset cache statistics
    pub async fn reset_stats(&self) {
        *self.stats.write().await = CacheStats::default();
    }

    /// Create tagged cache
    pub fn tags(&self, tags: &[&str]) -> TaggedCache {
        TaggedCache {
            cache: self.clone(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Remember with lock (stampede prevention)
    pub async fn remember_with_lock<T, F, Fut>(
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
        // Check cache first
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        // Acquire lock
        let lock = {
            let mut locks = self.locks.lock().await;
            locks
                .entry(key.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(())))
                .clone()
        };

        let _guard = lock.lock().await;

        // Double-check after acquiring lock
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        // Compute and cache
        let value = f().await?;
        self.set(key, &value, ttl).await?;
        Ok(value)
    }

    async fn add_tag(&self, tag: &str, key: &str) {
        let mut tags = self.tags.write().await;
        tags.entry(tag.to_string())
            .or_insert_with(HashSet::new)
            .insert(key.to_string());
    }

    async fn flush_tag(&self, tag: &str) -> CacheResult<()> {
        let keys = {
            let tags = self.tags.read().await;
            tags.get(tag).cloned()
        };

        if let Some(keys) = keys {
            for key in keys {
                self.delete(&key).await?;
            }
        }

        let mut tags = self.tags.write().await;
        tags.remove(tag);

        Ok(())
    }
}

impl Default for MemoryCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Cache for MemoryCache {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        let entries = self.entries.read().await;

        if let Some(entry) = entries.get(key) {
            if entry.is_expired() {
                drop(entries);
                self.delete(key).await?;
                self.stats.write().await.misses += 1;
                return Ok(None);
            }

            let value = serde_json::from_slice(&entry.data)
                .map_err(|e| CacheError::Deserialization(e.to_string()))?;
            self.stats.write().await.hits += 1;
            Ok(Some(value))
        } else {
            self.stats.write().await.misses += 1;
            Ok(None)
        }
    }

    async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        let data =
            serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))?;

        let entry = CacheEntry::new(data, ttl);

        let mut entries = self.entries.write().await;
        entries.insert(key.to_string(), entry);
        self.stats.write().await.sets += 1;

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let mut entries = self.entries.write().await;
        entries.remove(key);
        drop(entries);
        // Remove key from all tag sets to prevent dangling references
        let mut tags = self.tags.write().await;
        for keys in tags.values_mut() {
            keys.remove(key);
        }
        drop(tags);
        self.stats.write().await.deletes += 1;
        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let entries = self.entries.read().await;
        if let Some(entry) = entries.get(key) {
            Ok(!entry.is_expired())
        } else {
            Ok(false)
        }
    }

    async fn flush(&self) -> CacheResult<()> {
        let mut entries = self.entries.write().await;
        entries.clear();
        let mut tags = self.tags.write().await;
        tags.clear();
        Ok(())
    }

    /// Override to preserve the original TTL of an existing entry
    async fn increment(&self, key: &str, amount: i64) -> CacheResult<i64> {
        let remaining = {
            let entries = self.entries.read().await;
            entries
                .get(key)
                .filter(|e| !e.is_expired())
                .map(|e| e.remaining_ttl())
        };
        let current: i64 = self.get(key).await?.unwrap_or(0);
        let new_val = current + amount;
        let ttl = remaining.unwrap_or(Duration::from_secs(86400));
        self.set(key, &new_val, ttl).await?;
        Ok(new_val)
    }

    /// Extend an existing, non-expired entry's expiration to `now + ttl`
    /// without re-reading or rewriting its stored value.
    async fn touch(&self, key: &str, ttl: Duration) -> CacheResult<bool> {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(key) {
            if entry.is_expired() {
                return Ok(false);
            }
            entry.expires_at = std::time::Instant::now() + ttl;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}

/// Tagged cache
pub struct TaggedCache {
    cache: MemoryCache,
    tags: Vec<String>,
}

impl TaggedCache {
    /// Set value with tags
    pub async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        self.cache.set(key, value, ttl).await?;

        // Add tags
        for tag in &self.tags {
            self.cache.add_tag(tag, key).await;
        }

        Ok(())
    }

    /// Get value
    pub async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        self.cache.get(key).await
    }

    /// Flush all entries with these tags
    pub async fn flush(&self) -> CacheResult<()> {
        for tag in &self.tags {
            self.cache.flush_tag(tag).await?;
        }
        Ok(())
    }
}

#[cfg(feature = "redis-backend")]
pub use redis::{RedisCache, RedisTaggedCache};

#[cfg(feature = "redis-backend")]
pub use pubsub::{PubSubMessage, RedisPubSub};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::advanced::{CacheWarmer, MultiLevelCache, ProbabilisticCache};
    pub use crate::config::{CacheBackend, CacheConfig, CacheConfigBuilder};
    pub use crate::{Cache, CacheError, CacheResult, CacheStats, MemoryCache, TaggedCache};

    #[cfg(feature = "redis-backend")]
    pub use crate::{RedisCache, RedisTaggedCache};

    #[cfg(feature = "memcached")]
    pub use crate::{MemcachedDriver, MemcachedOps};

    #[cfg(feature = "database")]
    pub use crate::DatabaseDriver;

    #[cfg(feature = "file")]
    pub use crate::FileDriver;
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Adversarial touch tests (Feature 1 validation) ---

    #[tokio::test]
    async fn test_touch_missing_key_returns_false() {
        let cache = MemoryCache::new();
        assert!(!cache.touch("nope", Duration::from_secs(60)).await.unwrap());
    }

    #[tokio::test]
    async fn test_touch_existing_returns_true_and_value_unchanged() {
        let cache = MemoryCache::new();
        cache
            .set("k", &"original", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(cache.touch("k", Duration::from_secs(120)).await.unwrap());
        // Value must be unchanged and still retrievable.
        let v: Option<String> = cache.get("k").await.unwrap();
        assert_eq!(v, Some("original".to_string()));
    }

    #[tokio::test]
    async fn test_touch_extends_lifetime_past_original_expiry() {
        let cache = MemoryCache::new();
        // Short TTL: would expire in 50ms.
        cache
            .set("k", &"v", Duration::from_millis(50))
            .await
            .unwrap();
        // Extend well past the original expiry.
        assert!(cache.touch("k", Duration::from_secs(10)).await.unwrap());
        // Sleep past the ORIGINAL expiry; key must survive thanks to the extension.
        tokio::time::sleep(Duration::from_millis(120)).await;
        assert!(
            cache.exists("k").await.unwrap(),
            "touch should have extended lifetime past the original 50ms expiry"
        );
        let v: Option<String> = cache.get("k").await.unwrap();
        assert_eq!(v, Some("v".to_string()));
    }

    #[tokio::test]
    async fn test_touch_can_shorten_lifetime() {
        // touch sets expiry to now+ttl absolutely (Laravel semantics), so a shorter
        // ttl than the remaining lifetime moves expiry EARLIER.
        let cache = MemoryCache::new();
        cache.set("k", &"v", Duration::from_secs(60)).await.unwrap();
        assert!(cache.touch("k", Duration::from_millis(30)).await.unwrap());
        tokio::time::sleep(Duration::from_millis(80)).await;
        assert!(
            !cache.exists("k").await.unwrap(),
            "touch with a short ttl should set an absolute (earlier) expiry"
        );
    }

    #[tokio::test]
    async fn test_touch_expired_key_returns_false_and_does_not_resurrect() {
        let cache = MemoryCache::new();
        cache
            .set("k", &"v", Duration::from_millis(20))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Entry is present in the map but expired -> touch must not resurrect it.
        assert!(!cache.touch("k", Duration::from_secs(60)).await.unwrap());
        assert!(!cache.exists("k").await.unwrap());
        let v: Option<String> = cache.get("k").await.unwrap();
        assert_eq!(v, None);
    }

    #[tokio::test]
    async fn test_default_touch_impl_is_noop_false() {
        // Backends that don't override touch must return Ok(false).
        struct Dummy;
        #[async_trait]
        impl Cache for Dummy {
            async fn get<T: DeserializeOwned + Send>(&self, _k: &str) -> CacheResult<Option<T>> {
                Ok(None)
            }
            async fn set<T: Serialize + Sync>(
                &self,
                _k: &str,
                _v: &T,
                _ttl: Duration,
            ) -> CacheResult<()> {
                Ok(())
            }
            async fn delete(&self, _k: &str) -> CacheResult<()> {
                Ok(())
            }
            async fn exists(&self, _k: &str) -> CacheResult<bool> {
                Ok(false)
            }
            async fn flush(&self) -> CacheResult<()> {
                Ok(())
            }
        }
        assert!(!Dummy.touch("k", Duration::from_secs(1)).await.unwrap());
    }

    #[tokio::test]
    async fn test_basic_operations() {
        let cache = MemoryCache::new();

        // Set and get
        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, Some("value1".to_string()));

        // Delete
        cache.delete("key1").await.unwrap();
        let value: Option<String> = cache.get("key1").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let cache = MemoryCache::new();

        cache
            .set("key", &"value", Duration::from_millis(100))
            .await
            .unwrap();

        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));

        tokio::time::sleep(Duration::from_millis(150)).await;

        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_exists() {
        let cache = MemoryCache::new();

        assert!(!cache.exists("key").await.unwrap());

        cache
            .set("key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        assert!(cache.exists("key").await.unwrap());

        cache.delete("key").await.unwrap();
        assert!(!cache.exists("key").await.unwrap());
    }

    #[tokio::test]
    async fn test_touch_extends_ttl() {
        let cache = MemoryCache::new();

        // Set a short TTL that would otherwise expire quickly.
        cache
            .set("key", &"value", Duration::from_millis(100))
            .await
            .unwrap();

        // Touch to extend the expiration well beyond the original TTL.
        let touched = cache.touch("key", Duration::from_secs(60)).await.unwrap();
        assert!(touched);

        // Wait past the original TTL; the entry must still be present.
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(cache.exists("key").await.unwrap());
        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_touch_missing_key() {
        let cache = MemoryCache::new();
        let touched = cache.touch("missing", Duration::from_secs(60)).await.unwrap();
        assert!(!touched);
    }

    #[tokio::test]
    async fn test_touch_expired_key() {
        let cache = MemoryCache::new();
        cache
            .set("key", &"value", Duration::from_millis(50))
            .await
            .unwrap();

        tokio::time::sleep(Duration::from_millis(80)).await;

        // Already expired: touch must report false and not resurrect the entry.
        let touched = cache.touch("key", Duration::from_secs(60)).await.unwrap();
        assert!(!touched);
        assert!(!cache.exists("key").await.unwrap());
    }

    #[tokio::test]
    async fn test_flush() {
        let cache = MemoryCache::new();

        cache
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .set("key2", &"value2", Duration::from_secs(60))
            .await
            .unwrap();

        cache.flush().await.unwrap();

        assert!(!cache.exists("key1").await.unwrap());
        assert!(!cache.exists("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_remember() {
        let cache = MemoryCache::new();

        let value: String = cache
            .remember("key", Duration::from_secs(60), || async {
                Ok("computed".to_string())
            })
            .await
            .unwrap();

        assert_eq!(value, "computed");

        // Second call should use cached value
        let value: String = cache
            .remember("key", Duration::from_secs(60), || async {
                Ok("new_value".to_string())
            })
            .await
            .unwrap();

        assert_eq!(value, "computed");
    }

    #[tokio::test]
    async fn test_tags() {
        let cache = MemoryCache::new();

        cache
            .tags(&["users", "user:1"])
            .set("user:1:profile", &"data1", Duration::from_secs(60))
            .await
            .unwrap();

        cache
            .tags(&["users", "user:2"])
            .set("user:2:profile", &"data2", Duration::from_secs(60))
            .await
            .unwrap();

        // Flush by tag
        cache.tags(&["users"]).flush().await.unwrap();

        let value: Option<String> = cache.get("user:1:profile").await.unwrap();
        assert_eq!(value, None);

        let value: Option<String> = cache.get("user:2:profile").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_remember_with_lock() {
        let cache = MemoryCache::new();

        let value: String = cache
            .remember_with_lock("key", Duration::from_secs(60), || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("computed".to_string())
            })
            .await
            .unwrap();

        assert_eq!(value, "computed");
    }

    #[tokio::test]
    async fn test_concurrent_remember_with_lock() {
        let cache = Arc::new(MemoryCache::new());
        let mut handles = vec![];

        for _ in 0..5 {
            let cache = cache.clone();
            let handle = tokio::spawn(async move {
                cache
                    .remember_with_lock("key", Duration::from_secs(60), || async {
                        tokio::time::sleep(Duration::from_millis(50)).await;
                        Ok::<_, CacheError>("computed".to_string())
                    })
                    .await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.unwrap().unwrap();
            assert_eq!(result, "computed");
        }
    }
}
