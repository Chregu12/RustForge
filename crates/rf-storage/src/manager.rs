//! Storage manager for handling multiple storage disks

use crate::{Storage, StorageError, StorageResult};
use std::collections::HashMap;
use std::sync::Arc;

/// Storage manager that handles multiple storage disks
///
/// This is similar to Laravel's Storage facade that allows you to switch
/// between different storage backends (local, S3, etc.)
///
/// # Example
///
/// ```rust,no_run
/// use rf_storage::{StorageManager, LocalStorage, S3Storage, S3Config};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut manager = StorageManager::new();
///
/// // Register local disk
/// let local = LocalStorage::new("/var/storage".into(), "http://localhost/storage");
/// manager.add_disk("local", Arc::new(local));
///
/// // Register S3 disk
/// let s3_config = S3Config {
///     bucket: "my-bucket".to_string(),
///     region: "us-east-1".to_string(),
///     endpoint: None,
///     access_key: "key".to_string(),
///     secret_key: "secret".to_string(),
///     path_style: false,
/// };
/// let s3 = S3Storage::new(s3_config).await?;
/// manager.add_disk("s3", Arc::new(s3));
///
/// // Set default disk
/// manager.set_default("s3");
///
/// // Use default disk
/// manager.disk_default().put("file.txt", b"Hello".to_vec()).await?;
///
/// // Use specific disk
/// manager.disk("local")?.put("file.txt", b"Hello".to_vec()).await?;
/// # Ok(())
/// # }
/// ```
pub struct StorageManager {
    disks: HashMap<String, Arc<dyn Storage>>,
    default_disk: Option<String>,
}

impl StorageManager {
    /// Create a new storage manager
    pub fn new() -> Self {
        Self {
            disks: HashMap::new(),
            default_disk: None,
        }
    }

    /// Add a storage disk
    pub fn add_disk(&mut self, name: impl Into<String>, storage: Arc<dyn Storage>) {
        let name = name.into();

        // Set as default if no default is set
        if self.default_disk.is_none() {
            self.default_disk = Some(name.clone());
        }

        self.disks.insert(name, storage);
    }

    /// Get a storage disk by name
    pub fn disk(&self, name: &str) -> StorageResult<&Arc<dyn Storage>> {
        self.disks
            .get(name)
            .ok_or_else(|| StorageError::Other(format!("Disk '{}' not found", name)))
    }

    /// Get the default storage disk
    pub fn disk_default(&self) -> StorageResult<&Arc<dyn Storage>> {
        let default_name = self
            .default_disk
            .as_ref()
            .ok_or_else(|| StorageError::Other("No default disk configured".to_string()))?;

        self.disk(default_name)
    }

    /// Set the default disk
    pub fn set_default(&mut self, name: impl Into<String>) {
        self.default_disk = Some(name.into());
    }

    /// Get the name of the default disk
    pub fn default_disk_name(&self) -> Option<&str> {
        self.default_disk.as_deref()
    }

    /// Get all disk names
    pub fn disk_names(&self) -> Vec<String> {
        self.disks.keys().cloned().collect()
    }

    /// Check if a disk exists
    pub fn has_disk(&self, name: &str) -> bool {
        self.disks.contains_key(name)
    }

    /// Remove a disk
    pub fn remove_disk(&mut self, name: &str) -> Option<Arc<dyn Storage>> {
        let removed = self.disks.remove(name);

        // Update default if we removed it
        if self.default_disk.as_deref() == Some(name) {
            self.default_disk = self.disks.keys().next().cloned();
        }

        removed
    }
}

impl Default for StorageManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MemoryStorage;

    #[tokio::test]
    async fn test_storage_manager() {
        let mut manager = StorageManager::new();

        let local = Arc::new(MemoryStorage::new());
        let s3 = Arc::new(MemoryStorage::new());

        manager.add_disk("local", local.clone());
        manager.add_disk("s3", s3.clone());

        assert_eq!(manager.disk_names().len(), 2);
        assert!(manager.has_disk("local"));
        assert!(manager.has_disk("s3"));
    }

    #[tokio::test]
    async fn test_default_disk() {
        let mut manager = StorageManager::new();

        let local = Arc::new(MemoryStorage::new());
        manager.add_disk("local", local);

        // First disk is default
        assert_eq!(manager.default_disk_name(), Some("local"));

        let s3 = Arc::new(MemoryStorage::new());
        manager.add_disk("s3", s3);

        // Set new default
        manager.set_default("s3");
        assert_eq!(manager.default_disk_name(), Some("s3"));
    }

    #[tokio::test]
    async fn test_disk_operations() {
        let mut manager = StorageManager::new();

        let storage = Arc::new(MemoryStorage::new());
        manager.add_disk("test", storage);

        let disk = manager.disk("test").unwrap();
        disk.put("file.txt", b"Hello".to_vec()).await.unwrap();

        let contents = disk.get("file.txt").await.unwrap();
        assert_eq!(contents, b"Hello");
    }

    #[tokio::test]
    async fn test_remove_disk() {
        let mut manager = StorageManager::new();

        let storage = Arc::new(MemoryStorage::new());
        manager.add_disk("test", storage);

        assert!(manager.has_disk("test"));
        manager.remove_disk("test");
        assert!(!manager.has_disk("test"));
    }

    #[tokio::test]
    async fn test_disk_not_found() {
        let manager = StorageManager::new();
        let result = manager.disk("nonexistent");
        assert!(result.is_err());
    }
}
