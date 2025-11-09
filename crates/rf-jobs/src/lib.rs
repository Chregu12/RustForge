//! # rf-jobs - Background Jobs & Queue System
//!
//! Production-ready background job processing with:
//! - Asynchronous job queue with Redis backend
//! - Worker pool with configurable concurrency
//! - Job scheduling (cron-like patterns)
//! - Retry logic with exponential backoff
//! - Failed job handling (Dead Letter Queue)
//!
//! ## Quick Start
//!
//! ```ignore
//! use rf_jobs::{Job, JobContext, JobResult, QueueManager};
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
//! // 2. Dispatch job
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let manager = QueueManager::new("redis://localhost:6379").await?;
//! let job = SendEmailJob {
//!     to: "user@example.com".to_string(),
//!     subject: "Welcome!".to_string(),
//! };
//! manager.dispatch(job).await?;
//! # Ok(())
//! # }
//! ```

pub mod context;
pub mod error;
pub mod job;
pub mod queue;
pub mod scheduler;
pub mod worker;

// Re-export main types
pub use context::JobContext;
pub use error::{JobError, JobResult, QueueError, SchedulerError, WorkerError};
pub use job::{FailedJob, Job, JobPayload};
pub use queue::QueueManager;
pub use scheduler::Scheduler;
pub use worker::{Worker, WorkerConfig, WorkerPool};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        context::JobContext,
        error::{JobError, JobResult, QueueError},
        job::{Job, JobPayload},
        queue::QueueManager,
        worker::{WorkerConfig, WorkerPool},
    };
}
