//! # rf-storage-facade
//!
//! Laravel-style Storage facade for the RustForge framework.
//!
//! ## Features
//!
//! - **Static Storage API**: Use `Storage::put()`, `Storage::get()`, etc. - no `.await` needed!
//! - **Global Storage Manager**: Thread-safe global storage state
//! - **Disk Support**: Multiple disk configurations
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_storage_facade::Storage;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Put a file
//! Storage::put("file.txt", b"Hello, World!".to_vec())?;
//!
//! // Get a file
//! let contents = Storage::get("file.txt")?;
//!
//! // Check if file exists
//! if Storage::exists("file.txt") {
//!     println!("File exists!");
//! }
//!
//! // Get file size
//! let size = Storage::size("file.txt")?;
//!
//! // Delete a file
//! Storage::delete("file.txt")?;
//! # Ok(())
//! # }
//! ```

pub mod facade;
pub mod manager;

pub use facade::Storage;
pub use manager::{StorageManager, GLOBAL_STORAGE};

// Re-export commonly used types from rf-storage
pub use rf_storage::{StorageError, StorageResult};
