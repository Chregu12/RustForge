//! Foundry Queue - Multi-backend job queue system
//!
//! This crate provides a comprehensive job queue system for the Foundry framework.
//!
//! # Features
//!
//! - **Multiple Backends**: Redis, In-Memory, Database (future)
//! - **Type-Safe API**: Generic methods for serializable job payloads
//! - **Delayed Jobs**: Schedule jobs for future execution
//! - **Job Retry**: Automatic retry with configurable attempts
//! - **Worker Process**: Background job processing
//! - **Job Priority**: Support for job prioritization
//!
//! # Example
//!
//! ```no_run
//! use foundry_queue::prelude::*;
//! use serde_json::json;
//!
//! # async fn example() -> Result<(), QueueError> {
//! // Create queue manager from environment
//! let queue = QueueManager::from_env()?;
//!
//! // Dispatch a job
//! let job = Job::new("send_email")
//!     .with_payload(json!({"to": "user@example.com", "subject": "Hello"}))
//!     .with_delay(std::time::Duration::from_secs(60));
//!
//! queue.dispatch(job).await?;
//!
//! // Start a worker to process jobs
//! let worker = Worker::new(queue.clone());
//! worker.run().await?;
//!
//! # Ok(())
//! # }
//! ```

pub mod backends;
pub mod worker;
pub mod job;
pub mod manager;
pub mod error;

pub use backends::{MemoryBackend, RedisBackend};
pub use worker::Worker;
pub use job::{Job, JobStatus, JobResult};
pub use manager::{QueueManager, QueueConfig};
pub use error::{QueueError, QueueResult};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::backends::{MemoryBackend, RedisBackend};
    pub use crate::worker::Worker;
    pub use crate::job::{Job, JobStatus, JobResult};
    pub use crate::manager::{QueueManager, QueueConfig};
    pub use crate::error::{QueueError, QueueResult};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::QueueBackend;
    use serde_json::json;

    #[tokio::test]
    async fn test_basic_queue() {
        let backend = MemoryBackend::new();
        let job = Job::new("test_job").with_payload(json!({"data": "test"}));

        backend.push(job.clone()).await.unwrap();
        let retrieved = backend.pop().await.unwrap();

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "test_job");
    }
}
