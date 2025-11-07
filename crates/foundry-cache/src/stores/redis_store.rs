use crate::store::{CacheError, CacheStats, CacheStore, CacheValue};
use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use std::time::Duration;

/// Redis cache store
pub struct RedisStore {
    pool: Pool,
    prefix: String,
}

impl RedisStore {
    pub fn new(url: impl Into<String>) -> Result<Self, CacheError> {
        Self::with_prefix(url, "cache:")
    }

    pub fn with_prefix(url: impl Into<String>, prefix: impl Into<String>) -> Result<Self, CacheError> {
        let cfg = Config {
            url: Some(url.into()),
            ..Default::default()
        };

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        Ok(Self {
            pool,
            prefix: prefix.into(),
        })
    }

    pub fn from_env() -> Result<Self, CacheError> {
        let url = std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let prefix = std::env::var("CACHE_PREFIX").unwrap_or_else(|_| "cache:".to_string());
        Self::with_prefix(url, prefix)
    }

    fn make_key(&self, key: &str) -> String {
        format!("{}{}", self.prefix, key)
    }
}

#[async_trait]
impl CacheStore for RedisStore {
    async fn get(&self, key: &str) -> Result<Option<CacheValue>, CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        let redis_key = self.make_key(key);
        let data: Option<Vec<u8>> = conn
            .get(&redis_key)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        if let Some(data) = data {
            let value: CacheValue = serde_json::from_slice(&data)
                .map_err(|e| CacheError::Deserialization(e.to_string()))?;

            if value.is_expired() {
                self.delete(key).await?;
                Ok(None)
            } else {
                Ok(Some(value))
            }
        } else {
            Ok(None)
        }
    }

    async fn set(&self, key: &str, value: CacheValue, ttl: Option<Duration>) -> Result<(), CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        let redis_key = self.make_key(key);
        let final_value = if let Some(ttl) = ttl {
            CacheValue::with_ttl(value.data, ttl)
        } else {
            value
        };

        let data = serde_json::to_vec(&final_value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;

        if let Some(ttl) = ttl {
            conn.set_ex::<_, _, ()>(&redis_key, data, ttl.as_secs())
                .await
                .map_err(|e| CacheError::Redis(e.to_string()))?;
        } else {
            conn.set::<_, _, ()>(&redis_key, data)
                .await
                .map_err(|e| CacheError::Redis(e.to_string()))?;
        }

        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        let redis_key = self.make_key(key);
        let deleted: i32 = conn
            .del(&redis_key)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(deleted > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        let redis_key = self.make_key(key);
        let exists: bool = conn
            .exists(&redis_key)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(exists)
    }

    async fn flush(&self) -> Result<(), CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        // Get all keys with our prefix
        let pattern = format!("{}*", self.prefix);
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        if !keys.is_empty() {
            conn.del::<_, ()>(&keys)
                .await
                .map_err(|e| CacheError::Redis(e.to_string()))?;
        }

        Ok(())
    }

    async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        let redis_key = self.make_key(key);
        let result: i64 = conn
            .incr(&redis_key, amount)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(result)
    }

    async fn decrement(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        let redis_key = self.make_key(key);
        let result: i64 = conn
            .decr(&redis_key, amount)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(result)
    }

    async fn stats(&self) -> Result<CacheStats, CacheError> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        // Get basic stats from Redis INFO command
        let info: String = redis::cmd("INFO")
            .arg("stats")
            .query_async(&mut *conn)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        // Parse relevant stats (simplified)
        let pattern = format!("{}*", self.prefix);
        let keys: Vec<String> = conn
            .keys(&pattern)
            .await
            .map_err(|e| CacheError::Redis(e.to_string()))?;

        Ok(CacheStats {
            hits: 0,  // Would need to track separately
            misses: 0,
            size: 0,
            entries: keys.len() as u64,
            evictions: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_store_get_set() {
        let store = RedisStore::new("redis://127.0.0.1:6379").unwrap();
        let value = CacheValue::from_string("test_value");

        store.set("test_key", value.clone(), None).await.unwrap();
        let result = store.get("test_key").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().to_string().unwrap(), "test_value");

        store.delete("test_key").await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_store_increment() {
        let store = RedisStore::new("redis://127.0.0.1:6379").unwrap();

        let result = store.increment("counter", 5).await.unwrap();
        assert_eq!(result, 5);

        let result = store.increment("counter", 3).await.unwrap();
        assert_eq!(result, 8);

        store.delete("counter").await.unwrap();
    }
}
