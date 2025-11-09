//! In-memory storage backend for testing

use crate::{Storage, StorageError};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// In-memory storage for testing
///
/// # Example
///
/// ```
/// use rf_storage::{MemoryStorage, Storage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = MemoryStorage::new();
///
/// storage.put("test.txt", b"Hello".to_vec()).await?;
/// assert!(storage.exists("test.txt").await?);
///
/// let contents = storage.get("test.txt").await?;
/// assert_eq!(contents, b"Hello");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct MemoryStorage {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    public_url: String,
}

impl MemoryStorage {
    /// Create new memory storage
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            public_url: "http://localhost:3000".into(),
        }
    }

    /// Create with custom public URL
    pub fn with_url(public_url: impl Into<String>) -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            public_url: public_url.into(),
        }
    }

    /// Get all stored files (for testing)
    pub fn files(&self) -> HashMap<String, Vec<u8>> {
        self.files.lock().unwrap().clone()
    }

    /// Get number of stored files
    pub fn count(&self) -> usize {
        self.files.lock().unwrap().len()
    }

    /// Clear all files
    pub fn clear(&self) {
        self.files.lock().unwrap().clear();
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, path: &str, contents: Vec<u8>) -> Result<(), StorageError> {
        let size = contents.len();
        self.files.lock().unwrap().insert(path.to_string(), contents);

        tracing::debug!(
            path = %path,
            size = size,
            "File stored in memory"
        );

        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| StorageError::FileNotFound(path.to_string()))
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        self.files
            .lock()
            .unwrap()
            .remove(path)
            .ok_or_else(|| StorageError::FileNotFound(path.to_string()))?;

        tracing::debug!(path = %path, "File deleted from memory");

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        Ok(self.files.lock().unwrap().contains_key(path))
    }

    async fn size(&self, path: &str) -> Result<u64, StorageError> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .map(|v| v.len() as u64)
            .ok_or_else(|| StorageError::FileNotFound(path.to_string()))
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let prefix = prefix.trim_end_matches('/');

        let files: Vec<String> = self
            .files
            .lock()
            .unwrap()
            .keys()
            .filter(|k| {
                if prefix.is_empty() {
                    true
                } else {
                    k.starts_with(prefix)
                }
            })
            .cloned()
            .collect();

        Ok(files)
    }

    fn url(&self, path: &str) -> String {
        format!(
            "{}/storage/{}",
            self.public_url.trim_end_matches('/'),
            path.trim_start_matches('/')
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_storage_put_get() {
        let storage = MemoryStorage::new();

        storage.put("test.txt", b"Hello, World!".to_vec()).await.unwrap();

        let contents = storage.get("test.txt").await.unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_memory_storage_exists() {
        let storage = MemoryStorage::new();

        assert!(!storage.exists("test.txt").await.unwrap());

        storage.put("test.txt", b"content".to_vec()).await.unwrap();

        assert!(storage.exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_delete() {
        let storage = MemoryStorage::new();

        storage.put("test.txt", b"content".to_vec()).await.unwrap();
        assert!(storage.exists("test.txt").await.unwrap());

        storage.delete("test.txt").await.unwrap();
        assert!(!storage.exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_size() {
        let storage = MemoryStorage::new();

        storage.put("test.txt", b"Hello".to_vec()).await.unwrap();

        let size = storage.size("test.txt").await.unwrap();
        assert_eq!(size, 5);
    }

    #[tokio::test]
    async fn test_memory_storage_list() {
        let storage = MemoryStorage::new();

        storage.put("dir1/file1.txt", b"1".to_vec()).await.unwrap();
        storage.put("dir1/file2.txt", b"2".to_vec()).await.unwrap();
        storage.put("dir2/file3.txt", b"3".to_vec()).await.unwrap();

        let files = storage.list("dir1").await.unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.contains(&"dir1/file1.txt".to_string()));
        assert!(files.contains(&"dir1/file2.txt".to_string()));
    }

    #[tokio::test]
    async fn test_memory_storage_copy() {
        let storage = MemoryStorage::new();

        storage.put("original.txt", b"content".to_vec()).await.unwrap();
        storage.copy("original.txt", "copy.txt").await.unwrap();

        assert!(storage.exists("original.txt").await.unwrap());
        assert!(storage.exists("copy.txt").await.unwrap());

        let contents = storage.get("copy.txt").await.unwrap();
        assert_eq!(contents, b"content");
    }

    #[tokio::test]
    async fn test_memory_storage_move() {
        let storage = MemoryStorage::new();

        storage.put("old.txt", b"content".to_vec()).await.unwrap();
        storage.move_file("old.txt", "new.txt").await.unwrap();

        assert!(!storage.exists("old.txt").await.unwrap());
        assert!(storage.exists("new.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_memory_storage_url() {
        let storage = MemoryStorage::with_url("https://example.com");

        let url = storage.url("documents/test.pdf");
        assert_eq!(url, "https://example.com/storage/documents/test.pdf");
    }

    #[tokio::test]
    async fn test_memory_storage_clear() {
        let storage = MemoryStorage::new();

        storage.put("file1.txt", b"1".to_vec()).await.unwrap();
        storage.put("file2.txt", b"2".to_vec()).await.unwrap();

        assert_eq!(storage.count(), 2);

        storage.clear();
        assert_eq!(storage.count(), 0);
    }
}
