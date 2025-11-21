//! # Database Sharding Module
//!
//! Horizontal database partitioning for scaling and multi-tenancy.
//!
//! ## Overview
//!
//! Database sharding distributes data across multiple database instances based on a sharding key.
//! This module provides:
//!
//! - Shard manager for connection management
//! - Multiple sharding strategies (hash, range, tenant-based)
//! - Shardable trait for models
//! - Cross-shard query support
//!
//! ## Use Cases
//!
//! 1. **Multi-tenancy**: Each tenant's data on a separate shard
//! 2. **Horizontal scaling**: Distribute users across multiple databases
//! 3. **Geographic distribution**: Route requests to regional databases
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::sharding::*;
//! use std::sync::Arc;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create sharding strategy
//! let strategy = HashStrategy::new(vec![
//!     "shard1".to_string(),
//!     "shard2".to_string(),
//! ]);
//!
//! // Create shard manager
//! let mut manager = ShardManager::new(Arc::new(strategy));
//!
//! // Register shards
//! manager.add_shard("shard1".to_string(), Arc::new(db1));
//! manager.add_shard("shard2".to_string(), Arc::new(db2));
//!
//! // Use sharding
//! let user_id = 12345;
//! let db = manager.connection_for(&user_id.to_string()).await?;
//! # Ok(())
//! # }
//! ```

pub mod manager;
pub mod strategies;

pub use manager::{ShardManager, ShardStrategy};
pub use strategies::{HashStrategy, RangeStrategy, TenantStrategy};
