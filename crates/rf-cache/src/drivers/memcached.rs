//! Memcached cache backend driver
//!
//! Provides a distributed caching solution using Memcached servers.

use crate::{Cache, CacheError, CacheResult};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::time::Duration;

/// Memcached cache driver
pub struct MemcachedDriver {
    client: memcache::Client,
    prefix: String,
}

impl MemcachedDriver {
    /// Create a new Memcached driver
    ///
    /// # Arguments
    ///
    /// * `servers` - List of Memcached server URLs (e.g., "memcache://localhost:11211")
    /// * `prefix` - Key prefix for namespacing
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_cache::drivers::memcached::MemcachedDriver;
    ///
    /// let driver = MemcachedDriver::new(
    ///     vec!["memcache://localhost:11211".to_string()],
    ///     "myapp".to_string()
    /// ).unwrap();
    /// ```
    pub fn new(servers: Vec<String>, prefix: String) -> Result<Self, CacheError> {
        let server_str = servers.join(",");
        let client = memcache::Client::connect(server_str)
            .map_err(|e| CacheError::Backend(format!("Failed to connect to Memcached: {}", e)))?;

        Ok(Self { client, prefix })
    }

    /// Create a new Memcached driver with default prefix
    pub fn with_default_prefix(servers: Vec<String>) -> Result<Self, CacheError> {
        Self::new(servers, "rf_cache".to_string())
    }

    /// Generate prefixed key
    fn make_key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }
}

#[async_trait]
impl Cache for MemcachedDriver {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        let prefixed_key = self.make_key(key);

        // Memcached operations are synchronous, so we use tokio::task::spawn_blocking
        let client = self.client.clone();
        let result = tokio::task::spawn_blocking(move || {
            client.get::<String>(&prefixed_key)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?;

        match result {
            Ok(Some(value)) => {
                let deserialized = serde_json::from_str(&value)
                    .map_err(|e| CacheError::Deserialization(e.to_string()))?;
                Ok(Some(deserialized))
            }
            Ok(None) => Ok(None),
            Err(_) => Ok(None),
        }
    }

    async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        let prefixed_key = self.make_key(key);
        let serialized = serde_json::to_string(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;
        let expiration = ttl.as_secs() as u32;

        let client = self.client.clone();
        tokio::task::spawn_blocking(move || {
            client.set(&prefixed_key, serialized, expiration)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?
        .map_err(|e| CacheError::Backend(format!("Memcached set error: {}", e)))?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let prefixed_key = self.make_key(key);
        let client = self.client.clone();

        tokio::task::spawn_blocking(move || {
            client.delete(&prefixed_key)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?
        .map_err(|e| CacheError::Backend(format!("Memcached delete error: {}", e)))?;

        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let prefixed_key = self.make_key(key);
        let client = self.client.clone();

        let exists = tokio::task::spawn_blocking(move || {
            client.get::<String>(&prefixed_key).is_ok()
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?;

        Ok(exists)
    }

    async fn flush(&self) -> CacheResult<()> {
        let client = self.client.clone();

        tokio::task::spawn_blocking(move || {
            client.flush()
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?
        .map_err(|e| CacheError::Backend(format!("Memcached flush error: {}", e)))?;

        Ok(())
    }
}

/// Extension trait for Memcached-specific operations
#[async_trait]
pub trait MemcachedOps: Cache {
    /// Increment a counter value
    async fn increment(&self, key: &str, amount: u64) -> CacheResult<u64>;

    /// Decrement a counter value
    async fn decrement(&self, key: &str, amount: u64) -> CacheResult<u64>;

    /// Touch a key to extend its TTL
    async fn touch(&self, key: &str, ttl: Duration) -> CacheResult<bool>;
}

#[async_trait]
impl MemcachedOps for MemcachedDriver {
    async fn increment(&self, key: &str, amount: u64) -> CacheResult<u64> {
        let prefixed_key = self.make_key(key);
        let client = self.client.clone();

        let result = tokio::task::spawn_blocking(move || {
            client.increment(&prefixed_key, amount)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?
        .map_err(|e| CacheError::Backend(format!("Memcached increment error: {}", e)))?;

        Ok(result)
    }

    async fn decrement(&self, key: &str, amount: u64) -> CacheResult<u64> {
        let prefixed_key = self.make_key(key);
        let client = self.client.clone();

        let result = tokio::task::spawn_blocking(move || {
            client.decrement(&prefixed_key, amount)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?
        .map_err(|e| CacheError::Backend(format!("Memcached decrement error: {}", e)))?;

        Ok(result)
    }

    async fn touch(&self, key: &str, ttl: Duration) -> CacheResult<bool> {
        let prefixed_key = self.make_key(key);
        let expiration = ttl.as_secs() as u32;
        let client = self.client.clone();

        let result = tokio::task::spawn_blocking(move || {
            client.touch(&prefixed_key, expiration)
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?
        .map_err(|e| CacheError::Backend(format!("Memcached touch error: {}", e)))?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests require a running Memcached instance
    // Run: docker run -d -p 11211:11211 memcached

    #[tokio::test]
    #[ignore] // Ignore by default - requires Memcached server
    async fn test_memcached_basic_operations() {
        let driver = MemcachedDriver::new(
            vec!["memcache://localhost:11211".to_string()],
            "test".to_string(),
        )
        .expect("Failed to create Memcached driver");

        // Test set and get
        driver
            .set("test_key", &"test_value", Duration::from_secs(60))
            .await
            .unwrap();

        let value: Option<String> = driver.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Test delete
        driver.delete("test_key").await.unwrap();
        let value: Option<String> = driver.get("test_key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    #[ignore] // Ignore by default - requires Memcached server
    async fn test_memcached_increment() {
        let driver = MemcachedDriver::new(
            vec!["memcache://localhost:11211".to_string()],
            "test".to_string(),
        )
        .expect("Failed to create Memcached driver");

        // Initialize counter
        driver
            .set("counter", &0u64, Duration::from_secs(60))
            .await
            .unwrap();

        // Increment
        let result = driver.increment("counter", 5).await.unwrap();
        assert_eq!(result, 5);

        // Increment again
        let result = driver.increment("counter", 3).await.unwrap();
        assert_eq!(result, 8);

        // Clean up
        driver.delete("counter").await.unwrap();
    }
}
