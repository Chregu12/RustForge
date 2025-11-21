//! # rf-jobs Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-jobs.
//!
//! ## Usage
//!
//! ```rust
//! use rf_jobs::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: batch::{BatchState, BatchStatus, JobBatch};
pub use crate:: chain::{ChainState, ChainStatus, JobChain};
pub use crate:: context::JobContext;
pub use crate:: error::{JobError, JobResult, QueueError, SchedulerError, WorkerError};
pub use crate:: job::{FailedJob, Job, JobPayload};
pub use crate:: queue::{QueueManager, QueuePriority};
pub use crate:: rate_limit::{RateLimiter, RateLimitExt};
pub use crate:: registry::{BackoffStrategy, JobRegistry, JobWithRegistry};
pub use crate:: scheduler::Scheduler;
pub use crate:: serialization::{serialize_job, serialize_job_delayed, SerializedJob};
