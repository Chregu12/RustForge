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
//! cache.set("key", "value", Duration::from_secs(60)).await?;
//! let value: Option<String> = cache.get("key").await?;
//! cache.delete("key").await?;
//!
//! // With tags
//! cache.tags(&["users", "user:123"])
//!     .set("user:123:profile", "data", Duration::from_secs(3600))
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
    async fn remember<T, F, Fut>(
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
        if let Some(value) = self.get(key).await? {
            return Ok(value);
        }

        let value = f().await?;
        self.set(key, &value, ttl).await?;
        Ok(value)
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
}

/// In-memory cache implementation
#[derive(Clone)]
pub struct MemoryCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    tags: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl MemoryCache {
    /// Create new memory cache
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            tags: Arc::new(RwLock::new(HashMap::new())),
            locks: Arc::new(Mutex::new(HashMap::new())),
        }
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
                return Ok(None);
            }

            let value = serde_json::from_slice(&entry.data)
                .map_err(|e| CacheError::Deserialization(e.to_string()))?;
            Ok(Some(value))
        } else {
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

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let mut entries = self.entries.write().await;
        entries.remove(key);
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
}
