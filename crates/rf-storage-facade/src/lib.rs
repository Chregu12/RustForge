//! # rf-storage-facade
//!
//! Laravel-style `Storage` facade for the RustForge framework.
//!
//! This crate used to carry its **own** duplicate `Storage`/`StorageManager`
//! implementation backed by a *separate* in-memory `HashMap` global — data was
//! never written to disk and it lived in a different global than `rf::Storage`
//! (which resolves to `rf_storage::StorageFacade`). The `storage!` helper macro
//! expands to `rf_storage_facade::Storage::…`, so it hit that mock store instead
//! of the real one — a split-brain bug.
//!
//! It now simply **re-exports the single real implementation from
//! [`rf_storage`]** (which persists to real files on disk through the global
//! storage manager), so there is exactly one source of truth shared by the
//! prelude facade and the `storage!` macro.
//!
//! # Recommended Usage
//!
//! Prefer the consolidated `rf` crate (`use rf::Storage;`). When depending on
//! this crate directly:
//!
//! ```rust
//! use rf_storage_facade::Storage;
//! ```

// One source of truth: the real Storage facade + global manager live in
// `rf-storage`. The real facade is named `StorageFacade`; re-export it under the
// historical `Storage` name so `rf_storage_facade::Storage::…` (emitted by the
// `storage!` macro and used by `rustforge::Storage`) keeps resolving.
pub use rf_storage::StorageFacade as Storage;
pub use rf_storage::{StorageManager, GLOBAL_STORAGE};

// Re-export commonly used types from rf-storage (kept for API stability).
pub use rf_storage::{StorageError, StorageResult};
