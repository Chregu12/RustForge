use crate::manager::StorageManager;
use anyhow::Result;
use bytes::Bytes;
use std::path::Path;
use uuid::Uuid;

use std::sync::Arc;

pub struct FileService {
    storage_manager: Arc<StorageManager>,
}

impl FileService {
    pub fn new(storage_manager: Arc<StorageManager>) -> Self {
        Self { storage_manager }
    }

    pub async fn store(
        &self,
        contents: Bytes,
        original_name: &str,
        disk: Option<&str>,
    ) -> Result<String> {
        let disk = self.storage_manager.disk(disk)?;

        let extension = Path::new(original_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let filename = format!("{}.{}", Uuid::new_v4(), extension);
        let path = format!(
            "files/{}/{}",
            chrono::Local::now().format("%Y/%m/%d"),
            filename
        );

        disk.put(&path, contents).await?;

        Ok(path)
    }

    pub async fn store_in(
        &self,
        directory: &str,
        contents: Bytes,
        filename: &str,
        disk: Option<&str>,
    ) -> Result<String> {
        let disk = self.storage_manager.disk(disk)?;
        let path = format!("{}/{}", directory, filename);

        disk.put(&path, contents).await?;

        Ok(path)
    }

    pub async fn get(&self, path: &str, disk: Option<&str>) -> Result<Bytes> {
        let disk = self.storage_manager.disk(disk)?;
        disk.get(path).await
    }

    pub async fn exists(&self, path: &str, disk: Option<&str>) -> Result<bool> {
        let disk = self.storage_manager.disk(disk)?;
        disk.exists(path).await
    }

    pub async fn delete(&self, path: &str, disk: Option<&str>) -> Result<()> {
        let disk = self.storage_manager.disk(disk)?;
        disk.delete(path).await
    }

    pub fn url(&self, path: &str, disk: Option<&str>) -> Result<String> {
        let disk = self.storage_manager.disk(disk)?;
        Ok(disk.url(path))
    }

    pub async fn temporary_url(
        &self,
        path: &str,
        expires_in: chrono::Duration,
        disk: Option<&str>,
    ) -> Result<String> {
        let disk = self.storage_manager.disk(disk)?;
        disk.temporary_url(path, expires_in).await
    }

    pub async fn copy(&self, from: &str, to: &str, disk: Option<&str>) -> Result<()> {
        let disk = self.storage_manager.disk(disk)?;
        disk.copy(from, to).await
    }

    pub async fn move_file(&self, from: &str, to: &str, disk: Option<&str>) -> Result<()> {
        let disk = self.storage_manager.disk(disk)?;
        disk.move_file(from, to).await
    }
}
