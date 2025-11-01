pub mod config;
pub mod local;
pub mod manager;
pub mod service;

use anyhow::Result;
use async_trait::async_trait;
use bytes::Bytes;

#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub mime_type: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub modified_at: chrono::DateTime<chrono::Utc>,
}

#[async_trait]
pub trait Storage: Send + Sync + std::fmt::Debug {
    async fn put(&self, path: &str, contents: Bytes) -> Result<()>;
    async fn get(&self, path: &str) -> Result<Bytes>;
    async fn exists(&self, path: &str) -> Result<bool>;
    async fn delete(&self, path: &str) -> Result<()>;
    async fn metadata(&self, path: &str) -> Result<FileMetadata>;
    async fn list(&self, directory: &str) -> Result<Vec<FileMetadata>>;
    async fn temporary_url(&self, path: &str, expires_in: chrono::Duration) -> Result<String>;
    fn url(&self, path: &str) -> String;
    async fn copy(&self, from: &str, to: &str) -> Result<()>;
    async fn move_file(&self, from: &str, to: &str) -> Result<()>;
}

#[derive(Debug, Clone)]
pub enum StorageDriver {
    Local,
    S3,
    Azure,
    Google,
}

#[derive(Debug, Clone)]
pub struct Disk {
    name: String,
    driver: StorageDriver,
    storage: std::sync::Arc<dyn Storage>,
}

impl Disk {
    pub fn new(name: String, driver: StorageDriver, storage: std::sync::Arc<dyn Storage>) -> Self {
        Self {
            name,
            driver,
            storage,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn driver(&self) -> &StorageDriver {
        &self.driver
    }

    pub async fn put(&self, path: &str, contents: Bytes) -> Result<()> {
        self.storage.put(path, contents).await
    }

    pub async fn get(&self, path: &str) -> Result<Bytes> {
        self.storage.get(path).await
    }

    pub async fn exists(&self, path: &str) -> Result<bool> {
        self.storage.exists(path).await
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        self.storage.delete(path).await
    }

    pub async fn metadata(&self, path: &str) -> Result<FileMetadata> {
        self.storage.metadata(path).await
    }

    pub fn url(&self, path: &str) -> String {
        self.storage.url(path)
    }

    pub async fn temporary_url(&self, path: &str, expires_in: chrono::Duration) -> Result<String> {
        self.storage.temporary_url(path, expires_in).await
    }

    pub async fn copy(&self, from: &str, to: &str) -> Result<()> {
        self.storage.copy(from, to).await
    }

    pub async fn move_file(&self, from: &str, to: &str) -> Result<()> {
        self.storage.move_file(from, to).await
    }
}
