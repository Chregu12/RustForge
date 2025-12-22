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
//! use rf_queue::{Job, MemoryQueue, dispatch, QueueFacade};
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
//! // Create queue
//! let queue = Arc::new(MemoryQueue::new());
//!
//! // Dispatch job using synchronous API
//! let job = SendEmailJob {
//!     to: "user@example.com".to_string(),
//!     subject: "Hello".to_string(),
//! };
//!
//! // Simple dispatch
//! dispatch(Arc::clone(&queue), job).expect("Failed to dispatch job");
//!
//! // Or use the facade for more control
//! let facade = QueueFacade::new(queue);
//! println!("Queue size: {}", facade.size("default").unwrap());
//! ```
//!
//! ## Delayed Jobs
//!
//! ```no_run
//! # use rf_queue::{Job, MemoryQueue, dispatch_later};
//! # use async_trait::async_trait;
//! # use serde::{Serialize, Deserialize};
//! # use std::sync::Arc;
//! # use std::time::Duration;
//! # #[derive(Serialize, Deserialize)]
//! # struct SendEmailJob { to: String }
//! # #[async_trait]
//! # impl Job for SendEmailJob {
//! #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
//! #     fn job_type(&self) -> &'static str { "send_email" }
//! # }
//! let queue = Arc::new(MemoryQueue::new());
//! let job = SendEmailJob { to: "user@example.com".to_string() };
//!
//! // Execute after 5 minutes using synchronous API
//! dispatch_later(queue, job, Duration::from_secs(300))
//!     .expect("Failed to dispatch delayed job");
//! ```

mod api;
mod config;
mod error;
mod job;
mod memory;
mod queue;
mod worker;

#[cfg(feature = "redis-backend")]
mod redis;

pub mod drivers;

pub use api::{dispatch, dispatch_later, QueueFacade};
pub use config::{QueueConfig, QueueConfigBuilder};
pub use error::{QueueError, QueueResult};
pub use job::{Job, JobMetadata};
pub use memory::MemoryQueue;
pub use queue::Queue;
pub use worker::Worker;

#[cfg(feature = "redis-backend")]
pub use redis::RedisQueue;

// Re-export drivers
#[cfg(feature = "database")]
pub use drivers::database::DatabaseQueue;

#[cfg(feature = "sqs")]
pub use drivers::sqs::SqsQueue;

pub use drivers::failover::FailoverQueue;
