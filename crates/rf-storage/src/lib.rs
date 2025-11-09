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
mod memory;
mod storage;

pub use error::{StorageError, StorageResult};
pub use memory::MemoryStorage;
pub use storage::Storage;

// Note: LocalStorage will be implemented in future updates
// pub use local::LocalStorage;
