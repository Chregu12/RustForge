//! Synchronous public API for job dispatching
//!
//! This module provides a synchronous public API while using async operations internally.
//! This follows Laravel's pattern where job dispatch operations appear synchronous to the user.

use crate::error::QueueError;
use crate::job::Job;
use crate::queue::{QueueManager, QueuePriority};
use rf_core::runtime::block_on;
use std::time::Duration;
use uuid::Uuid;

/// Dispatch a job to its default queue (synchronous API)
///
/// # Example
///
/// ```ignore
/// use rf_jobs::{Job, dispatch};
/// # use async_trait::async_trait;
/// # use serde::{Serialize, Deserialize};
/// # #[derive(Serialize, Deserialize)]
/// # struct SendEmailJob { to: String }
/// # #[async_trait]
/// # impl Job for SendEmailJob {
/// #     async fn handle(&self, _ctx: rf_jobs::JobContext) -> rf_jobs::JobResult { Ok(()) }
/// # }
///
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// let job_id = dispatch(job).expect("Failed to dispatch job");
/// println!("Dispatched job: {}", job_id);
/// ```
pub fn dispatch<J: Job>(queue_manager: &QueueManager, job: J) -> Result<Uuid, QueueError> {
    block_on(async { queue_manager.dispatch(job).await })
}

/// Dispatch a job to a specific queue (synchronous API)
///
/// # Example
///
/// ```ignore
/// use rf_jobs::{Job, dispatch_to};
///
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// let job_id = dispatch_to(job, "emails").expect("Failed to dispatch job");
/// ```
pub fn dispatch_to<J: Job>(
    queue_manager: &QueueManager,
    job: J,
    queue: &str,
) -> Result<Uuid, QueueError> {
    block_on(async { queue_manager.dispatch_to(job, queue).await })
}

/// Dispatch a job with a delay (synchronous API)
///
/// # Example
///
/// ```ignore
/// use rf_jobs::{Job, dispatch_later};
/// use std::time::Duration;
///
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// let job_id = dispatch_later(job, Duration::from_secs(300))
///     .expect("Failed to dispatch delayed job");
/// ```
pub fn dispatch_later<J: Job>(
    queue_manager: &QueueManager,
    job: J,
    delay: Duration,
) -> Result<Uuid, QueueError> {
    block_on(async { queue_manager.dispatch_later(job, delay).await })
}

/// Dispatch a job with priority (synchronous API)
///
/// # Example
///
/// ```ignore
/// use rf_jobs::{Job, dispatch_with_priority, QueuePriority};
///
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// let job_id = dispatch_with_priority(job, QueuePriority::High)
///     .expect("Failed to dispatch job");
/// ```
pub fn dispatch_with_priority<J: Job>(
    queue_manager: &QueueManager,
    job: J,
    priority: QueuePriority,
) -> Result<Uuid, QueueError> {
    block_on(async { queue_manager.dispatch_with_priority(job, priority).await })
}

/// Dispatch a job to a specific queue with priority (synchronous API)
///
/// # Example
///
/// ```ignore
/// use rf_jobs::{Job, dispatch_on, QueuePriority};
///
/// let job = SendEmailJob { to: "user@example.com".to_string() };
/// let job_id = dispatch_on(job, "emails", QueuePriority::High)
///     .expect("Failed to dispatch job");
/// ```
pub fn dispatch_on<J: Job>(
    queue_manager: &QueueManager,
    job: J,
    queue: &str,
    priority: QueuePriority,
) -> Result<Uuid, QueueError> {
    block_on(async { queue_manager.dispatch_on(job, queue, priority).await })
}

/// Get queue size (synchronous API)
pub fn queue_size(queue_manager: &QueueManager, queue: &str) -> Result<u64, QueueError> {
    block_on(async { queue_manager.size(queue).await })
}

/// Clear a queue (synchronous API)
pub fn clear_queue(queue_manager: &QueueManager, queue: &str) -> Result<(), QueueError> {
    block_on(async { queue_manager.clear(queue).await })
}

/// Retry a failed job by ID (synchronous API)
pub fn retry_failed(queue_manager: &QueueManager, job_id: Uuid) -> Result<(), QueueError> {
    block_on(async { queue_manager.retry_failed(job_id).await })
}

/// Clear all failed jobs (synchronous API)
pub fn clear_failed_jobs(queue_manager: &QueueManager) -> Result<(), QueueError> {
    block_on(async { queue_manager.clear_failed().await })
}

/// Synchronous QueueManager wrapper
///
/// Provides a synchronous API for queue operations while maintaining
/// async operations internally.
pub struct SyncQueueManager {
    inner: QueueManager,
}

impl SyncQueueManager {
    /// Create new synchronous queue manager
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_jobs::SyncQueueManager;
    ///
    /// let manager = SyncQueueManager::new("redis://localhost:6379")
    ///     .expect("Failed to create queue manager");
    /// ```
    pub fn new(redis_url: &str) -> Result<Self, QueueError> {
        let inner = block_on(async { QueueManager::new(redis_url).await })?;
        Ok(Self { inner })
    }

    /// Get reference to inner async queue manager
    pub fn inner(&self) -> &QueueManager {
        &self.inner
    }

    /// Dispatch a job (synchronous API)
    pub fn dispatch<J: Job>(&self, job: J) -> Result<Uuid, QueueError> {
        dispatch(&self.inner, job)
    }

    /// Dispatch a job to a specific queue (synchronous API)
    pub fn dispatch_to<J: Job>(&self, job: J, queue: &str) -> Result<Uuid, QueueError> {
        dispatch_to(&self.inner, job, queue)
    }

    /// Dispatch a job with delay (synchronous API)
    pub fn dispatch_later<J: Job>(&self, job: J, delay: Duration) -> Result<Uuid, QueueError> {
        dispatch_later(&self.inner, job, delay)
    }

    /// Dispatch a job with priority (synchronous API)
    pub fn dispatch_with_priority<J: Job>(
        &self,
        job: J,
        priority: QueuePriority,
    ) -> Result<Uuid, QueueError> {
        dispatch_with_priority(&self.inner, job, priority)
    }

    /// Get queue size (synchronous API)
    pub fn size(&self, queue: &str) -> Result<u64, QueueError> {
        queue_size(&self.inner, queue)
    }

    /// Clear a queue (synchronous API)
    pub fn clear(&self, queue: &str) -> Result<(), QueueError> {
        clear_queue(&self.inner, queue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Job, JobContext, JobResult};
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        value: i32,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self, _ctx: JobContext) -> JobResult {
            Ok(())
        }
    }

    /// Check if Redis is available for testing
    async fn redis_available() -> bool {
        match redis::Client::open("redis://localhost:6379") {
            Ok(client) => match client.get_multiplexed_async_connection().await {
                Ok(_) => true,
                Err(_) => false,
            },
            Err(_) => false,
        }
    }

    #[test]
    fn test_sync_queue_manager_new() {
        // This will fail if Redis is not available, but demonstrates the sync API
        let result = SyncQueueManager::new("redis://localhost:6379");

        // We expect this to either succeed or fail with a connection error
        // The important part is that the API is synchronous
        match result {
            Ok(_manager) => {
                // Success - Redis was available
            }
            Err(QueueError::ConnectionError(_)) => {
                // Expected if Redis is not running
            }
            Err(e) => {
                panic!("Unexpected error: {:?}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_sync_dispatch() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_sync_dispatch: Redis not available");
            return;
        }

        let manager = SyncQueueManager::new("redis://localhost:6379").unwrap();
        let job = TestJob { value: 42 };

        let result = manager.dispatch(job);
        assert!(result.is_ok());
    }
}
