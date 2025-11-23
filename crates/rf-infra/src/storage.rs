use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use rf_plugins::{CommandError, StoragePort, StoredFile};
use rf_storage::{Storage, StorageManager};
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct FileStorageAdapter {
    manager: Arc<StorageManager>,
}

impl FileStorageAdapter {
    pub fn new(manager: Arc<StorageManager>) -> Self {
        Self { manager }
    }

    fn disk(&self, name: &str) -> Result<Arc<dyn Storage>, CommandError> {
        self.manager.disk(name)
            .map(|d| d.clone())
            .map_err(|e| CommandError::Other(e.into()))
    }
}

#[async_trait]
impl StoragePort for FileStorageAdapter {
    async fn put(
        &self,
        disk_name: &str,
        path: &str,
        contents: Vec<u8>,
    ) -> Result<StoredFile, CommandError> {
        let disk = self.disk(disk_name)?;
        disk.put(path, contents.clone())
            .await
            .map_err(|e| CommandError::Other(e.into()))?;

        Ok(StoredFile {
            disk: disk_name.to_string(),
            path: path.to_string(),
            size: contents.len() as u64,
            url: Some(format!("storage://{}/{}", disk_name, path)),
        })
    }

    async fn get(&self, disk_name: &str, path: &str) -> Result<Vec<u8>, CommandError> {
        let disk = self.disk(disk_name)?;
        disk.get(path).await.map_err(|e| CommandError::Other(e.into()))
    }

    async fn delete(&self, disk_name: &str, path: &str) -> Result<(), CommandError> {
        let disk = self.disk(disk_name)?;
        disk.delete(path).await.map_err(|e| CommandError::Other(e.into()))
    }

    async fn exists(&self, disk_name: &str, path: &str) -> Result<bool, CommandError> {
        let disk = self.disk(disk_name)?;
        disk.exists(path).await.map_err(|e| CommandError::Other(e.into()))
    }

    async fn url(&self, disk_name: &str, path: &str) -> Result<String, CommandError> {
        Ok(format!("storage://{}/{}", disk_name, path))
    }
}

#[derive(Clone, Default)]
pub struct InMemoryStorage {
    inner: Arc<RwLock<HashMap<String, HashMap<String, Vec<u8>>>>>,
}

impl InMemoryStorage {
    fn disk_mut<'a>(
        &'a self,
        guard: &'a mut HashMap<String, HashMap<String, Vec<u8>>>,
        disk: &str,
    ) -> &'a mut HashMap<String, Vec<u8>> {
        guard.entry(disk.to_string()).or_default()
    }
}

#[async_trait]
impl StoragePort for InMemoryStorage {
    async fn put(
        &self,
        disk: &str,
        path: &str,
        contents: Vec<u8>,
    ) -> Result<StoredFile, CommandError> {
        let mut guard = self.inner.write().await;
        let bucket = self.disk_mut(&mut guard, disk);
        bucket.insert(path.to_string(), contents.clone());

        Ok(StoredFile {
            disk: disk.to_string(),
            path: path.to_string(),
            size: contents.len() as u64,
            url: Some(format!("memory://{disk}/{path}")),
        })
    }

    async fn get(&self, disk: &str, path: &str) -> Result<Vec<u8>, CommandError> {
        let guard = self.inner.read().await;
        guard
            .get(disk)
            .and_then(|bucket| bucket.get(path).cloned())
            .ok_or_else(|| {
                CommandError::Message(format!("File `{disk}:{path}` not found in memory storage"))
            })
    }

    async fn delete(&self, disk: &str, path: &str) -> Result<(), CommandError> {
        let mut guard = self.inner.write().await;
        if let Some(bucket) = guard.get_mut(disk) {
            bucket.remove(path);
        }
        Ok(())
    }

    async fn exists(&self, disk: &str, path: &str) -> Result<bool, CommandError> {
        let guard = self.inner.read().await;
        Ok(guard
            .get(disk)
            .and_then(|bucket| bucket.get(path))
            .is_some())
    }

    async fn url(&self, disk: &str, path: &str) -> Result<String, CommandError> {
        Ok(format!("memory://{disk}/{path}"))
    }
}
