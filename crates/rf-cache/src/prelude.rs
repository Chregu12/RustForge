//! # rf-cache Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-cache.
//!
//! ## Usage
//!
//! ```rust
//! use rf_cache::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: config::{CacheBackend, CacheConfig, CacheConfigBuilder};
pub use crate:: redis::{RedisCache, RedisTaggedCache};
