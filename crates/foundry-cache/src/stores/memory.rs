use crate::store::{CacheError, CacheStats, CacheStore, CacheValue};
use async_trait::async_trait;
use moka::future::Cache;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// In-memory cache store using Moka
pub struct MemoryStore {
    cache: Cache<String, CacheValue>,
    hits: Arc<AtomicU64>,
    misses: Arc<AtomicU64>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::with_capacity(10_000)
    }

    pub fn with_capacity(max_capacity: u64) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(Duration::from_secs(3600)) // Default 1 hour
            .build();

        Self {
            cache,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn with_config(max_capacity: u64, default_ttl: Duration) -> Self {
        let cache = Cache::builder()
            .max_capacity(max_capacity)
            .time_to_live(default_ttl)
            .build();

        Self {
            cache,
            hits: Arc::new(AtomicU64::new(0)),
            misses: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl Default for MemoryStore {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheStore for MemoryStore {
    async fn get(&self, key: &str) -> Result<Option<CacheValue>, CacheError> {
        if let Some(value) = self.cache.get(key).await {
            if value.is_expired() {
                self.cache.invalidate(key).await;
                self.misses.fetch_add(1, Ordering::Relaxed);
                Ok(None)
            } else {
                self.hits.fetch_add(1, Ordering::Relaxed);
                Ok(Some(value))
            }
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: CacheValue, ttl: Option<Duration>) -> Result<(), CacheError> {
        let final_value = if let Some(ttl) = ttl {
            CacheValue::with_ttl(value.data, ttl)
        } else {
            value
        };

        self.cache.insert(key.to_string(), final_value).await;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let existed = self.cache.get(key).await.is_some();
        self.cache.invalidate(key).await;
        Ok(existed)
    }

    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.cache.get(key).await.is_some())
    }

    async fn flush(&self) -> Result<(), CacheError> {
        self.cache.invalidate_all();
        Ok(())
    }

    async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        let current = if let Some(value) = self.cache.get(key).await {
            let s = value.to_string()?;
            s.parse::<i64>().unwrap_or(0)
        } else {
            0
        };

        let new_value = current + amount;
        let value = CacheValue::from_string(new_value.to_string());
        self.cache.insert(key.to_string(), value).await;

        Ok(new_value)
    }

    async fn decrement(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        self.increment(key, -amount).await
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        Ok(CacheStats {
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            size: 0, // Moka doesn't expose memory size easily
            entries: self.cache.entry_count(),
            evictions: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_store_get_set() {
        let store = MemoryStore::new();
        let value = CacheValue::from_string("test_value");

        store.set("test_key", value.clone(), None).await.unwrap();
        let result = store.get("test_key").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().to_string().unwrap(), "test_value");
    }

    #[tokio::test]
    async fn test_memory_store_delete() {
        let store = MemoryStore::new();
        let value = CacheValue::from_string("test");

        store.set("key", value, None).await.unwrap();
        assert!(store.delete("key").await.unwrap());
        assert!(store.get("key").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_memory_store_increment() {
        let store = MemoryStore::new();

        let result = store.increment("counter", 5).await.unwrap();
        assert_eq!(result, 5);

        let result = store.increment("counter", 3).await.unwrap();
        assert_eq!(result, 8);
    }

    #[tokio::test]
    async fn test_memory_store_ttl() {
        let store = MemoryStore::new();
        let value = CacheValue::from_string("expire_me");

        store.set("ttl_key", value, Some(Duration::from_millis(100))).await.unwrap();

        assert!(store.get("ttl_key").await.unwrap().is_some());

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(store.get("ttl_key").await.unwrap().is_none());
    }
}
