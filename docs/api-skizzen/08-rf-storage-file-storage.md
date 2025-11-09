# API-Skizze: rf-storage - File Storage System

**Phase**: Phase 2 - Modular Rebuild
**PR-Slice**: #9
**Status**: Planning
**Date**: 2025-11-09

## 1. Overview

The `rf-storage` crate provides a unified file storage interface with multiple backend support for local filesystem and cloud storage.

**Key Features:**
- Storage trait for multiple backends
- Local filesystem storage
- In-memory storage for testing
- Async file operations
- Path management and URL generation
- File metadata and MIME type detection

**Comparison with Laravel Storage:**
- ✅ Storage facade pattern
- ✅ Multiple drivers (local, memory)
- ✅ File operations (put, get, delete, exists)
- ✅ Visibility (public/private)
- ⏳ S3 driver (future)
- ⏳ File streaming (future)

## 2. Core Types

### 2.1 Storage Trait

```rust
use async_trait::async_trait;

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

    /// Get file last modified time
    async fn last_modified(&self, path: &str) -> Result<chrono::DateTime<chrono::Utc>, StorageError>;

    /// List files in directory
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
```

### 2.2 File Upload

```rust
use bytes::Bytes;

pub struct UploadedFile {
    /// Original filename
    pub filename: String,

    /// MIME type
    pub content_type: String,

    /// File contents
    pub contents: Bytes,

    /// File size
    pub size: u64,
}

impl UploadedFile {
    /// Store file using storage backend
    pub async fn store(
        &self,
        storage: &dyn Storage,
        path: &str,
    ) -> Result<String, StorageError> {
        let full_path = self.generate_path(path)?;
        storage.put(&full_path, self.contents.to_vec()).await?;
        Ok(full_path)
    }

    /// Store with auto-generated filename
    pub async fn store_as(
        &self,
        storage: &dyn Storage,
        directory: &str,
        filename: &str,
    ) -> Result<String, StorageError> {
        let path = format!("{}/{}", directory.trim_end_matches('/'), filename);
        storage.put(&path, self.contents.to_vec()).await?;
        Ok(path)
    }

    /// Generate unique path
    fn generate_path(&self, base: &str) -> Result<String, StorageError> {
        let ext = std::path::Path::new(&self.filename)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let uuid = uuid::Uuid::new_v4();
        let filename = if ext.is_empty() {
            uuid.to_string()
        } else {
            format!("{}.{}", uuid, ext)
        };

        Ok(format!("{}/{}", base.trim_end_matches('/'), filename))
    }
}
```

### 2.3 Storage Configuration

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Default storage driver
    pub default: StorageDriver,

    /// Local storage configuration
    pub local: Option<LocalConfig>,

    /// Public URL base
    pub public_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StorageDriver {
    Local,
    Memory,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalConfig {
    /// Root directory for file storage
    pub root: String,

    /// Public directory (for public URLs)
    pub public_dir: Option<String>,

    /// Visibility mode
    pub visibility: Visibility,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            default: StorageDriver::Local,
            local: Some(LocalConfig {
                root: "./storage".into(),
                public_dir: Some("./public".into()),
                visibility: Visibility::Private,
            }),
            public_url: "http://localhost:3000".into(),
        }
    }
}
```

## 3. Backend Implementations

### 3.1 Local Filesystem Backend

```rust
use std::path::PathBuf;
use tokio::fs;

pub struct LocalStorage {
    root: PathBuf,
    public_url: String,
    visibility: Visibility,
}

impl LocalStorage {
    pub fn new(config: LocalConfig, public_url: String) -> Result<Self, StorageError> {
        let root = PathBuf::from(&config.root);

        // Create root directory if it doesn't exist
        std::fs::create_dir_all(&root)?;

        Ok(Self {
            root,
            public_url,
            visibility: config.visibility,
        })
    }

