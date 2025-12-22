//! Synchronous public API for queue operations
//!
//! This module provides a synchronous public API while using async operations internally.
//! This follows Laravel's pattern where queue operations appear synchronous to the user.

use crate::error::QueueResult;
use crate::job::{Job, JobMetadata};
use crate::queue::Queue;
use rf_core::runtime::block_on;
use std::sync::Arc;
use std::time::Duration;

/// Synchronous queue API facade
pub struct QueueFacade {
    queue: Arc<dyn Queue>,
}

impl QueueFacade {
    /// Create new queue facade
    pub fn new(queue: Arc<dyn Queue>) -> Self {
        Self { queue }
    }

    /// Push a job to the queue (synchronous API)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_queue::{Job, JobMetadata, MemoryQueue, QueueFacade};
    /// use std::sync::Arc;
    /// # use async_trait::async_trait;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct SendEmailJob { to: String }
    /// # #[async_trait]
    /// # impl Job for SendEmailJob {
    /// #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
    /// #     fn job_type(&self) -> &'static str { "send_email" }
    /// # }
    ///
    /// let queue = Arc::new(MemoryQueue::new());
    /// let facade = QueueFacade::new(queue);
    ///
    /// let job = SendEmailJob { to: "user@example.com".to_string() };
    /// let metadata = JobMetadata::new(&job).unwrap();
    /// facade.push(metadata).unwrap();
    /// ```
    pub fn push(&self, metadata: JobMetadata) -> QueueResult<String> {
        let queue = Arc::clone(&self.queue);
        block_on(async move { queue.push(metadata).await })
    }

    /// Push a job with delay (synchronous API)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_queue::{Job, JobMetadata, MemoryQueue, QueueFacade};
    /// use std::sync::Arc;
    /// use std::time::Duration;
    /// # use async_trait::async_trait;
    /// # use serde::{Serialize, Deserialize};
    /// # #[derive(Serialize, Deserialize)]
    /// # struct SendEmailJob { to: String }
    /// # #[async_trait]
    /// # impl Job for SendEmailJob {
    /// #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
    /// #     fn job_type(&self) -> &'static str { "send_email" }
    /// # }
    ///
    /// let queue = Arc::new(MemoryQueue::new());
    /// let facade = QueueFacade::new(queue);
    ///
    /// let job = SendEmailJob { to: "user@example.com".to_string() };
    /// let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(300)).unwrap();
    /// facade.push_later(metadata).unwrap();
    /// ```
    pub fn push_later(&self, metadata: JobMetadata) -> QueueResult<String> {
        let queue = Arc::clone(&self.queue);
        block_on(async move { queue.push(metadata).await })
    }

    /// Reserve the next job for processing (synchronous API)
    pub fn reserve(&self, queue: &str) -> QueueResult<Option<JobMetadata>> {
        let queue_ref = Arc::clone(&self.queue);
        let queue_name = queue.to_string();
        block_on(async move { queue_ref.reserve(&queue_name).await })
    }

    /// Mark a job as completed (synchronous API)
    pub fn complete(&self, job_id: &str) -> QueueResult<()> {
        let queue = Arc::clone(&self.queue);
        let id = job_id.to_string();
        block_on(async move { queue.complete(&id).await })
    }

    /// Mark a job as failed (synchronous API)
    pub fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        let queue = Arc::clone(&self.queue);
        let id = job_id.to_string();
        let err = error.to_string();
        block_on(async move { queue.fail(&id, &err).await })
    }

    /// Retry a failed job (synchronous API)
    pub fn retry(&self, metadata: JobMetadata) -> QueueResult<()> {
        let queue = Arc::clone(&self.queue);
        block_on(async move { queue.retry(metadata).await })
    }

    /// Get queue size (synchronous API)
    pub fn size(&self, queue: &str) -> QueueResult<usize> {
        let queue_ref = Arc::clone(&self.queue);
        let queue_name = queue.to_string();
        block_on(async move { queue_ref.size(&queue_name).await })
    }

    /// Clear a queue (synchronous API)
    pub fn clear(&self, queue: &str) -> QueueResult<()> {
        let queue_ref = Arc::clone(&self.queue);
        let queue_name = queue.to_string();
        block_on(async move { queue_ref.clear(&queue_name).await })
    }
}

/// Helper function to dispatch a job (synchronous API)
///
/// # Example
///
/// ```no_run
/// use rf_queue::{Job, dispatch, MemoryQueue};
/// use std::sync::Arc;
/// # use async_trait::async_trait;
/// # use serde::{Serialize, Deserialize};
/// # #[derive(Serialize, Deserialize)]
/// # struct SendEmailJob { to: String }
/// # #[async_trait]
/// # impl Job for SendEmailJob {
/// #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
/// #     fn job_type(&self) -> &'static str { "send_email" }
/// # }
///
/// let queue = Arc::new(MemoryQueue::new());
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// dispatch(queue, job).unwrap();
/// ```
pub fn dispatch<J: Job>(queue: Arc<dyn Queue>, job: J) -> QueueResult<String> {
    let metadata = JobMetadata::new(&job)?;
    let facade = QueueFacade::new(queue);
    facade.push(metadata)
}

/// Helper function to dispatch a job with delay (synchronous API)
///
/// # Example
///
/// ```no_run
/// use rf_queue::{Job, dispatch_later, MemoryQueue};
/// use std::sync::Arc;
/// use std::time::Duration;
/// # use async_trait::async_trait;
/// # use serde::{Serialize, Deserialize};
/// # #[derive(Serialize, Deserialize)]
/// # struct SendEmailJob { to: String }
/// # #[async_trait]
/// # impl Job for SendEmailJob {
/// #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
/// #     fn job_type(&self) -> &'static str { "send_email" }
/// # }
///
/// let queue = Arc::new(MemoryQueue::new());
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// dispatch_later(queue, job, Duration::from_secs(300)).unwrap();
/// ```
pub fn dispatch_later<J: Job>(
    queue: Arc<dyn Queue>,
    job: J,
    delay: Duration,
) -> QueueResult<String> {
    let metadata = JobMetadata::new_delayed(&job, delay)?;
    let facade = QueueFacade::new(queue);
    facade.push_later(metadata)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryQueue;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestJob {
        message: String,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<(), crate::QueueError> {
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "test_job"
        }
    }

    #[test]
    fn test_sync_dispatch() {
        let queue = Arc::new(MemoryQueue::new());
        let job = TestJob {
            message: "test".to_string(),
        };

        let result = dispatch(queue, job);
        assert!(result.is_ok());
    }

    #[test]
    fn test_sync_dispatch_later() {
        let queue = Arc::new(MemoryQueue::new());
        let job = TestJob {
            message: "test".to_string(),
        };

        let result = dispatch_later(queue, job, Duration::from_secs(10));
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_push() {
        let queue = Arc::new(MemoryQueue::new());
        let facade = QueueFacade::new(queue);

        let job = TestJob {
            message: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        let result = facade.push(metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_facade_size() {
        let queue = Arc::new(MemoryQueue::new());
        let facade = QueueFacade::new(Arc::clone(&queue) as Arc<dyn Queue>);

        // Initial size should be 0
        let size = facade.size("default").unwrap();
        assert_eq!(size, 0);

        // Add a job
        let job = TestJob {
            message: "test".to_string(),
        };
        dispatch(queue, job).unwrap();

        // Size should now be 1
        let size = facade.size("default").unwrap();
        assert_eq!(size, 1);
    }
}
