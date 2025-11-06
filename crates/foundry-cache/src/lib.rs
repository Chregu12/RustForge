//! Foundry Cache - Multi-backend caching system
//!
//! This crate provides a comprehensive caching system for the Foundry framework.
//!
//! # Features
//!
//! - **Multiple Backends**: Redis, File, In-Memory
//! - **Type-Safe API**: Generic methods for serializable types
//! - **TTL Support**: Configurable time-to-live for cache entries
//! - **Atomic Operations**: Increment/decrement counters
//! - **Batch Operations**: Get/set multiple values at once
//! - **Statistics**: Track cache hits, misses, and performance
//!
//! # Example
//!
//! ```no_run
//! use foundry_cache::prelude::*;
//! use std::time::Duration;
//!
//! # async fn example() -> Result<(), CacheError> {
//! // Create cache manager from environment
//! let cache = CacheManager::from_env()?;
//!
//! // Set a value
//! cache.set("user:1", &"John Doe", Some(Duration::from_secs(3600))).await?;
//!
//! // Get a value
//! let name: Option<String> = cache.get("user:1").await?;
//!
//! // Remember (get or compute and cache)
//! let value = cache.remember("expensive", Duration::from_secs(60), || {
//!     // Expensive computation
//!     42
//! }).await?;
//!
//! # Ok(())
//! # }
//! ```

pub mod store;
pub mod stores;
pub mod manager;
pub mod tags;
pub mod zero_copy;

pub use store::{CacheStore, CacheValue, CacheStats, CacheError};
pub use stores::{MemoryStore, RedisStore, FileStore};
pub use manager::{CacheManager, CacheConfig};
pub use tags::TaggedCache;
pub use zero_copy::{ZeroCopyCache, CachedData, ZeroCopyError};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::store::{CacheStore, CacheValue, CacheStats, CacheError};
    pub use crate::stores::{MemoryStore, RedisStore, FileStore};
    pub use crate::manager::{CacheManager, CacheConfig};
    pub use crate::tags::TaggedCache;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_basic_caching() {
        let cache = MemoryStore::new();

        let value = CacheValue::from_string("test_value");
        cache.set("test_key", value.clone(), Some(Duration::from_secs(60)))
            .await
            .unwrap();

        let result = cache.get("test_key").await.unwrap();
        assert!(result.is_some());
    }
}