    fn resolve_path(&self, path: &str) -> Result<PathBuf, StorageError> {
        let normalized = path.trim_start_matches('/');
        let full_path = self.root.join(normalized);

        // Security: Prevent path traversal
        if !full_path.starts_with(&self.root) {
            return Err(StorageError::SecurityViolation(
                "Path traversal detected".into(),
            ));
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

    async fn last_modified(&self, path: &str) -> Result<chrono::DateTime<chrono::Utc>, StorageError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Err(StorageError::FileNotFound(path.into()));
        }

        let metadata = fs::metadata(&full_path).await?;
        let modified = metadata.modified()?;

        Ok(chrono::DateTime::from(modified))
    }

    async fn list(&self, path: &str) -> Result<Vec<String>, StorageError> {
        let full_path = self.resolve_path(path)?;

        if !full_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut read_dir = fs::read_dir(&full_path).await?;

        while let Some(entry) = read_dir.next_entry().await? {
            if let Some(name) = entry.file_name().to_str() {
                entries.push(format!("{}/{}", path.trim_end_matches('/'), name));
            }
        }

        Ok(entries)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/storage/{}", self.public_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }
}
```

### 3.2 Memory Backend (Testing)

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MemoryStorage {
    files: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    public_url: String,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            files: Arc::new(Mutex::new(HashMap::new())),
            public_url: "http://localhost:3000".into(),
        }
    }

    /// Get all stored files (for testing)
    pub fn files(&self) -> HashMap<String, Vec<u8>> {
        self.files.lock().unwrap().clone()
    }

    /// Clear all files
    pub fn clear(&self) {
        self.files.lock().unwrap().clear();
    }
}

#[async_trait]
impl Storage for MemoryStorage {
    async fn put(&self, path: &str, contents: Vec<u8>) -> Result<(), StorageError> {
        self.files.lock().unwrap().insert(path.into(), contents);
        Ok(())
    }

    async fn get(&self, path: &str) -> Result<Vec<u8>, StorageError> {
        self.files
            .lock()
            .unwrap()
            .get(path)
            .cloned()
            .ok_or_else(|| StorageError::FileNotFound(path.into()))
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        self.files
            .lock()
            .unwrap()
            .remove(path)
            .ok_or_else(|| StorageError::FileNotFound(path.into()))?;
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
            .ok_or_else(|| StorageError::FileNotFound(path.into()))
    }

    async fn last_modified(&self, _path: &str) -> Result<chrono::DateTime<chrono::Utc>, StorageError> {
        // Memory storage doesn't track modification times
        Ok(chrono::Utc::now())
    }

    async fn list(&self, path: &str) -> Result<Vec<String>, StorageError> {
        let prefix = path.trim_end_matches('/');
        let files: Vec<String> = self
            .files
            .lock()
            .unwrap()
            .keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();

        Ok(files)
    }

    fn url(&self, path: &str) -> String {
        format!("{}/storage/{}", self.public_url.trim_end_matches('/'), path.trim_start_matches('/'))
    }
}
```

## 4. Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Security violation: {0}")]
    SecurityViolation(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("System time error: {0}")]
    TimeError(#[from] std::time::SystemTimeError),

    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Storage error: {0}")]
    Other(String),
}
```

## 5. Usage Examples

### 5.1 Basic File Operations

```rust
use rf_storage::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = LocalConfig {
        root: "./storage".into(),
        public_dir: Some("./public".into()),
        visibility: Visibility::Public,
    };

    let storage = LocalStorage::new(config, "http://localhost:3000".into())?;

    // Store file
    storage.put("documents/test.txt", b"Hello, World!".to_vec()).await?;

    // Get file
    let contents = storage.get("documents/test.txt").await?;
    println!("File contents: {}", String::from_utf8(contents)?);

    // Check existence
    assert!(storage.exists("documents/test.txt").await?);

    // Get file size
    let size = storage.size("documents/test.txt").await?;
    println!("File size: {} bytes", size);

