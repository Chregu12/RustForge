//! Stub implementations for types that were in rf-infra
//! These provide default in-memory implementations for development

use async_trait::async_trait;
use rf_plugins::{
    CachePort, CommandError, DomainEvent, EventPort, QueueJob, QueuePort, StoragePort,
    StoredFile, ValidationPort, ValidationRules, ValidationReport,
};
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::time::Duration;

// ============================================================================
// Storage
// ============================================================================

/// Storage configuration
#[derive(Clone, Debug)]
pub struct StorageConfig {
    pub default_disk: String,
    pub base_path: PathBuf,
}

impl StorageConfig {
    pub fn from_env() -> Self {
        Self {
            default_disk: std::env::var("STORAGE_DISK").unwrap_or_else(|_| "local".into()),
            base_path: std::env::var("STORAGE_PATH")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("./storage")),
        }
    }
}

/// Storage manager
pub struct StorageManager {
    config: StorageConfig,
}

impl StorageManager {
    pub fn new(config: StorageConfig) -> Result<Self, String> {
        Ok(Self { config })
    }

    pub fn base_path(&self) -> &PathBuf {
        &self.config.base_path
    }

    pub fn default_disk(&self) -> &str {
        &self.config.default_disk
    }
}

/// File storage adapter
pub struct FileStorageAdapter {
    manager: Arc<StorageManager>,
}

impl FileStorageAdapter {
    pub fn new(manager: Arc<StorageManager>) -> Self {
        Self { manager }
    }
}

#[async_trait]
impl StoragePort for FileStorageAdapter {
    async fn put(
        &self,
        disk: &str,
        path: &str,
        contents: Vec<u8>,
    ) -> Result<StoredFile, CommandError> {
        let full_path = self.manager.base_path().join(disk).join(path);
        if let Some(parent) = full_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| CommandError::Message(format!("Storage error: {}", e)))?;
        }
        std::fs::write(&full_path, &contents)
            .map_err(|e| CommandError::Message(format!("Storage error: {}", e)))?;

        Ok(StoredFile {
            disk: disk.to_string(),
            path: path.to_string(),
            size: contents.len() as u64,
            url: None,
        })
    }

    async fn get(&self, disk: &str, path: &str) -> Result<Vec<u8>, CommandError> {
        let full_path = self.manager.base_path().join(disk).join(path);
        std::fs::read(&full_path)
            .map_err(|e| CommandError::Message(format!("Storage error: {}", e)))
    }

    async fn delete(&self, disk: &str, path: &str) -> Result<(), CommandError> {
        let full_path = self.manager.base_path().join(disk).join(path);
        std::fs::remove_file(&full_path)
            .map_err(|e| CommandError::Message(format!("Storage error: {}", e)))
    }

    async fn exists(&self, disk: &str, path: &str) -> Result<bool, CommandError> {
        let full_path = self.manager.base_path().join(disk).join(path);
        Ok(full_path.exists())
    }

    async fn url(&self, disk: &str, path: &str) -> Result<String, CommandError> {
        Ok(format!("/storage/{}/{}", disk, path))
    }
}

// ============================================================================
// Validation
// ============================================================================

/// Simple validation service
pub struct SimpleValidationService;

#[async_trait]
impl ValidationPort for SimpleValidationService {
    async fn validate(
        &self,
        _payload: Value,
        _rules: ValidationRules,
    ) -> Result<ValidationReport, CommandError> {
        Ok(ValidationReport {
            valid: true,
            errors: Vec::new(),
        })
    }
}

// ============================================================================
// Cache
// ============================================================================

/// In-memory cache store
#[derive(Default)]
pub struct InMemoryCacheStore {
    data: RwLock<HashMap<String, Value>>,
}

#[async_trait]
impl CachePort for InMemoryCacheStore {
    async fn get(&self, key: &str) -> Result<Option<Value>, CommandError> {
        let data = self
            .data
            .read()
            .map_err(|e| CommandError::Message(format!("Cache error: {}", e)))?;
        Ok(data.get(key).cloned())
    }

    async fn put(
        &self,
        key: &str,
        value: Value,
        _ttl: Option<Duration>,
    ) -> Result<(), CommandError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| CommandError::Message(format!("Cache error: {}", e)))?;
        data.insert(key.to_string(), value);
        Ok(())
    }

    async fn forget(&self, key: &str) -> Result<(), CommandError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| CommandError::Message(format!("Cache error: {}", e)))?;
        data.remove(key);
        Ok(())
    }

    async fn clear(&self, prefix: Option<&str>) -> Result<(), CommandError> {
        let mut data = self
            .data
            .write()
            .map_err(|e| CommandError::Message(format!("Cache error: {}", e)))?;
        if let Some(prefix) = prefix {
            data.retain(|k, _| !k.starts_with(prefix));
        } else {
            data.clear();
        }
        Ok(())
    }
}

// ============================================================================
// Queue
// ============================================================================

/// In-memory queue
#[derive(Default)]
pub struct InMemoryQueue {
    jobs: RwLock<Vec<QueueJob>>,
}

#[async_trait]
impl QueuePort for InMemoryQueue {
    async fn dispatch(&self, job: QueueJob) -> Result<(), CommandError> {
        let mut jobs = self
            .jobs
            .write()
            .map_err(|e| CommandError::Message(format!("Queue error: {}", e)))?;
        jobs.push(job);
        Ok(())
    }
}

// ============================================================================
// Events
// ============================================================================

/// In-memory event bus
#[derive(Default)]
pub struct InMemoryEventBus {
    events: RwLock<Vec<DomainEvent>>,
}

#[async_trait]
impl EventPort for InMemoryEventBus {
    async fn publish(&self, event: DomainEvent) -> Result<(), CommandError> {
        let mut events = self
            .events
            .write()
            .map_err(|e| CommandError::Message(format!("Event error: {}", e)))?;
        events.push(event);
        Ok(())
    }
}
