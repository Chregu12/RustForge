use crate::store::{CacheError, CacheStore, CacheValue};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::fs;
use tokio::sync::RwLock;

/// File-based cache store
pub struct FileStore {
    directory: PathBuf,
    lock: RwLock<()>,
}

impl FileStore {
    pub async fn new(directory: impl Into<PathBuf>) -> Result<Self, CacheError> {
        let directory = directory.into();

        // Create directory if it doesn't exist
        if !directory.exists() {
            fs::create_dir_all(&directory).await?;
        }

        Ok(Self {
            directory,
            lock: RwLock::new(()),
        })
    }

    pub fn from_env() -> Result<Self, CacheError> {
        let directory = std::env::var("CACHE_FILE_PATH")
            .unwrap_or_else(|_| "./storage/cache".to_string());

        // Use blocking version since we can't await in this context
        std::fs::create_dir_all(&directory)?;

        Ok(Self {
            directory: PathBuf::from(directory),
            lock: RwLock::new(()),
        })
    }

    fn get_file_path(&self, key: &str) -> PathBuf {
        // Use hash to avoid file system issues with special characters
        let hash = format!("{:x}", md5::compute(key));
        self.directory.join(format!("{}.cache", hash))
    }

    async fn read_value(&self, path: &Path) -> Result<Option<CacheValue>, CacheError> {
        if !path.exists() {
            return Ok(None);
        }

        let data = fs::read(path).await?;
        let value: CacheValue = serde_json::from_slice(&data)
            .map_err(|e| CacheError::Deserialization(e.to_string()))?;

        if value.is_expired() {
            fs::remove_file(path).await.ok(); // Ignore errors
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    async fn write_value(&self, path: &Path, value: &CacheValue) -> Result<(), CacheError> {
        let data = serde_json::to_vec(value)
            .map_err(|e| CacheError::Serialization(e.to_string()))?;

        fs::write(path, data).await?;
        Ok(())
    }
}

#[async_trait]
impl CacheStore for FileStore {
    async fn get(&self, key: &str) -> Result<Option<CacheValue>, CacheError> {
        let _guard = self.lock.read().await;
        let path = self.get_file_path(key);
        self.read_value(&path).await
    }

    async fn set(&self, key: &str, value: CacheValue, ttl: Option<Duration>) -> Result<(), CacheError> {
        let _guard = self.lock.write().await;
        let path = self.get_file_path(key);

        let final_value = if let Some(ttl) = ttl {
            CacheValue::with_ttl(value.data, ttl)
        } else {
            value
        };

        self.write_value(&path, &final_value).await
    }

    async fn delete(&self, key: &str) -> Result<bool, CacheError> {
        let _guard = self.lock.write().await;
        let path = self.get_file_path(key);

        if path.exists() {
            fs::remove_file(path).await?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    async fn exists(&self, key: &str) -> Result<bool, CacheError> {
        let _guard = self.lock.read().await;
        let path = self.get_file_path(key);
        Ok(path.exists())
    }

    async fn flush(&self) -> Result<(), CacheError> {
        let _guard = self.lock.write().await;

        let mut entries = fs::read_dir(&self.directory).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("cache") {
                fs::remove_file(path).await.ok(); // Ignore errors
            }
        }

        Ok(())
    }

    async fn increment(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        let _guard = self.lock.write().await;
        let path = self.get_file_path(key);

        let current = if let Some(value) = self.read_value(&path).await? {
            let s = value.to_string()?;
            s.parse::<i64>().unwrap_or(0)
        } else {
            0
        };

        let new_value = current + amount;
        let value = CacheValue::from_string(new_value.to_string());
        self.write_value(&path, &value).await?;

        Ok(new_value)
    }

    async fn decrement(&self, key: &str, amount: i64) -> Result<i64, CacheError> {
        self.increment(key, -amount).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_store_get_set() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).await.unwrap();
        let value = CacheValue::from_string("test_value");

        store.set("test_key", value.clone(), None).await.unwrap();
        let result = store.get("test_key").await.unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().to_string().unwrap(), "test_value");
    }

    #[tokio::test]
    async fn test_file_store_delete() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).await.unwrap();
        let value = CacheValue::from_string("test");

        store.set("key", value, None).await.unwrap();
        assert!(store.delete("key").await.unwrap());
        assert!(store.get("key").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_file_store_flush() {
        let temp_dir = TempDir::new().unwrap();
        let store = FileStore::new(temp_dir.path()).await.unwrap();

        store.set("key1", CacheValue::from_string("value1"), None).await.unwrap();
        store.set("key2", CacheValue::from_string("value2"), None).await.unwrap();

        store.flush().await.unwrap();

        assert!(store.get("key1").await.unwrap().is_none());
        assert!(store.get("key2").await.unwrap().is_none());
    }
}
