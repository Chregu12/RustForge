//! Cache watcher for cache hit/miss tracking

use crate::{Entry, EntryType, Storage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Cache operation type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum CacheOperation {
    Hit,
    Miss,
    Set,
    Delete,
    Forget,
    Flush,
}

/// Cache information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheInfo {
    pub operation: CacheOperation,
    pub key: String,
    pub value: Option<String>,
    pub ttl: Option<u64>,
    pub driver: String,
    pub tags: Vec<String>,
    pub occurred_at: DateTime<Utc>,
}

impl CacheInfo {
    /// Create a new cache info for a hit
    pub fn hit(key: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            operation: CacheOperation::Hit,
            key: key.into(),
            value: None,
            ttl: None,
            driver: driver.into(),
            tags: Vec::new(),
            occurred_at: Utc::now(),
        }
    }

    /// Create a new cache info for a miss
    pub fn miss(key: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            operation: CacheOperation::Miss,
            key: key.into(),
            value: None,
            ttl: None,
            driver: driver.into(),
            tags: Vec::new(),
            occurred_at: Utc::now(),
        }
    }

    /// Create a new cache info for a set operation
    pub fn set(key: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            operation: CacheOperation::Set,
            key: key.into(),
            value: None,
            ttl: None,
            driver: driver.into(),
            tags: Vec::new(),
            occurred_at: Utc::now(),
        }
    }

    /// Create a new cache info for a delete operation
    pub fn delete(key: impl Into<String>, driver: impl Into<String>) -> Self {
        Self {
            operation: CacheOperation::Delete,
            key: key.into(),
            value: None,
            ttl: None,
            driver: driver.into(),
            tags: Vec::new(),
            occurred_at: Utc::now(),
        }
    }

    /// Create a new cache info for a flush operation
    pub fn flush(driver: impl Into<String>) -> Self {
        Self {
            operation: CacheOperation::Flush,
            key: "*".to_string(),
            value: None,
            ttl: None,
            driver: driver.into(),
            tags: Vec::new(),
            occurred_at: Utc::now(),
        }
    }

    /// Set the cached value (truncated for display)
    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        let val = value.into();
        // Truncate large values for storage
        self.value = Some(if val.len() > 1000 {
            // Find a valid char boundary at or before byte 1000
            let mut end = 1000;
            while !val.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            format!("{}... ({} bytes)", &val[..end], val.len())
        } else {
            val
        });
        self
    }

    /// Set TTL in seconds
    pub fn with_ttl(mut self, ttl_seconds: u64) -> Self {
        self.ttl = Some(ttl_seconds);
        self
    }

    /// Add cache tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }
}

/// Cache watcher for monitoring cache operations
#[derive(Clone)]
pub struct CacheWatcher {
    storage: Storage,
}

impl CacheWatcher {
    /// Create a new cache watcher
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    /// Record a cache operation
    pub async fn record(&self, info: CacheInfo) {
        let entry = Entry::new(
            EntryType::Cache,
            json!({
                "operation": info.operation,
                "key": info.key,
                "value": info.value,
                "ttl": info.ttl,
                "driver": info.driver,
                "tags": info.tags,
                "occurred_at": info.occurred_at,
            }),
        )
        .with_tag(format!("operation:{:?}", info.operation).to_lowercase())
        .with_tag(format!("driver:{}", info.driver));

        self.storage.store(entry).await;
    }

    /// Get all recorded cache operations
    pub async fn all(&self) -> Vec<Entry> {
        self.storage.by_type(EntryType::Cache).await
    }

