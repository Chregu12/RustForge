//! # rf-storage Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-storage.
//!
//! ## Usage
//!
//! ```rust
//! use rf_storage::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: error::{StorageError, StorageResult};
pub use crate:: local::LocalStorage;
pub use crate:: manager::StorageManager;
pub use crate:: memory::MemoryStorage;
pub use crate:: s3::{S3Config, S3Storage};
pub use crate:: storage::Storage;
pub use crate:: stream::{detect_content_type, extract_file_name, FileStream};
