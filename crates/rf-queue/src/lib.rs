//! # rf-queue: Background Job Processing for RustForge
//!
//! Provides a robust queue system for asynchronous job processing with multiple backends.
//!
//! ## Features
//!
//! - **Type-Safe Jobs**: Define jobs with the `Job` trait
//! - **Multiple Backends**: Memory (dev) and Redis (production)
//! - **Job Retries**: Automatic retry with configurable attempts
//! - **Delayed Jobs**: Schedule jobs for future execution
//! - **Worker Pool**: Concurrent job processing
//! - **Priority Queues**: Job prioritization support
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_queue::{Job, MemoryQueue, Worker, JobMetadata, Queue};
//! use async_trait::async_trait;
//! use serde::{Serialize, Deserialize};
//! use std::sync::Arc;
//!
//! #[derive(Serialize, Deserialize)]
//! struct SendEmailJob {
//!     to: String,
//!     subject: String,
//! }
//!
//! #[async_trait]
//! impl Job for SendEmailJob {
//!     async fn handle(&self) -> Result<(), rf_queue::QueueError> {
//!         // Send email logic
//!         println!("Sending email to {}", self.to);
//!         Ok(())
//!     }
//!
//!     fn job_type(&self) -> &'static str {
//!         "send_email"
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create queue
//! let queue = Arc::new(MemoryQueue::new());
//!
//! // Dispatch job
//! let job = SendEmailJob {
//!     to: "user@example.com".to_string(),
//!     subject: "Hello".to_string(),
//! };
//!
//! let metadata = JobMetadata::new(&job)?;
//! queue.push(metadata).await?;
//!
//! // Start worker
//! let worker = Worker::new(Arc::clone(&queue) as Arc<dyn Queue>)
//!     .concurrency(5)
//!     .handle(|job: SendEmailJob| Box::pin(async move { job.handle().await }));
//!
//! // worker.start().await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Delayed Jobs
//!
//! ```no_run
//! # use rf_queue::{Job, MemoryQueue, JobMetadata, Queue};
//! # use async_trait::async_trait;
//! # use serde::{Serialize, Deserialize};
//! # use std::time::Duration;
//! # #[derive(Serialize, Deserialize)]
//! # struct SendEmailJob { to: String }
//! # #[async_trait]
//! # impl Job for SendEmailJob {
//! #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
//! #     fn job_type(&self) -> &'static str { "send_email" }
//! # }
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let queue = MemoryQueue::new();
//! let job = SendEmailJob { to: "user@example.com".to_string() };
//!
//! // Execute after 5 minutes
//! let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(300))?;
//! queue.push(metadata).await?;
//! # Ok(())
//! # }
//! ```

mod error;
mod job;
mod memory;
mod queue;
mod worker;

pub use error::{QueueError, QueueResult};
pub use job::{Job, JobMetadata};
pub use memory::MemoryQueue;
pub use queue::Queue;
pub use worker::Worker;
