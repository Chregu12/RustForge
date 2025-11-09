//! Local filesystem storage backend

use crate::{Storage, StorageError};
use async_trait::async_trait;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Local filesystem storage
///
/// Stores files in the local filesystem with path security.
///
/// # Example
///
/// ```no_run
/// use rf_storage::{LocalStorage, Storage};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let storage = LocalStorage::new("./storage", "http://localhost:3000").await?;
///
/// storage.put("documents/test.pdf", b"PDF content".to_vec()).await?;
/// let url = storage.url("documents/test.pdf");
/// # Ok(())
/// # }
/// ```
pub struct LocalStorage {
    root: PathBuf,
    public_url: String,
}

impl LocalStorage {
    /// Create new local storage
    ///
    /// # Arguments
    ///
    /// * `root` - Root directory for file storage
    /// * `public_url` - Base URL for public file access
    pub async fn new(root: impl AsRef<Path>, public_url: impl Into<String>) -> Result<Self, StorageError> {
        let root = PathBuf::from(root.as_ref());

        // Create root directory if it doesn't exist
        if !root.exists() {
            fs::create_dir_all(&root).await?;
        }

        Ok(Self {
            root,
            public_url: public_url.into(),
        })
    }

    /// Resolve path and check for security violations
    fn resolve_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        let normalized = path.trim_start_matches('/');

        // Security: Check for path traversal patterns
        if normalized.contains("..") {
            return Err(StorageError::InvalidPath(
                "Path traversal detected".into(),
            ));
        }

        let full_path = self.root.join(normalized);

        // Double check: Ensure path starts with root
        // Use canonicalize only for the root to avoid issues with non-existent files
        let canonical_root = self.root.canonicalize()
            .unwrap_or_else(|_| self.root.clone());

        // Check if full_path would be under root when canonicalized
        if let Ok(canonical_full) = full_path.canonicalize() {
            if !canonical_full.starts_with(&canonical_root) {
                return Err(StorageError::InvalidPath(
                    "Path outside storage root".into(),
                ));
            }
        }

        Ok(full_path)
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn put(&self, path: &str, contents: Vec<u8>) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;

        // Create parent directories
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&full_path, contents).await?;

        tracing::debug!(
            path = %path,
            full_path = ?full_path,
            "File written to local storage"
        );

        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(StorageError::FileNotFound(path.into()));
        }

        Ok(fs::read(&full_path).await?)
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(StorageError::FileNotFound(path.into()));
        }

        fs::remove_file(&full_path).await?;

        tracing::debug!(path = %path, "File deleted from local storage");

        Ok(())
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        let full_path = self.resolve_path(path)?;
        Ok(full_path.exists())
    }

    async fn size(&self, path: &str) -> Result<u64, StorageError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(StorageError::FileNotFound(path.into()));
        }

        let metadata = fs::metadata(&full_path).await?;
        Ok(metadata.len())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let dir_path = self.resolve_path(prefix)?;

        if !dir_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&dir_path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                let relative_path = format!("{}/{}", prefix.trim_end_matches('/'), name);
                entries.push(relative_path);
            }
        }

        Ok(entries)
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
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_local_storage_put_get() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        storage
            .put("test.txt", b"Hello, World!".to_vec())
            .await
            .unwrap();

        let contents = storage.get("test.txt").await.unwrap();
        assert_eq!(contents, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_local_storage_exists() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        assert!(!storage.exists("test.txt").await.unwrap());

        storage.put("test.txt", b"content".to_vec()).await.unwrap();

        assert!(storage.exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_local_storage_delete() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        storage.put("test.txt", b"content".to_vec()).await.unwrap();
        assert!(storage.exists("test.txt").await.unwrap());

        storage.delete("test.txt").await.unwrap();
        assert!(!storage.exists("test.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_local_storage_size() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        storage.put("test.txt", b"Hello".to_vec()).await.unwrap();

        let size = storage.size("test.txt").await.unwrap();
        assert_eq!(size, 5);
    }

    #[tokio::test]
    async fn test_local_storage_list() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        storage.put("dir/file1.txt", b"1".to_vec()).await.unwrap();
        storage.put("dir/file2.txt", b"2".to_vec()).await.unwrap();

        let files = storage.list("dir").await.unwrap();
        assert_eq!(files.len(), 2);
    }

    #[tokio::test]
    async fn test_local_storage_path_traversal() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        // Attempt path traversal
        let result = storage.put("../etc/passwd", b"hack".to_vec()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_storage_url() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "https://example.com")
            .await
            .unwrap();

        let url = storage.url("documents/test.pdf");
        assert_eq!(url, "https://example.com/storage/documents/test.pdf");
    }

    #[tokio::test]
    async fn test_local_storage_nested_directories() {
        let dir = tempdir().unwrap();
        let storage = LocalStorage::new(dir.path(), "http://localhost:3000")
            .await
            .unwrap();

        // Create nested path
        storage
            .put("a/b/c/test.txt", b"nested".to_vec())
            .await
            .unwrap();

        let contents = storage.get("a/b/c/test.txt").await.unwrap();
        assert_eq!(contents, b"nested");
    }
}
