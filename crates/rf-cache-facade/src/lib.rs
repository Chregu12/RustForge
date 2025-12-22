//! # rf-cache-facade
//!
//! Laravel-style Cache facade for the RustForge framework.
//!
//! # Recommended Usage
//!
//! Use the consolidated `rf` crate for simpler imports:
//! ```rust
//! use rf::Cache;  // or use rf::prelude::*;
//! ```
//!
//! ## Features
//!
//! - **Static Cache API**: Use `Cache::get()`, `Cache::put()`, etc.
//! - **Global Cache Manager**: Thread-safe global cache state
//! - **Tag Support**: Group cache entries with tags
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//! - **Flexible TTL**: Pass seconds as `u64` or `Duration`
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! // Recommended: use rf::Cache;
//! use rf_cache_facade::Cache;  // Direct import also works
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Put a value in cache - Laravel style with seconds!
//! Cache::put("key", "value", 3600)?;
//!
//! // Get a value from cache
//! if let Some(value) = Cache::get::<String>("key")? {
//!     println!("Cached value: {}", value);
//! }
//!
//! // Remember pattern - just pass seconds!
//! let value = Cache::remember("expensive_key", 3600, || async {
//!     Ok::<_, String>("expensive computation".to_string())
//! })?;
//!
//! // Add only if key doesn't exist
//! Cache::add("new_key", "value", 60)?;
//!
//! // Store forever
//! Cache::forever("permanent", "value")?;
//!
//! // Forget a value
//! Cache::forget("key")?;
//!
//! // Flush all cache
//! Cache::flush()?;
//! # Ok(())
//! # }
//! ```

pub mod facade;
pub mod manager;

pub use facade::{Cache, IntoTtl};
pub use manager::{CacheManager, GLOBAL_CACHE};

// Re-export commonly used types from rf-cache
pub use rf_cache::{CacheError, CacheResult, MemoryCache, TaggedCache};
