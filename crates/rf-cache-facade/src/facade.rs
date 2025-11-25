//! Cache facade providing Laravel-style static caching API

use crate::manager::GLOBAL_CACHE;
use rf_cache::{CacheResult, TaggedCache};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Trait for values that can be converted to a TTL duration.
///
/// This allows accepting both `u64` (seconds) and `Duration`.
///
/// # Examples
///
/// ```rust
/// // Both work:
/// Cache::put("key", &"value", 3600).await?;           // seconds
/// Cache::put("key", &"value", Duration::from_secs(3600)).await?;  // Duration
/// ```
pub trait IntoTtl {
    fn into_duration(self) -> Duration;
}

impl IntoTtl for u64 {
    fn into_duration(self) -> Duration {
        Duration::from_secs(self)
    }
}

impl IntoTtl for i64 {
    fn into_duration(self) -> Duration {
        Duration::from_secs(self.max(0) as u64)
    }
}

impl IntoTtl for u32 {
    fn into_duration(self) -> Duration {
        Duration::from_secs(self as u64)
    }
}

impl IntoTtl for i32 {
    fn into_duration(self) -> Duration {
        Duration::from_secs(self.max(0) as u64)
    }
}

impl IntoTtl for Duration {
    fn into_duration(self) -> Duration {
        self
    }
}

/// The Cache facade providing a static-like API for caching.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_cache_facade::Cache;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Put a value - Laravel style with seconds!
/// Cache::put("key", "value", 3600).await?;
///
/// // Get a value
/// if let Some(value) = Cache::get::<String>("key").await? {
///     println!("Value: {}", value);
/// }
///
/// // Remember pattern
/// let value = Cache::remember("expensive", 60, || async {
///     Ok::<_, String>("computed".to_string())
/// }).await?;
/// # Ok(())
/// # }
/// ```
pub struct Cache;

