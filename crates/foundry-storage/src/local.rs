use crate::{FileMetadata, Storage};
use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;
use std::path::PathBuf;
use tokio::fs;

#[derive(Debug)]
pub struct LocalStorage {
    root: PathBuf,
    url_base: String,
}

impl LocalStorage {
    pub fn new(root: impl Into<String>, url_base: impl Into<String>) -> Self {
        Self {
            root: PathBuf::from(root.into()),
            url_base: url_base.into(),
        }
    }

    fn full_path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }

    async fn ensure_directory(&self, path: &str) -> Result<()> {
        if let Some(parent) = self.full_path(path).parent() {
            fs::create_dir_all(parent).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl Storage for LocalStorage {
    async fn put(&self, path: &str, contents: Bytes) -> Result<()> {
        self.ensure_directory(path).await?;
        let full_path = self.full_path(path);
        fs::write(&full_path, &contents).await?;
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Bytes> {
        let full_path = self.full_path(path);
        let contents = fs::read(&full_path).await?;
        Ok(Bytes::from(contents))
    }

    async fn exists(&self, path: &str) -> Result<bool> {
        let full_path = self.full_path(path);
        Ok(fs::try_exists(&full_path).await.unwrap_or(false))
    }

    async fn delete(&self, path: &str) -> Result<()> {
        let full_path = self.full_path(path);
        if self.exists(path).await? {
            fs::remove_file(&full_path).await?;
        }
        Ok(())
    }

    async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        let full_path = self.full_path(path);
        let meta = fs::metadata(&full_path).await?;

        let mime_type = mime_guess::from_path(&full_path)
            .first_or_octet_stream()
            .to_string();

        let modified = meta.modified()?;
        let system_time = std::time::SystemTime::now();

        Ok(FileMetadata {
            path: path.to_string(),
            size: meta.len(),
            mime_type,
            created_at: chrono::DateTime::<chrono::Utc>::from(system_time),
            modified_at: chrono::DateTime::<chrono::Utc>::from(modified),
        })
    }

    async fn list(&self, directory: &str) -> Result<Vec<FileMetadata>> {
        let full_path = self.full_path(directory);
        let mut entries = Vec::new();

        let mut dir = fs::read_dir(&full_path).await?;
        while let Some(entry) = dir.next_entry().await? {
            let path = entry.path();
            if let Ok(rel_path) = path.strip_prefix(&self.root) {
                if let Ok(meta) = entry.metadata().await {
                    let mime_type = mime_guess::from_path(&path)
                        .first_or_octet_stream()
                        .to_string();

                    let modified = meta.modified()?;

                    entries.push(FileMetadata {
                        path: rel_path.to_string_lossy().to_string(),
                        size: meta.len(),
                        mime_type,
                        created_at: chrono::DateTime::<chrono::Utc>::from(
                            std::time::SystemTime::now(),
                        ),
                        modified_at: chrono::DateTime::<chrono::Utc>::from(modified),
                    });
                }
            }
        }

        Ok(entries)
    }

    async fn temporary_url(&self, path: &str, _expires_in: chrono::Duration) -> Result<String> {
        // For local storage, temporary URLs are just regular URLs
        Ok(self.url(path))
    }

    fn url(&self, path: &str) -> String {
        format!("{}/{}", self.url_base, path)
    }

    async fn copy(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.full_path(from);
        let to_path = self.full_path(to);

        self.ensure_directory(to).await?;
        fs::copy(&from_path, &to_path).await?;
        Ok(())
    }

    async fn move_file(&self, from: &str, to: &str) -> Result<()> {
        let from_path = self.full_path(from);
        let to_path = self.full_path(to);

        self.ensure_directory(to).await?;
        fs::rename(&from_path, &to_path).await?;
        Ok(())
    }
}
