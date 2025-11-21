//! # rf-queue Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-queue.
//!
//! ## Usage
//!
//! ```rust
//! use rf_queue::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: config::{QueueConfig, QueueConfigBuilder};
pub use crate:: error::{QueueError, QueueResult};
pub use crate:: job::{Job, JobMetadata};
pub use crate:: memory::MemoryQueue;
pub use crate:: queue::Queue;
pub use crate:: worker::Worker;
pub use crate:: redis::RedisQueue;
