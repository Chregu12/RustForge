//! Storage trait definition

use crate::StorageError;
use async_trait::async_trait;

/// Storage backend trait
#[async_trait]
pub trait Storage: Send + Sync {
    /// Store file at path with contents
    async fn put(&self, path: &str, contents: Vec<u8>) -> Result<(), StorageError>;

    /// Get file contents
    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError>;

    /// Delete file
    async fn delete(&self, path: &str) -> Result<(), StorageError>;

    /// Check if file exists
    async fn exists(&self, path: &str) -> Result<bool, StorageError>;

    /// Get file size in bytes
    async fn size(&self, path: &str) -> Result<u64, StorageError>;

    /// List files in directory (with prefix)
    async fn list(&self, path: &str) -> Result<Vec<String>, StorageError>;

    /// Get public URL for file
    fn url(&self, path: &str) -> String;

    /// Copy file
    async fn copy(&self, from: &str, to: &str) -> Result<(), StorageError> {
        let contents = self.get(from).await?;
        self.put(to, contents).await
    }

    /// Move file
    async fn move_file(&self, from: &str, to: &str) -> Result<(), StorageError> {
        self.copy(from, to).await?;
        self.delete(from).await
    }
}
