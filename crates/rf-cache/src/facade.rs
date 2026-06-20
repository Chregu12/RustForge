//! Cache facade providing Laravel-style static caching API

use crate::cache_manager::GLOBAL_CACHE;
use crate::{CacheResult, TaggedCache};
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Trait for values that can be converted to a TTL duration.
///
/// This allows accepting both `u64` (seconds) and `Duration`.
///
/// # Examples
///
/// ```rust,no_run
/// use rf_cache::facade::Cache;
/// use std::time::Duration;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Both work:
/// Cache::put("key", &"value", 3600)?;           // seconds
/// Cache::put("key", &"value", Duration::from_secs(3600))?;  // Duration
/// # Ok(())
/// # }
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
/// use rf_cache::facade::Cache;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Put a value - Laravel style with seconds!
/// Cache::put("key", "value", 3600)?;
///
/// // Get a value
/// if let Some(value) = Cache::get::<String>("key")? {
///     println!("Value: {}", value);
/// }
///
/// // Remember pattern
/// let value = Cache::remember("expensive", 60, || async {
///     Ok::<_, rf_cache::CacheError>("computed".to_string())
/// })?;
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
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(value) = Cache::get::<String>("key")? {
    ///     println!("Value: {}", value);
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get<T: DeserializeOwned + Send>(key: &str) -> CacheResult<Option<T>> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.get(key)
    }

    /// Put a value in cache with TTL
    ///
    /// Accepts seconds as integer or Duration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Laravel style - just pass seconds!
    /// Cache::put("key", "value", 3600)?;
    ///
    /// // Also works with Duration
    /// use std::time::Duration;
    /// Cache::put("key", "value", Duration::from_secs(3600))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn put<T: Serialize + Sync, TTL: IntoTtl>(
        key: &str,
        value: T,
        ttl: TTL,
    ) -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.put(key, &value, ttl.into_duration())
    }

    /// Store a value forever (very long TTL)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::forever("key", "value")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn forever<T: Serialize + Sync>(key: &str, value: T) -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.forever(key, &value)
    }

    /// Remove a value from cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::forget("key")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn forget(key: &str) -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.forget(key)
    }

    /// Alias for [`forget`] — naming-consistency convenience.
    ///
    /// [`forget`]: Cache::forget
    pub fn delete(key: &str) -> CacheResult<()> {
        Self::forget(key)
    }

    /// Check if a key exists in cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// if Cache::has("key")? {
    ///     println!("Key exists");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn has(key: &str) -> CacheResult<bool> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.has(key)
    }

    /// Touch a key: extend its expiration to `now + ttl` without re-reading or
    /// rewriting its value.
    ///
    /// Accepts seconds as integer or Duration. Returns `true` if the key existed
    /// and was touched, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::put("key", "value", 60)?;
    ///
    /// // Laravel style - just pass seconds!
    /// let touched = Cache::touch("key", 3600)?;
    /// if touched {
    ///     println!("Expiration extended");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn touch<TTL: IntoTtl>(key: &str, ttl: TTL) -> CacheResult<bool> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.touch(key, ttl.into_duration())
    }

    /// Flush all cache entries
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// Cache::flush()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn flush() -> CacheResult<()> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.flush()
    }

    /// Remember pattern: get from cache or compute and store
    ///
    /// Accepts seconds as integer or Duration.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Laravel style - just pass seconds!
    /// let users = Cache::remember("users", 3600, || async {
    ///     Ok::<_, rf_cache::CacheError>("computed".to_string())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn remember<T, F, Fut, TTL: IntoTtl>(
        key: &str,
        ttl: TTL,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.remember(key, ttl.into_duration(), f)
    }

    /// Remember forever: get from cache or compute and store forever
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let value = Cache::remember_forever("key", || async {
    ///     Ok::<_, rf_cache::CacheError>("computed".to_string())
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn remember_forever<T, F, Fut>(
        key: &str,
        f: F,
    ) -> CacheResult<T>
    where
        T: Serialize + DeserializeOwned + Send + Sync + 'static,
        F: FnOnce() -> Fut + Send,
        Fut: std::future::Future<Output = CacheResult<T>> + Send,
    {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.remember_forever(key, f)
    }

    /// Pull: get and delete a value from cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// if let Some(value) = Cache::pull::<String>("key")? {
    ///     println!("Value: {}", value);
    ///     // Value is now removed from cache
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn pull<T: DeserializeOwned + Send>(key: &str) -> CacheResult<Option<T>> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.pull(key)
    }

    /// Add: store only if key doesn't exist
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Laravel style
    /// let added = Cache::add("key", "value", 60)?;
    /// if added {
    ///     println!("Value was added");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn add<T: Serialize + Sync, TTL: IntoTtl>(
        key: &str,
        value: T,
        ttl: TTL,
    ) -> CacheResult<bool> {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.add(key, &value, ttl.into_duration())
    }

    /// Create a tagged cache
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_cache::facade::Cache;
    /// use std::time::Duration;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tagged = Cache::tags(&["users", "user:123"]);
    /// // Note: TaggedCache is still async internally
    /// # Ok(())
    /// # }
    /// ```
    pub fn tags(tags: &[&str]) -> TaggedCache {
        let manager = GLOBAL_CACHE.read().unwrap();
        manager.tags(tags)
    }

    /// Increment a numeric value in cache
    pub fn increment(key: &str, value: i64) -> CacheResult<i64> {
        let manager = GLOBAL_CACHE.read().unwrap();
        let current: Option<i64> = manager.get(key)?;
        let new_value = current.unwrap_or(0) + value;
        manager.put(key, &new_value, Duration::from_secs(3600))?;
        Ok(new_value)
    }

    /// Decrement a numeric value in cache
    pub fn decrement(key: &str, value: i64) -> CacheResult<i64> {
        Self::increment(key, -value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_put_get() {
        // Laravel-style: just pass seconds!
        Cache::put("test_key", "test_value", 60).unwrap();

        let value: Option<String> = Cache::get("test_key").unwrap();
        assert!(value.is_some());

        Cache::forget("test_key").unwrap();
    }

    #[test]
    fn test_cache_has() {
        // Laravel-style: just pass seconds!
        Cache::put("exists", "value", 60).unwrap();

        assert!(Cache::has("exists").unwrap());
        assert!(!Cache::has("not_exists").unwrap());

        Cache::forget("exists").unwrap();
    }

    #[test]
    fn test_cache_remember() {
        // Laravel-style: just pass seconds!
        let value = Cache::remember("remember_key", 60, || async {
            Ok::<_, crate::CacheError>("computed".to_string())
        })
        .unwrap();

        assert_eq!(value, "computed");

        Cache::forget("remember_key").unwrap();
    }

    #[test]
    fn test_cache_pull() {
        // Laravel-style: just pass seconds!
        Cache::put("pull_key", "value", 60).unwrap();

        let value: Option<String> = Cache::pull("pull_key").unwrap();
        assert_eq!(value, Some("value".to_string()));

        // Should be removed
        assert!(!Cache::has("pull_key").unwrap());
    }

    #[test]
    fn test_cache_add() {
        Cache::forget("add_key").ok();

        // Laravel-style: just pass seconds!
        let added = Cache::add("add_key", "value1", 60).unwrap();
        assert!(added);

        let added = Cache::add("add_key", "value2", 60).unwrap();
        assert!(!added);

        Cache::forget("add_key").unwrap();
    }

    #[test]
    fn test_cache_touch() {
        Cache::forget("touch_key").ok();

        // Touch on a missing key returns false.
        assert!(!Cache::touch("touch_key", 60).unwrap());

        // After putting a value, touch returns true and the key still exists.
        Cache::put("touch_key", "value", 60).unwrap();
        assert!(Cache::touch("touch_key", 3600).unwrap());
        assert!(Cache::has("touch_key").unwrap());

        Cache::forget("touch_key").unwrap();
    }

    #[test]
    fn test_facade_touch_value_unchanged() {
        Cache::forget("touch_unchanged").ok();
        Cache::put("touch_unchanged", "original", 60).unwrap();
        assert!(Cache::touch("touch_unchanged", 3600).unwrap());
        // Value must not be rewritten by touch.
        let v: Option<String> = Cache::get("touch_unchanged").unwrap();
        assert_eq!(v, Some("original".to_string()));
        Cache::forget("touch_unchanged").unwrap();
    }

    #[test]
    fn test_facade_touch_extends_lifetime_past_original_expiry() {
        Cache::forget("touch_extend").ok();
        // Use Duration to confirm IntoTtl-for-Duration reaches the backend.
        Cache::put("touch_extend", "v", Duration::from_millis(60)).unwrap();
        assert!(Cache::touch("touch_extend", Duration::from_secs(10)).unwrap());
        // Sleep past the original 60ms expiry; key must survive.
        std::thread::sleep(Duration::from_millis(150));
        assert!(
            Cache::has("touch_extend").unwrap(),
            "facade touch must extend lifetime through the same backend as put"
        );
        let v: Option<String> = Cache::get("touch_extend").unwrap();
        assert_eq!(v, Some("v".to_string()));
        Cache::forget("touch_extend").unwrap();
    }

    #[test]
    fn test_facade_touch_expired_returns_false() {
        Cache::forget("touch_expired").ok();
        Cache::put("touch_expired", "v", Duration::from_millis(30)).unwrap();
        std::thread::sleep(Duration::from_millis(80));
        // Expired key: touch must not resurrect it.
        assert!(!Cache::touch("touch_expired", 3600).unwrap());
        assert!(!Cache::has("touch_expired").unwrap());
        Cache::forget("touch_expired").unwrap();
    }

    #[test]
    fn test_facade_touch_ttl_conversion_matches_put() {
        // i64 IntoTtl: negative clamps to 0 (immediate expiry), same as put's path.
        Cache::forget("touch_i64").ok();
        Cache::put("touch_i64", "v", 60i64).unwrap();
        // Touch with a negative i64 -> Duration::ZERO -> expires immediately.
        assert!(Cache::touch("touch_i64", -5i64).unwrap());
        // Now expired (tiny sleep guards against same-instant strict comparison).
        std::thread::sleep(Duration::from_millis(5));
        assert!(!Cache::has("touch_i64").unwrap());
        Cache::forget("touch_i64").unwrap();
    }

    #[test]
    fn test_cache_increment_decrement() {
        Cache::forget("counter").ok();

        let value = Cache::increment("counter", 5).unwrap();
        assert_eq!(value, 5);

        let value = Cache::increment("counter", 3).unwrap();
        assert_eq!(value, 8);

        let value = Cache::decrement("counter", 2).unwrap();
        assert_eq!(value, 6);

        Cache::forget("counter").unwrap();
    }
}
