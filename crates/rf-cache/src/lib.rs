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

    /// Extend the TTL of an existing key without changing its value.
    ///
    /// Returns `true` if the key existed (and was not expired), `false`
    /// otherwise.  The default implementation re-serialises and re-stores the
    /// value; backends that support a native TTL-refresh operation (e.g. Redis
    /// `EXPIRE`) should override this method for efficiency.
    async fn touch(&self, key: &str, seconds: u64) -> bool {
        // We work with raw bytes to avoid requiring T: Serialize here.
        // The default delegates to get + set using serde_json::Value.
        if let Ok(Some(val)) = self.get::<serde_json::Value>(key).await {
            let ttl = Duration::from_secs(seconds);
            self.set(key, &val, ttl).await.is_ok()
        } else {
            false
        }
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

    /// Override: extend the TTL of an existing entry without deserialising
    /// the value, by directly updating `expires_at` in the entry.
    async fn touch(&self, key: &str, seconds: u64) -> bool {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(key) {
            if !entry.is_expired() {
                entry.expires_at = std::time::Instant::now() + Duration::from_secs(seconds);
                return true;
            }
        }
        false
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

    // ─── New tests ────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn set_and_get_string() {
        let cache = MemoryCache::new();
        cache
            .set("greeting", &"hello".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        let val: Option<String> = cache.get("greeting").await.unwrap();
        assert_eq!(val, Some("hello".to_string()));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let cache = MemoryCache::new();
        let val: Option<String> = cache.get("no-such-key").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn set_overwrites_existing_value() {
        let cache = MemoryCache::new();
        cache
            .set("k", &"v1".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .set("k", &"v2".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        let val: Option<String> = cache.get("k").await.unwrap();
        assert_eq!(val, Some("v2".to_string()));
    }

    #[tokio::test]
    async fn forget_removes_value() {
        let cache = MemoryCache::new();
        cache
            .set("tmp", &"value".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        cache.delete("tmp").await.unwrap();
        let val: Option<String> = cache.get("tmp").await.unwrap();
        assert!(val.is_none());
    }

    #[tokio::test]
    async fn forget_nonexistent_key_is_ok() {
        let cache = MemoryCache::new();
        assert!(cache.delete("ghost").await.is_ok());
    }

    #[tokio::test]
    async fn flush_removes_all_entries() {
        let cache = MemoryCache::new();
        for i in 0..5i32 {
            cache
                .set(&format!("k{}", i), &i, Duration::from_secs(60))
                .await
                .unwrap();
        }
        cache.flush().await.unwrap();
        for i in 0..5i32 {
            let v: Option<i32> = cache.get(&format!("k{}", i)).await.unwrap();
            assert!(v.is_none());
        }
    }

    #[tokio::test]
    async fn value_expires_after_ttl() {
        let cache = MemoryCache::new();
        cache
            .set("ephemeral", &"here today".to_string(), Duration::from_millis(50))
            .await
            .unwrap();

        let v: Option<String> = cache.get("ephemeral").await.unwrap();
        assert!(v.is_some());

        tokio::time::sleep(Duration::from_millis(100)).await;

        let v: Option<String> = cache.get("ephemeral").await.unwrap();
        assert!(v.is_none());
    }

    #[tokio::test]
    async fn remember_computes_and_caches_value() {
        let cache = MemoryCache::new();
        let computed: String = cache
            .remember("expensive", Duration::from_secs(60), || async {
                Ok("computed result".to_string())
            })
            .await
            .unwrap();
        assert_eq!(computed, "computed result");

        let cached: String = cache
            .remember("expensive", Duration::from_secs(60), || async {
                Ok("should not run".to_string())
            })
            .await
            .unwrap();
        assert_eq!(cached, "computed result");
    }

    #[tokio::test]
    async fn increment_from_zero() {
        let cache = MemoryCache::new();
        let v = cache.increment("views", 1).await.unwrap();
        assert_eq!(v, 1);
    }

    #[tokio::test]
    async fn increment_accumulates() {
        let cache = MemoryCache::new();
        cache.increment("hits", 1).await.unwrap();
        cache.increment("hits", 1).await.unwrap();
        let v = cache.increment("hits", 5).await.unwrap();
        assert_eq!(v, 7);
    }

    #[tokio::test]
    async fn decrement_reduces_value() {
        let cache = MemoryCache::new();
        cache
            .set("stock", &10i64, Duration::from_secs(60))
            .await
            .unwrap();
        let v = cache.decrement("stock", 3).await.unwrap();
        assert_eq!(v, 7);
    }

    #[tokio::test]
    async fn decrement_below_zero_is_allowed() {
        let cache = MemoryCache::new();
        let v = cache.decrement("balance", 5).await.unwrap();
        assert_eq!(v, -5);
    }

    #[tokio::test]
    async fn tagged_set_and_get() {
        let cache = MemoryCache::new();
        cache
            .tags(&["users"])
            .set("user:1", &"Alice".to_string(), Duration::from_secs(60))
            .await
            .unwrap();

        let v: Option<String> = cache.tags(&["users"]).get("user:1").await.unwrap();
        assert_eq!(v, Some("Alice".to_string()));
    }

    #[tokio::test]
    async fn flush_tag_removes_all_entries_with_that_tag() {
        let cache = MemoryCache::new();
        cache
            .tags(&["posts"])
            .set("post:1", &"First".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .tags(&["posts"])
            .set("post:2", &"Second".to_string(), Duration::from_secs(60))
            .await
            .unwrap();

        cache.tags(&["posts"]).flush().await.unwrap();

        let v1: Option<String> = cache.get("post:1").await.unwrap();
        let v2: Option<String> = cache.get("post:2").await.unwrap();
        assert!(v1.is_none());
        assert!(v2.is_none());
    }

    #[tokio::test]
    async fn flushing_one_tag_does_not_affect_other_tagged_entries() {
        let cache = MemoryCache::new();
        cache
            .tags(&["users"])
            .set("user:99", &"keep me".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        cache
            .tags(&["posts"])
            .set("post:99", &"flush me".to_string(), Duration::from_secs(60))
            .await
            .unwrap();

        cache.tags(&["posts"]).flush().await.unwrap();

        let kept: Option<String> = cache.get("user:99").await.unwrap();
        assert_eq!(kept, Some("keep me".to_string()));
    }

    #[tokio::test]
    async fn entry_with_multiple_tags_removed_by_any_tag_flush() {
        let cache = MemoryCache::new();
        cache
            .tags(&["a", "b"])
            .set("shared", &"value".to_string(), Duration::from_secs(60))
            .await
            .unwrap();

        cache.tags(&["a"]).flush().await.unwrap();

        let v: Option<String> = cache.get("shared").await.unwrap();
        assert!(v.is_none());
    }

    #[tokio::test]
    async fn stats_track_hits_and_misses() {
        let cache = MemoryCache::new();
        cache
            .set("stat_key", &"v".to_string(), Duration::from_secs(60))
            .await
            .unwrap();

        let _: Option<String> = cache.get("stat_key").await.unwrap();
        let _: Option<String> = cache.get("no_key").await.unwrap();

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
    }

    #[test]
    fn stats_empty_hit_rate_is_zero() {
        let stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);
    }

    // ─── Cache::touch() tests ─────────────────────────────────────────────────

    #[tokio::test]
    async fn touch_existing_key_returns_true() {
        let cache = MemoryCache::new();
        cache
            .set("k", &"v".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        let result = cache.touch("k", 120).await;
        assert!(result);
    }

    #[tokio::test]
    async fn touch_nonexistent_key_returns_false() {
        let cache = MemoryCache::new();
        let result = cache.touch("ghost", 60).await;
        assert!(!result);
    }

    #[tokio::test]
    async fn touch_preserves_value() {
        let cache = MemoryCache::new();
        cache
            .set("greeting", &"hello".to_string(), Duration::from_secs(60))
            .await
            .unwrap();
        cache.touch("greeting", 120).await;
        let val: Option<String> = cache.get("greeting").await.unwrap();
        assert_eq!(val, Some("hello".to_string()));
    }

    #[tokio::test]
    async fn touch_extends_ttl_so_key_does_not_expire() {
        let cache = MemoryCache::new();
        cache
            .set("short", &"x".to_string(), Duration::from_millis(80))
            .await
            .unwrap();
        // Extend before expiry.
        tokio::time::sleep(Duration::from_millis(50)).await;
        let touched = cache.touch("short", 10).await; // extend by 10 s
        assert!(touched, "touch should succeed while key is still alive");
        // If we immediately check, value must still be present.
        let val: Option<String> = cache.get("short").await.unwrap();
        assert!(val.is_some(), "value must survive after touch");
    }

    #[tokio::test]
    async fn touch_expired_key_returns_false() {
        let cache = MemoryCache::new();
        cache
            .set("expired", &"y".to_string(), Duration::from_millis(50))
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(100)).await;
        let result = cache.touch("expired", 60).await;
        assert!(!result, "touch on an expired key must return false");
    }

    #[tokio::test]
    async fn touch_numeric_value_stays_intact() {
        let cache = MemoryCache::new();
        cache.set("counter", &42i64, Duration::from_secs(60)).await.unwrap();
        cache.touch("counter", 120).await;
        let val: Option<i64> = cache.get("counter").await.unwrap();
        assert_eq!(val, Some(42));
    }

    #[tokio::test]
    async fn set_many_and_get_many() {
        let cache = MemoryCache::new();
        let items: Vec<(String, String)> = vec![
            ("a".into(), "alpha".into()),
            ("b".into(), "beta".into()),
            ("c".into(), "gamma".into()),
        ];
        let refs: Vec<(&str, &String)> = items.iter().map(|(k, v)| (k.as_str(), v)).collect();
        cache.set_many(&refs, Duration::from_secs(60)).await.unwrap();

        let results: HashMap<String, String> = cache.get_many(&["a", "b", "c"]).await.unwrap();
        assert_eq!(results.get("a").map(|s| s.as_str()), Some("alpha"));
        assert_eq!(results.get("b").map(|s| s.as_str()), Some("beta"));
        assert_eq!(results.get("c").map(|s| s.as_str()), Some("gamma"));
    }
}
