//! Redis cache backend for production
//!
//! Provides a production-ready cache implementation using Redis as the backend.
//! Supports distributed caching, tag-based invalidation, and stampede prevention.
//!
//! ## Features
//!
//! - **Distributed**: Cache shared across multiple instances
//! - **Persistent**: Optional persistence with Redis
//! - **Tag Support**: Group related cache entries
//! - **Stampede Prevention**: Built-in locking mechanism
//! - **TTL Support**: Automatic expiration
//! - **Connection Pooling**: Efficient connection management
//!
//! ## Example
//!
//! ```no_run
//! use rf_cache::{RedisCache, Cache};
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), rf_cache::CacheError> {
//! let cache = RedisCache::new("redis://localhost:6379", "myapp").await?;
//!
//! // Basic operations
//! cache.set("key", &"value", Duration::from_secs(60)).await?;
//! let value: Option<String> = cache.get("key").await?;
//! cache.delete("key").await?;
//!
//! // With tags
//! cache.tags(&["users", "user:123"])
//!     .set("user:123:profile", &"data", Duration::from_secs(3600))
//!     .await?;
//!
//! // Invalidate by tag
//! cache.tags(&["users"]).flush().await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "redis-backend")]
use crate::{Cache, CacheError, CacheResult};
#[cfg(feature = "redis-backend")]
use async_trait::async_trait;
#[cfg(feature = "redis-backend")]
use deadpool_redis::{Config, Pool, Runtime};
#[cfg(feature = "redis-backend")]
use redis::AsyncCommands;
#[cfg(feature = "redis-backend")]
use serde::{de::DeserializeOwned, Serialize};
#[cfg(feature = "redis-backend")]
use std::sync::Arc;
#[cfg(feature = "redis-backend")]
use std::time::Duration;
#[cfg(feature = "redis-backend")]
use tokio::sync::Mutex;

#[cfg(feature = "redis-backend")]
/// Redis cache backend
///
/// Provides a production-ready cache implementation using Redis.
/// Supports distributed caching, tagging, and advanced features.
#[derive(Clone)]
pub struct RedisCache {
    pool: Pool,
    prefix: String,
    locks: Arc<Mutex<std::collections::HashMap<String, Arc<Mutex<()>>>>>,
}

#[cfg(feature = "redis-backend")]
impl RedisCache {
    /// Create new Redis cache
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    /// * `prefix` - Cache key prefix for namespacing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rf_cache::RedisCache;
    /// # async fn example() -> Result<(), rf_cache::CacheError> {
    /// let cache = RedisCache::new("redis://localhost:6379", "myapp").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(redis_url: &str, prefix: &str) -> CacheResult<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        // Test connection
        let mut conn = pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        redis::cmd("PING")
            .query_async::<_, String>(&mut conn)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(Self {
            pool,
            prefix: prefix.to_string(),
            locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
        })
    }

    /// Get cache key with prefix
    fn cache_key(&self, key: &str) -> String {
        format!("{}:cache:{}", self.prefix, key)
    }

    /// Get tag key
    fn tag_key(&self, tag: &str) -> String {
        format!("{}:tag:{}", self.prefix, tag)
    }

    /// Get lock key
    fn lock_key(&self, key: &str) -> String {
        format!("{}:lock:{}", self.prefix, key)
    }

    /// Create tagged cache
    pub fn tags(&self, tags: &[&str]) -> RedisTaggedCache {
        RedisTaggedCache {
            cache: self.clone(),
            tags: tags.iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Remember with lock (stampede prevention)
    ///
    /// Prevents cache stampede by using distributed locks.
    /// Only one process will compute the value while others wait.
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

        // Try to acquire distributed lock
        let lock_key = self.lock_key(key);
        let lock_acquired = self
            .acquire_lock(&lock_key, Duration::from_secs(10))
            .await?;

        if lock_acquired {
            // We got the lock, double-check cache and compute if needed
            if let Some(value) = self.get(key).await? {
                self.release_lock(&lock_key).await?;
                return Ok(value);
            }

            // Compute value
            let value = f().await?;
            self.set(key, &value, ttl).await?;

            // Release lock
            self.release_lock(&lock_key).await?;

            Ok(value)
        } else {
            // Someone else has the lock, wait and try again
            tokio::time::sleep(Duration::from_millis(100)).await;

            // Try to get cached value (should be there now)
            if let Some(value) = self.get(key).await? {
                Ok(value)
            } else {
                // Still not there, try one more time with lock
                self.remember_with_lock(key, ttl, f).await
            }
        }
    }

    /// Acquire distributed lock
    async fn acquire_lock(&self, lock_key: &str, ttl: Duration) -> CacheResult<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let result: bool = redis::cmd("SET")
            .arg(lock_key)
            .arg("1")
            .arg("NX") // Only set if not exists
            .arg("EX")
            .arg(ttl.as_secs())
            .query_async(&mut conn)
            .await
            .unwrap_or(false);

        Ok(result)
    }

    /// Release distributed lock
    async fn release_lock(&self, lock_key: &str) -> CacheResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let _: () = conn
            .del(lock_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(())
    }

    /// Add key to tag
    async fn add_tag(&self, tag: &str, key: &str) -> CacheResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let tag_key = self.tag_key(tag);
        let cache_key = self.cache_key(key);

        let _: () = conn
            .sadd(&tag_key, &cache_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(())
    }

    /// Flush tag
    async fn flush_tag(&self, tag: &str) -> CacheResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let tag_key = self.tag_key(tag);

        // Get all keys with this tag
        let keys: Vec<String> = conn
            .smembers(&tag_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        // Delete all keys
        if !keys.is_empty() {
            let _: () = conn
                .del(&keys)
                .await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
        }

        // Delete tag set
        let _: () = conn
            .del(&tag_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(())
    }
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl Cache for RedisCache {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let cache_key = self.cache_key(key);

        let data: Option<Vec<u8>> = conn
            .get(&cache_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        if let Some(bytes) = data {
            let value = serde_json::from_slice(&bytes)
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
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let cache_key = self.cache_key(key);

        let data =
            serde_json::to_vec(value).map_err(|e| CacheError::Serialization(e.to_string()))?;

        let _: () = conn
            .set_ex(&cache_key, data, ttl.as_secs())
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let cache_key = self.cache_key(key);

        let _: () = conn
            .del(&cache_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        let cache_key = self.cache_key(key);

        let exists: bool = conn
            .exists(&cache_key)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        Ok(exists)
    }

    async fn flush(&self) -> CacheResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        // Get all keys with our prefix
        let pattern = format!("{}:*", self.prefix);
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| CacheError::Backend(e.to_string()))?;

        // Delete all keys
        if !keys.is_empty() {
            let _: () = conn
                .del(&keys)
                .await
                .map_err(|e| CacheError::Backend(e.to_string()))?;
        }

        Ok(())
    }
}

#[cfg(feature = "redis-backend")]
/// Tagged cache for Redis
pub struct RedisTaggedCache {
    cache: RedisCache,
    tags: Vec<String>,
}

#[cfg(feature = "redis-backend")]
impl RedisTaggedCache {
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
            self.cache.add_tag(tag, key).await?;
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

#[cfg(all(test, feature = "redis-backend"))]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn create_test_cache() -> RedisCache {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        RedisCache::new(&redis_url, "test").await.unwrap()
    }

    #[tokio::test]
    async fn test_redis_basic_operations() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_basic_operations: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let cache = create_test_cache().await;
        cache.flush().await.unwrap();

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
    async fn test_redis_distributed_cache() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_distributed_cache: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let cache1 = create_test_cache().await;
        let cache2 = create_test_cache().await;

        cache1.flush().await.unwrap();

        // Set in cache1
        cache1
            .set("shared", &"value", Duration::from_secs(60))
            .await
            .unwrap();

        // Get from cache2 (different instance)
        let value: Option<String> = cache2.get("shared").await.unwrap();
        assert_eq!(value, Some("value".to_string()));
    }

    #[tokio::test]
    async fn test_redis_ttl_expiration() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_ttl_expiration: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let cache = create_test_cache().await;
        cache.flush().await.unwrap();

        cache
            .set("key", &"value", Duration::from_secs(1))
            .await
            .unwrap();

        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));

        tokio::time::sleep(Duration::from_secs(2)).await;

        let value: Option<String> = cache.get("key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_redis_tags() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_tags: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let cache = create_test_cache().await;
        cache.flush().await.unwrap();

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
    async fn test_redis_remember_with_lock() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_remember_with_lock: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let cache = create_test_cache().await;
        cache.flush().await.unwrap();

        let value: String = cache
            .remember_with_lock("expensive", Duration::from_secs(60), || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("computed".to_string())
            })
            .await
            .unwrap();

        assert_eq!(value, "computed");

        // Second call should use cached value (no delay)
        let start = std::time::Instant::now();
        let value: String = cache
            .remember_with_lock("expensive", Duration::from_secs(60), || async {
                tokio::time::sleep(Duration::from_millis(100)).await;
                Ok("new_value".to_string())
            })
            .await
            .unwrap();

        let elapsed = start.elapsed();
        assert_eq!(value, "computed");
        assert!(elapsed < Duration::from_millis(50)); // Should be fast (cached)
    }

    #[tokio::test]
    async fn test_redis_concurrent_stampede_prevention() {
        if !redis_available().await {
            eprintln!(
                "⏭️  Skipping test_redis_concurrent_stampede_prevention: Redis not available"
            );
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let cache = Arc::new(create_test_cache().await);
        cache.flush().await.unwrap();

        let mut handles = vec![];

        for _ in 0..5 {
            let cache = cache.clone();
            let handle = tokio::spawn(async move {
                cache
                    .remember_with_lock("expensive", Duration::from_secs(60), || async {
                        tokio::time::sleep(Duration::from_millis(100)).await;
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
