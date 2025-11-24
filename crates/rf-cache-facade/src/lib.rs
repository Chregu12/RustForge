//! # rf-cache-facade
//!
//! Laravel-style Cache facade for the RustForge framework.
//!
//! ## Features
//!
//! - **Static Cache API**: Use `Cache::get()`, `Cache::put()`, etc.
//! - **Global Cache Manager**: Thread-safe global cache state
//! - **Tag Support**: Group cache entries with tags
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_cache_facade::Cache;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Put a value in cache
//! Cache::put("key", "value", Duration::from_secs(3600)).await?;
//!
//! // Get a value from cache
//! if let Some(value) = Cache::get::<String>("key").await? {
//!     println!("Cached value: {}", value);
//! }
//!
//! // Remember pattern
//! let value = Cache::remember("expensive_key", Duration::from_secs(3600), || async {
//!     Ok::<_, String>("expensive computation".to_string())
//! }).await?;
//!
//! // Forget a value
//! Cache::forget("key").await?;
//!
//! // Flush all cache
//! Cache::flush().await?;
//! # Ok(())
//! # }
//! ```

pub mod facade;
pub mod manager;

pub use facade::Cache;
pub use manager::{CacheManager, GLOBAL_CACHE};

// Re-export commonly used types from rf-cache
pub use rf_cache::{CacheError, CacheResult, MemoryCache, TaggedCache};
