//! File storage system for RustForge
//!
//! Provides a unified storage interface with multiple backend support.
//!
//! # Features
//!
//! - Storage trait for backend abstraction
//! - Local filesystem storage
//! - In-memory storage for testing
//! - Async file operations
//!
//! # Quick Start
//!
//! ```
//! use rf_storage::{MemoryStorage, Storage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = MemoryStorage::new();
//!
//! // Store file
//! storage.put("test.txt", b"Hello, World!".to_vec()).await?;
//!
//! // Check existence
//! assert!(storage.exists("test.txt").await?);
//!
//! // Get file
//! let contents = storage.get("test.txt").await?;
//! assert_eq!(contents, b"Hello, World!");
//! # Ok(())
//! # }
//! ```

mod error;
pub mod facade;
mod local;
mod manager;
mod memory;
mod s3;
pub mod service;
mod storage;
pub mod storage_manager;
mod stream;

pub use error::{StorageError, StorageResult};
pub use facade::StorageFacade;
pub use local::LocalStorage;
pub use manager::StorageManager;
pub use memory::MemoryStorage;
pub use s3::{S3Config, S3Storage};
pub use storage::Storage;
pub use storage_manager::{StorageManagerFacade, GLOBAL_STORAGE};
pub use stream::{detect_content_type, extract_file_name, FileStream};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        detect_content_type, extract_file_name, FileStream, LocalStorage, MemoryStorage, S3Config,
        S3Storage, Storage, StorageError, StorageManager, StorageResult,
    };
}
