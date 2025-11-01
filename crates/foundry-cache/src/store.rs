use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Cache store trait
#[async_trait]
pub trait CacheStore: Send + Sync {
    /// Get a value from the cache
    async fn get(&self, key: &str) -> Result<Option<CacheValue>, CacheError>;

    /// Set a value in the cache with optional TTL
    async fn set(&self, key: &str, value: CacheValue, ttl: Option<Duration>) -> Result<(), CacheError>;

    /// Delete a value from the cache
    async fn delete(&self, key: &str) -> Result<bool, CacheError>;

    /// Check if a key exists
    async fn exists(&self, key: &str) -> Result<bool, CacheError>;

    /// Clear all cache entries
    async fn flush(&self) -> Result<(), CacheError>;

    /// Get multiple values at once
    async fn get_many(&self, keys: &[String]) -> Result<Vec<Option<CacheValue>>, CacheError> {
        let mut results = Vec::new();
        for key in keys {
            results.push(self.get(key).await?);
        }
        Ok(results)
    }

    /// Set multiple values at once
    async fn set_many(&self, items: Vec<(String, CacheValue, Option<Duration>)>) -> Result<(), CacheError> {
        for (key, value, ttl) in items {
            self.set(&key, value, ttl).await?;
        }
        Ok(())
    }

    /// Delete multiple keys at once
    async fn delete_many(&self, keys: &[String]) -> Result<usize, CacheError> {
        let mut count = 0;
        for key in keys {
            if self.delete(key).await? {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Increment a numeric value
    async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError>;

    /// Decrement a numeric value
    async fn decrement(&self, key: &str, amount: i64) -> Result<i64, CacheError>;

    /// Get cache statistics (optional)
    async fn stats(&self) -> Result<CacheStats, CacheError> {
        Ok(CacheStats::default())
    }
}

/// Cache value wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheValue {
    pub data: Vec<u8>,
    pub created_at: i64,
    pub expires_at: Option<i64>,
}

impl CacheValue {
    pub fn new(data: Vec<u8>) -> Self {
        Self {
            data,
            created_at: chrono::Utc::now().timestamp(),
            expires_at: None,
        }
    }

    pub fn with_ttl(data: Vec<u8>, ttl: Duration) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            data,
            created_at: now,
            expires_at: Some(now + ttl.as_secs() as i64),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            chrono::Utc::now().timestamp() > expires_at
        } else {
            false
        }
    }

    pub fn from_json<T: Serialize>(value: &T) -> Result<Self, CacheError> {
        let data = serde_json::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;
        Ok(Self::new(data))
    }

    pub fn to_json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, CacheError> {
        serde_json::from_slice(&self.data)
            .map_err(|e| CacheError::Deserialization(e.to_string()))
    }

    pub fn from_string(value: impl Into<String>) -> Self {
        Self::new(value.into().into_bytes())
    }

    pub fn to_string(&self) -> Result<String, CacheError> {
        String::from_utf8(self.data.clone())
            .map_err(|e| CacheError::Deserialization(e.to_string()))
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub size: u64,
    pub entries: u64,
    pub evictions: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Key not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Deserialization error: {0}")]
    Deserialization(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Redis error: {0}")]
    Redis(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Invalid value: {0}")]
    InvalidValue(String),

    #[error("Cache error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_value_expiration() {
        let value = CacheValue::new(vec![1, 2, 3]);
        assert!(!value.is_expired());

        let value = CacheValue::with_ttl(vec![1, 2, 3], Duration::from_secs(0));
        std::thread::sleep(Duration::from_millis(10));
        assert!(value.is_expired());
    }

    #[test]
    fn test_cache_value_json() {
        #[derive(Serialize, Deserialize, PartialEq, Debug)]
        struct TestData {
            name: String,
            age: u32,
        }

        let data = TestData {
            name: "John".to_string(),
            age: 30,
        };

        let value = CacheValue::from_json(&data).unwrap();
        let decoded: TestData = value.to_json().unwrap();

        assert_eq!(data, decoded);
    }

    #[test]
    fn test_cache_stats_hit_rate() {
        let mut stats = CacheStats::default();
        assert_eq!(stats.hit_rate(), 0.0);

        stats.hits = 7;
        stats.misses = 3;
        assert_eq!(stats.hit_rate(), 0.7);
    }
}