    /// Get cache hits
    pub async fn hits(&self) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| entry.tags.contains(&"operation:hit".to_string()))
            .collect()
    }

    /// Get cache misses
    pub async fn misses(&self) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| entry.tags.contains(&"operation:miss".to_string()))
            .collect()
    }

    /// Get operations for a specific driver
    pub async fn by_driver(&self, driver: &str) -> Vec<Entry> {
        let all = self.all().await;
        let tag = format!("driver:{}", driver);
        all.into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect()
    }

    /// Get cache statistics
    pub async fn statistics(&self) -> CacheStatistics {
        let all = self.all().await;

        let hits = all
            .iter()
            .filter(|e| e.tags.contains(&"operation:hit".to_string()))
            .count();
        let misses = all
            .iter()
            .filter(|e| e.tags.contains(&"operation:miss".to_string()))
            .count();
        let sets = all
            .iter()
            .filter(|e| e.tags.contains(&"operation:set".to_string()))
            .count();
        let deletes = all
            .iter()
            .filter(|e| e.tags.contains(&"operation:delete".to_string()))
            .count();

        let total_lookups = hits + misses;
        let hit_rate = if total_lookups > 0 {
            (hits as f64 / total_lookups as f64) * 100.0
        } else {
            0.0
        };

        CacheStatistics {
            total_operations: all.len(),
            hits,
            misses,
            sets,
            deletes,
            hit_rate,
        }
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStatistics {
    pub total_operations: usize,
    pub hits: usize,
    pub misses: usize,
    pub sets: usize,
    pub deletes: usize,
    pub hit_rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_info_hit() {
        let info = CacheInfo::hit("user:123", "redis");
        assert_eq!(info.operation, CacheOperation::Hit);
        assert_eq!(info.key, "user:123");
        assert_eq!(info.driver, "redis");
    }

    #[tokio::test]
    async fn test_cache_info_miss() {
        let info = CacheInfo::miss("user:456", "redis");
        assert_eq!(info.operation, CacheOperation::Miss);
        assert_eq!(info.key, "user:456");
    }

    #[tokio::test]
    async fn test_cache_info_set_with_ttl() {
        let info = CacheInfo::set("session:abc", "redis")
            .with_value("session_data")
            .with_ttl(3600);

        assert_eq!(info.operation, CacheOperation::Set);
        assert_eq!(info.ttl, Some(3600));
        assert!(info.value.is_some());
    }

    #[tokio::test]
    async fn test_cache_watcher_record() {
        let storage = Storage::new();
        let watcher = CacheWatcher::new(storage);

        let info = CacheInfo::hit("user:123", "redis");
        watcher.record(info).await;

        let operations = watcher.all().await;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].content["operation"], "hit");
    }

    #[tokio::test]
    async fn test_cache_hits_and_misses() {
        let storage = Storage::new();
        let watcher = CacheWatcher::new(storage);

        watcher.record(CacheInfo::hit("key1", "redis")).await;
        watcher.record(CacheInfo::miss("key2", "redis")).await;
        watcher.record(CacheInfo::hit("key3", "redis")).await;

        let hits = watcher.hits().await;
        let misses = watcher.misses().await;

        assert_eq!(hits.len(), 2);
        assert_eq!(misses.len(), 1);
    }

    #[tokio::test]
    async fn test_cache_by_driver() {
        let storage = Storage::new();
        let watcher = CacheWatcher::new(storage);

        watcher.record(CacheInfo::hit("key1", "redis")).await;
        watcher.record(CacheInfo::hit("key2", "memcached")).await;
        watcher.record(CacheInfo::hit("key3", "redis")).await;

        let redis_ops = watcher.by_driver("redis").await;
        assert_eq!(redis_ops.len(), 2);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let storage = Storage::new();
        let watcher = CacheWatcher::new(storage);

        watcher.record(CacheInfo::hit("key1", "redis")).await;
        watcher.record(CacheInfo::hit("key2", "redis")).await;
        watcher.record(CacheInfo::miss("key3", "redis")).await;
        watcher.record(CacheInfo::set("key4", "redis")).await;
        watcher.record(CacheInfo::delete("key5", "redis")).await;

        let stats = watcher.statistics().await;
        assert_eq!(stats.total_operations, 5);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.sets, 1);
        assert_eq!(stats.deletes, 1);
        assert_eq!(stats.hit_rate, 66.66666666666666);
    }

    #[tokio::test]
    async fn test_cache_value_truncation() {
        let large_value = "x".repeat(2000);
        let info = CacheInfo::set("large_key", "redis").with_value(large_value.clone());

        assert!(info.value.is_some());
        let stored = info.value.unwrap();
        assert!(stored.len() < large_value.len());
        assert!(stored.contains("2000 bytes"));
    }

    #[tokio::test]
    async fn test_cache_flush_operation() {
        let storage = Storage::new();
        let watcher = CacheWatcher::new(storage);

        let info = CacheInfo::flush("redis");
        watcher.record(info).await;

        let operations = watcher.all().await;
        assert_eq!(operations.len(), 1);
        assert_eq!(operations[0].content["operation"], "flush");
        assert_eq!(operations[0].content["key"], "*");
    }

    #[tokio::test]
    async fn test_cache_with_tags() {
        let info = CacheInfo::set("user:123", "redis")
            .with_tags(vec!["users".to_string(), "authentication".to_string()]);

        assert_eq!(info.tags.len(), 2);
        assert!(info.tags.contains(&"users".to_string()));
    }
}
