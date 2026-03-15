//! File service providing high-level file operations over the StoragePort trait

use rf_plugins::{CommandError, StoragePort};
use std::sync::Arc;

/// High-level file service wrapping a StoragePort implementation
pub struct FileService {
    storage: Arc<dyn StoragePort>,
    default_disk: String,
}

impl FileService {
    /// Create a new FileService using the given StoragePort
    pub fn new(storage: Arc<dyn StoragePort>) -> Self {
        Self {
            storage,
            default_disk: "local".to_string(),
        }
    }

    /// Create a new FileService with a default disk
    pub fn with_default_disk(storage: Arc<dyn StoragePort>, disk: impl Into<String>) -> Self {
        Self {
            storage,
            default_disk: disk.into(),
        }
    }

    /// Store a file on the specified disk (defaults to "local")
    pub async fn store(
        &self,
        contents: impl Into<Vec<u8>>,
        path: &str,
        disk: Option<&str>,
    ) -> Result<String, CommandError> {
        let disk = disk.unwrap_or(&self.default_disk);
        self.storage.put(disk, path, contents.into()).await?;
        Ok(path.to_string())
    }

    /// Get the URL for a stored file
    pub fn url(&self, path: &str, disk: Option<&str>) -> Result<String, CommandError> {
        let disk = disk.unwrap_or(&self.default_disk);
        Ok(format!("/storage/{}/{}", disk, path))
    }

    /// Read file contents from the specified disk
    pub async fn read(&self, path: &str, disk: Option<&str>) -> Result<Vec<u8>, CommandError> {
        let disk = disk.unwrap_or(&self.default_disk);
        self.storage.get(disk, path).await
    }

    /// Delete a file from the specified disk
    pub async fn delete(&self, path: &str, disk: Option<&str>) -> Result<(), CommandError> {
        let disk = disk.unwrap_or(&self.default_disk);
        self.storage.delete(disk, path).await
    }

    /// Check if a file exists on the specified disk
    pub async fn exists(&self, path: &str, disk: Option<&str>) -> Result<bool, CommandError> {
        let disk = disk.unwrap_or(&self.default_disk);
        self.storage.exists(disk, path).await
    }
}
