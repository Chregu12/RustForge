//! File-based cache backend driver
//!
//! Provides a file system-based caching solution with atomic writes and proper locking.

use crate::{Cache, CacheError, CacheResult};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

/// File cache entry metadata
#[derive(Serialize, serde::Deserialize)]
struct FileCacheEntry {
    data: Vec<u8>,
    expires_at: u64, // Unix timestamp
}

impl FileCacheEntry {
    fn new(data: Vec<u8>, ttl: Duration) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let expires_at = now + ttl.as_secs();

        Self { data, expires_at }
    }

    fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        now > self.expires_at
    }
}

/// File-based cache driver with atomic writes
pub struct FileDriver {
    path: PathBuf,
    locks: Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

impl FileDriver {
    /// Create a new file cache driver
    ///
    /// # Arguments
    ///
    /// * `path` - Directory path for cache files
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_cache::drivers::file::FileDriver;
    /// use std::path::PathBuf;
    ///
    /// let driver = FileDriver::new(PathBuf::from("/tmp/cache")).unwrap();
    /// ```
    pub fn new(path: PathBuf) -> Result<Self, CacheError> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&path)
            .map_err(|e| CacheError::Backend(format!("Failed to create cache directory: {}", e)))?;

        Ok(Self {
            path,
            locks: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Get the file path for a cache key
    fn key_to_path(&self, key: &str) -> PathBuf {
        // Hash the key to create a safe filename
        let hash = md5::compute(key.as_bytes());
        let filename = format!("{:x}.cache", hash);

        // Create nested directory structure to avoid too many files in one directory
        let prefix = &filename[0..2];
        self.path.join(prefix).join(filename)
    }

    /// Acquire a lock for a specific key
    async fn acquire_lock(&self, key: &str) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().await;
        locks
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Read cache entry from file
    fn read_entry(&self, path: &Path) -> Result<FileCacheEntry, CacheError> {
        let mut file = File::open(path)
            .map_err(|e| CacheError::Backend(format!("Failed to open cache file: {}", e)))?;

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| CacheError::Backend(format!("Failed to read cache file: {}", e)))?;

        serde_json::from_slice(&buffer)
            .map_err(|e| CacheError::Deserialization(e.to_string()))
    }

    /// Write cache entry to file atomically
    fn write_entry(&self, path: &Path, entry: &FileCacheEntry) -> Result<(), CacheError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| CacheError::Backend(format!("Failed to create directory: {}", e)))?;
        }

        // Write to temporary file first (atomic write)
        let temp_path = path.with_extension("tmp");
        let mut temp_file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp_path)
            .map_err(|e| CacheError::Backend(format!("Failed to create temp file: {}", e)))?;

        let serialized = serde_json::to_vec(entry)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;

        temp_file
            .write_all(&serialized)
            .map_err(|e| CacheError::Backend(format!("Failed to write temp file: {}", e)))?;

        // Sync to disk
        temp_file
            .sync_all()
            .map_err(|e| CacheError::Backend(format!("Failed to sync temp file: {}", e)))?;

        // Atomic rename
        fs::rename(&temp_path, path)
            .map_err(|e| CacheError::Backend(format!("Failed to rename temp file: {}", e)))?;

        Ok(())
    }

    /// Clean up expired cache files
    pub async fn cleanup_expired(&self) -> CacheResult<usize> {
        let mut removed_count = 0;

        // Walk through cache directory
        let walk_path = self.path.clone();
        let entries = tokio::task::spawn_blocking(move || {
            let mut entries = Vec::new();
            if let Ok(read_dir) = fs::read_dir(&walk_path) {
                for entry in read_dir.flatten() {
                    if let Ok(path) = entry.path().canonicalize() {
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("cache") {
                            entries.push(path);
                        }
                    }
                }
            }
            entries
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?;

        for path in entries {
            if let Ok(entry) = self.read_entry(&path) {
                if entry.is_expired() {
                    if fs::remove_file(&path).is_ok() {
                        removed_count += 1;
                    }
                }
            }
        }

        Ok(removed_count)
    }
}

#[async_trait]
impl Cache for FileDriver {
    async fn get<T: DeserializeOwned + Send>(&self, key: &str) -> CacheResult<Option<T>> {
        let path = self.key_to_path(key);
        let lock = self.acquire_lock(key).await;
        let _guard = lock.lock().await;

        // Check if file exists
        if !path.exists() {
            return Ok(None);
        }

        let entry = self.read_entry(&path)?;

        // Check if expired
        if entry.is_expired() {
            // Delete expired file
            let _ = fs::remove_file(&path);
            return Ok(None);
        }

        // Deserialize data
        let value = serde_json::from_slice(&entry.data)
            .map_err(|e| CacheError::Deserialization(e.to_string()))?;

        Ok(Some(value))
    }

    async fn set<T: Serialize + Sync>(
        &self,
        key: &str,
        value: &T,
        ttl: Duration,
    ) -> CacheResult<()> {
        let path = self.key_to_path(key);
        let lock = self.acquire_lock(key).await;
        let _guard = lock.lock().await;

        // Serialize value
        let data = serde_json::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;

        // Create entry
        let entry = FileCacheEntry::new(data, ttl);

        // Write atomically
        self.write_entry(&path, &entry)?;

        Ok(())
    }

    async fn delete(&self, key: &str) -> CacheResult<()> {
        let path = self.key_to_path(key);
        let lock = self.acquire_lock(key).await;
        let _guard = lock.lock().await;

        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| CacheError::Backend(format!("Failed to delete cache file: {}", e)))?;
        }

        Ok(())
    }

    async fn exists(&self, key: &str) -> CacheResult<bool> {
        let path = self.key_to_path(key);
        let lock = self.acquire_lock(key).await;
        let _guard = lock.lock().await;

        if !path.exists() {
            return Ok(false);
        }

        let entry = match self.read_entry(&path) {
            Ok(e) => e,
            Err(_) => return Ok(false),
        };

        if entry.is_expired() {
            let _ = fs::remove_file(&path);
            return Ok(false);
        }

        Ok(true)
    }

    async fn flush(&self) -> CacheResult<()> {
        let path = self.path.clone();

        tokio::task::spawn_blocking(move || {
            // Remove all cache files
            if let Ok(read_dir) = fs::read_dir(&path) {
                for entry in read_dir.flatten() {
                    if let Ok(file_type) = entry.file_type() {
                        if file_type.is_file() {
                            let _ = fs::remove_file(entry.path());
                        } else if file_type.is_dir() {
                            let _ = fs::remove_dir_all(entry.path());
                        }
                    }
                }
            }
        })
        .await
        .map_err(|e| CacheError::Backend(format!("Task join error: {}", e)))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_driver_basic_operations() {
        let temp_dir = tempdir().unwrap();
        let driver = FileDriver::new(temp_dir.path().to_path_buf()).unwrap();

        // Test set and get
        driver
            .set("test_key", &"test_value", Duration::from_secs(60))
            .await
            .unwrap();

        let value: Option<String> = driver.get("test_key").await.unwrap();
        assert_eq!(value, Some("test_value".to_string()));

        // Test exists
        assert!(driver.exists("test_key").await.unwrap());

        // Test delete
        driver.delete("test_key").await.unwrap();
        let value: Option<String> = driver.get("test_key").await.unwrap();
        assert_eq!(value, None);
        assert!(!driver.exists("test_key").await.unwrap());
    }

    #[tokio::test]
    async fn test_file_driver_expiration() {
        let temp_dir = tempdir().unwrap();
        let driver = FileDriver::new(temp_dir.path().to_path_buf()).unwrap();

        // Set with short TTL
        driver
            .set("expire_key", &"value", Duration::from_millis(100))
            .await
            .unwrap();

        let value: Option<String> = driver.get("expire_key").await.unwrap();
        assert_eq!(value, Some("value".to_string()));

        // Wait for expiration
        tokio::time::sleep(Duration::from_millis(150)).await;

        let value: Option<String> = driver.get("expire_key").await.unwrap();
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_file_driver_flush() {
        let temp_dir = tempdir().unwrap();
        let driver = FileDriver::new(temp_dir.path().to_path_buf()).unwrap();

        // Set multiple keys
        driver
            .set("key1", &"value1", Duration::from_secs(60))
            .await
            .unwrap();
        driver
            .set("key2", &"value2", Duration::from_secs(60))
            .await
            .unwrap();

        // Flush all
        driver.flush().await.unwrap();

        // Verify all deleted
        assert!(!driver.exists("key1").await.unwrap());
        assert!(!driver.exists("key2").await.unwrap());
    }

    #[tokio::test]
    async fn test_file_driver_cleanup_expired() {
        let temp_dir = tempdir().unwrap();
        let driver = FileDriver::new(temp_dir.path().to_path_buf()).unwrap();

        // Set keys with different TTLs
        driver
            .set("long_key", &"value", Duration::from_secs(60))
            .await
            .unwrap();
        driver
            .set("short_key", &"value", Duration::from_millis(50))
            .await
            .unwrap();

        // Wait for short key to expire
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Clean up expired
        let removed = driver.cleanup_expired().await.unwrap();
        assert_eq!(removed, 1);

        // Verify long key still exists
        assert!(driver.exists("long_key").await.unwrap());
        assert!(!driver.exists("short_key").await.unwrap());
    }
}