impl Cache {
    /// Get a value from cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(value) = Cache::get::<String>("key").await? {
    ///     println!("Value: {}", value);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get<T: DeserializeOwned + Send>(key: &str) -> CacheResult<Option<T>> {
        let manager = GLOBAL_CACHE.read().await;
        manager.get(key).await
    }

    /// Put a value in cache with TTL
    ///
    /// Accepts seconds as integer or Duration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Laravel style - just pass seconds!
    /// Cache::put("key", "value", 3600).await?;
    ///
    /// // Also works with Duration
    /// use std::time::Duration;
    /// Cache::put("key", "value", Duration::from_secs(3600)).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn put<T: Serialize + Sync, TTL: IntoTtl>(
        key: &str,
        value: T,
        ttl: TTL,
    ) -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().await;
        manager.put(key, &value, ttl.into_duration()).await
    }

    /// Store a value forever (very long TTL)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::forever("key", "value").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn forever<T: Serialize + Sync>(key: &str, value: T) -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().await;
        manager.forever(key, &value).await
    }

    /// Remove a value from cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::forget("key").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn forget(key: &str) -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().await;
        manager.forget(key).await
    }

    /// Check if a key exists in cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// if Cache::has("key").await? {
    ///     println!("Key exists");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn has(key: &str) -> CacheResult<bool> {
        let manager = GLOBAL_CACHE.read().await;
        manager.has(key).await
    }

    /// Flush all cache entries
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::flush().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn flush() -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().await;
        manager.flush().await
    }

    /// Remember pattern: get from cache or compute and store
    ///
    /// Accepts seconds as integer or Duration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Laravel style - just pass seconds!
    /// let users = Cache::remember("users", 3600, || async {
    ///     Ok::<_, String>("computed".to_string())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remember<T, F, Fut, TTL: IntoTtl>(
        key: &str,
        ttl: TTL,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        let manager = GLOBAL_CACHE.read().await;
        manager.remember(key, ttl.into_duration(), f).await
    }

    /// Remember forever: get from cache or compute and store forever
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let value = Cache::remember_forever("key", || async {
    ///     Ok::<_, String>("computed".to_string())
    /// }).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn remember_forever<T, F, Fut>(
        key: &str,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        let manager = GLOBAL_CACHE.read().await;
        manager.remember_forever(key, f).await
    }

    /// Pull: get and delete a value from cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(value) = Cache::pull::<String>("key").await? {
    ///     println!("Value: {}", value);
    ///     // Value is now removed from cache
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn pull<T: DeserializeOwned + Send>(key: &str) -> CacheResult<Option<T>> {
        let manager = GLOBAL_CACHE.read().await;
        manager.pull(key).await
    }

    /// Add: store only if key doesn't exist
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Laravel style
    /// let added = Cache::add("key", "value", 60).await?;
    /// if added {
    ///     println!("Value was added");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn add<T: Serialize + Sync, TTL: IntoTtl>(
        key: &str,
        value: T,
        ttl: TTL,
    ) -> CacheResult<bool> {
        let manager = GLOBAL_CACHE.read().await;
        manager.add(key, &value, ttl.into_duration()).await
    }

    /// Create a tagged cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache_facade::Cache;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tagged = Cache::tags(&["users", "user:123"]).await;
    /// tagged.set("user:123:profile", &"data", Duration::from_secs(3600)).await?;
    ///
    /// // Later, flush all entries with tag "users"
    /// Cache::tags(&["users"]).await.flush().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn tags(tags: &[&str]) -> TaggedCache {
        let manager = GLOBAL_CACHE.read().await;
        manager.tags(tags)
    }

    /// Increment a numeric value in cache
    pub async fn increment(key: &str, value: i64) -> CacheResult<i64> {
        let manager = GLOBAL_CACHE.read().await;
        let current: Option<i64> = manager.get(key).await?;
        let new_value = current.unwrap_or(0) + value;
        manager.put(key, &new_value, Duration::from_secs(3600)).await?;
        Ok(new_value)
    }

    /// Decrement a numeric value in cache
    pub async fn decrement(key: &str, value: i64) -> CacheResult<i64> {
        Self::increment(key, -value).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_put_get() {
        // Laravel-style: just pass seconds!
        Cache::put("test_key", "test_value", 60).await.unwrap();

        let value: Option<String> = Cache::get("test_key").await.unwrap();
        assert!(value.is_some());

        Cache::forget("test_key").await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_has() {
        // Laravel-style: just pass seconds!
        Cache::put("exists", "value", 60).await.unwrap();

        assert!(Cache::has("exists").await.unwrap());
        assert!(!Cache::has("not_exists").await.unwrap());

        Cache::forget("exists").await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_remember() {
        // Laravel-style: just pass seconds!
        let value = Cache::remember("remember_key", 60, || async {
            Ok::<_, rf_cache::CacheError>("computed".to_string())
        })
        .await
        .unwrap();

        assert_eq!(value, "computed");

        Cache::forget("remember_key").await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_pull() {
        // Laravel-style: just pass seconds!
        Cache::put("pull_key", "value", 60).await.unwrap();

        let value: Option<String> = Cache::pull("pull_key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));

        // Should be removed
        assert!(!Cache::has("pull_key").await.unwrap());
    }

    #[tokio::test]
    async fn test_cache_add() {
        Cache::forget("add_key").await.ok();

        // Laravel-style: just pass seconds!
        let added = Cache::add("add_key", "value1", 60).await.unwrap();
        assert!(added);

        let added = Cache::add("add_key", "value2", 60).await.unwrap();
        assert!(!added);

        Cache::forget("add_key").await.unwrap();
    }

    #[tokio::test]
    async fn test_cache_increment_decrement() {
        Cache::forget("counter").await.ok();

        let value = Cache::increment("counter", 5).await.unwrap();
        assert_eq!(value, 5);

        let value = Cache::increment("counter", 3).await.unwrap();
        assert_eq!(value, 8);

        let value = Cache::decrement("counter", 2).await.unwrap();
        assert_eq!(value, 6);

        Cache::forget("counter").await.unwrap();
    }
}
