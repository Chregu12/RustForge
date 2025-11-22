//! Cache driver implementations
//!
//! This module contains various cache backend drivers for different storage systems.

#[cfg(feature = "memcached")]
pub mod memcached;

#[cfg(feature = "database")]
pub mod database;

#[cfg(feature = "file")]
pub mod file;

// Re-exports
#[cfg(feature = "memcached")]
pub use memcached::MemcachedDriver;

#[cfg(feature = "database")]
pub use database::DatabaseDriver;

#[cfg(feature = "file")]
pub use file::FileDriver;
