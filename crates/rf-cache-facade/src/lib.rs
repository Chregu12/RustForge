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
//! - **Flexible TTL**: Pass seconds as `u64` or `Duration`
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_cache_facade::Cache;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Put a value in cache - Laravel style with seconds!
//! Cache::put("key", "value", 3600).await?;
//!
//! // Get a value from cache
//! if let Some(value) = Cache::get::<String>("key").await? {
//!     println!("Cached value: {}", value);
//! }
//!
//! // Remember pattern - just pass seconds!
//! let value = Cache::remember("expensive_key", 3600, || async {
//!     Ok::<_, String>("expensive computation".to_string())
//! }).await?;
//!
//! // Add only if key doesn't exist
//! Cache::add("new_key", "value", 60).await?;
//!
//! // Store forever
//! Cache::forever("permanent", "value").await?;
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

pub use facade::{Cache, IntoTtl};
pub use manager::{CacheManager, GLOBAL_CACHE};

// Re-export commonly used types from rf-cache
pub use rf_cache::{CacheError, CacheResult, MemoryCache, TaggedCache};
