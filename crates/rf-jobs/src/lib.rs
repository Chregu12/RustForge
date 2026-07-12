//! # rf-jobs - Background Jobs & Queue System
//!
//! Production-ready background job processing with:
//! - Asynchronous job queue with Redis backend
//! - Worker pool with configurable concurrency
//! - Job scheduling (cron-like patterns)
//! - Retry logic with exponential backoff
//! - Failed job handling (Dead Letter Queue)
//!
//! ## Backend note: Redis vs in-process
//!
//! **rf-jobs** (`QueueManager`, `WorkerPool`) unconditionally requires a live Redis
//! connection.  If you need an **in-process, no-external-service** backend — for
//! integration tests, offline development, or small workloads — use the
//! **`rf-queue`** crate instead.  It ships a `MemoryQueue`, a `Worker` you can
//! drain in-process, and a process-global `Jobs` facade, all without Redis:
//!
//! ```ignore
//! // (from the `rf-queue` crate — see examples/jobs-offline)
//! use rf_queue::{Job, MemoryQueue, Worker, Jobs};
//! use std::sync::Arc;
//!
//! // Works in tests and anywhere without Redis
//! let queue = Arc::new(MemoryQueue::new());
//! // dispatch jobs, then: Worker::new(queue).work_once().await
//! ```
//!
//! See `examples/jobs-offline` for a runnable end-to-end example of the
//! `rf-queue` in-process path.
//!
//! ## Quick Start
//!
//! ```ignore
//! use rf_jobs::{Job, JobContext, JobResult, SyncQueueManager, dispatch};
//! use serde::{Deserialize, Serialize};
//! use async_trait::async_trait;
//!
//! // 1. Define a job
//! #[derive(Debug, Clone, Serialize, Deserialize)]
//! struct SendEmailJob {
//!     to: String,
//!     subject: String,
//! }
//!
//! #[async_trait]
//! impl Job for SendEmailJob {
//!     async fn handle(&self, ctx: JobContext) -> JobResult {
//!         ctx.log(&format!("Sending email to {}", self.to));
//!         // Send email logic
//!         Ok(())
//!     }
//! }
//!
//! // 2. Dispatch job using synchronous API
//! let manager = SyncQueueManager::new("redis://localhost:6379")
//!     .expect("Failed to create queue manager");
//!
//! let job = SendEmailJob {
//!     to: "user@example.com".to_string(),
//!     subject: "Welcome!".to_string(),
//! };
//!
//! // Dispatch synchronously
//! manager.dispatch(job).expect("Failed to dispatch job");
//! ```

pub mod api;
pub mod batch;
pub mod chain;
pub mod context;
pub mod error;
pub mod job;
pub mod queue;
pub mod rate_limit;
pub mod registry;
pub mod routing;
pub mod scheduler;
pub mod serialization;
pub mod worker;

// Re-export main types
pub use api::{
    clear_failed_jobs, clear_queue, dispatch, dispatch_later, dispatch_on, dispatch_to,
    dispatch_with_priority, queue_size, retry_failed, SyncQueueManager,
};
pub use batch::{BatchState, BatchStatus, JobBatch};
pub use chain::{ChainState, ChainStatus, JobChain};
pub use context::JobContext;
pub use error::{JobError, JobResult, QueueError, SchedulerError, WorkerError};
pub use job::{FailedJob, Job, JobPayload};
pub use queue::{QueueManager, QueuePriority};
pub use rate_limit::{RateLimitExt, RateLimiter};
pub use registry::{BackoffStrategy, JobRegistry, JobWithRegistry};
pub use routing::{JobRouter, QueueRoute};
pub use scheduler::Scheduler;
pub use serialization::{serialize_job, serialize_job_delayed, SerializedJob};
pub use worker::{Worker, WorkerConfig, WorkerPool};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        api::{dispatch, dispatch_later, SyncQueueManager},
        batch::{BatchState, BatchStatus, JobBatch},
        chain::{ChainState, ChainStatus, JobChain},
        context::JobContext,
        error::{JobError, JobResult, QueueError},
        job::{Job, JobPayload},
        queue::{QueueManager, QueuePriority},
        rate_limit::{RateLimitExt, RateLimiter},
        registry::{BackoffStrategy, JobRegistry, JobWithRegistry},
        serialization::{serialize_job, serialize_job_delayed, SerializedJob},
        worker::{WorkerConfig, WorkerPool},
    };
}