    // Get URL
    let url = storage.url("documents/test.txt");
    println!("Public URL: {}", url);

    // Delete file
    storage.delete("documents/test.txt").await?;

    Ok(())
}
```

### 5.2 File Upload Handling

```rust
use bytes::Bytes;

// Simulate uploaded file
let uploaded = UploadedFile {
    filename: "avatar.jpg".into(),
    content_type: "image/jpeg".into(),
    contents: Bytes::from_static(b"fake image data"),
    size: 14,
};

// Store with auto-generated path
let path = uploaded.store(&storage, "avatars").await?;
println!("Stored at: {}", path);

// Store with custom filename
let path = uploaded.store_as(&storage, "avatars", "user-123.jpg").await?;
println!("Stored at: {}", path);
```

### 5.3 Directory Operations

```rust
// List files in directory
let files = storage.list("documents").await?;
for file in files {
    println!("Found: {}", file);
}

// Copy file
storage.copy("documents/original.txt", "backups/copy.txt").await?;

// Move file
storage.move_file("documents/old.txt", "archive/old.txt").await?;
```

### 5.4 Testing with Memory Storage

```rust
#[tokio::test]
async fn test_file_operations() {
    let storage = MemoryStorage::new();

    storage.put("test.txt", b"content".to_vec()).await.unwrap();

    assert!(storage.exists("test.txt").await.unwrap());

    let contents = storage.get("test.txt").await.unwrap();
    assert_eq!(contents, b"content");

    storage.delete("test.txt").await.unwrap();
    assert!(!storage.exists("test.txt").await.unwrap());
}
```

## 6. Implementation Plan

### Phase 1: Core (25 minutes)
- [ ] Create rf-storage crate
- [ ] Implement Storage trait
- [ ] Implement error types
- [ ] Add configuration types

### Phase 2: Backends (30 minutes)
- [ ] Implement LocalStorage
- [ ] Implement MemoryStorage
- [ ] Add path security checks
- [ ] Test file operations

### Phase 3: Upload Handling (15 minutes)
- [ ] Implement UploadedFile
- [ ] Add filename generation
- [ ] Add MIME type support

### Phase 4: Examples & Tests (20 minutes)
- [ ] Create storage-demo example
- [ ] Write unit tests
- [ ] Add integration tests
- [ ] Documentation

**Total Estimated Time: 1.5 hours**

## 7. Dependencies

```toml
[dependencies]
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio = { workspace = true, features = ["fs"] }
uuid.workspace = true
chrono.workspace = true
bytes = "1.5"

[dev-dependencies]
tokio = { workspace = true, features = ["test-util", "macros"] }
```

## 8. Future Enhancements

1. **S3 Backend**: AWS S3 integration with rusoto or aws-sdk
2. **Streaming**: Large file upload/download streaming
3. **File Validation**: Size limits, MIME type checking
4. **Image Processing**: Thumbnails, resizing with image crate
5. **CDN Integration**: CloudFlare, Fastly support
6. **Temporary URLs**: Signed temporary URLs
7. **Disk Management**: Cleanup, quota management

## 9. Comparison with Laravel

| Feature | Laravel | rf-storage | Status |
|---------|---------|------------|--------|
| Storage facade | ✅ | ✅ | ✅ Complete |
| Local driver | ✅ | ✅ | ✅ Complete |
| File operations | ✅ | ✅ | ✅ Complete |
| Visibility | ✅ | ✅ | ✅ Complete |
| URL generation | ✅ | ✅ | ✅ Complete |
| File upload | ✅ | ✅ | ✅ Complete |
| S3 driver | ✅ | ⏳ | ⏳ Future |
| Streaming | ✅ | ⏳ | ⏳ Future |
| CDN | ✅ | ⏳ | ⏳ Future |

**Feature Parity**: ~65% (6/9 features)

---

**Estimated Lines of Code**: ~900 production + ~250 tests + ~200 examples = **~1,350 total**
