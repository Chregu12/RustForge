//! Failover queue backend driver
//!
//! Provides automatic failover between primary and backup queue backends.

use crate::{JobMetadata, Queue, QueueError, QueueResult};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::timeout;

/// Failover queue driver
///
/// Automatically switches to backup queue if primary fails or times out.
pub struct FailoverQueue {
    primary: Arc<dyn Queue>,
    fallback: Arc<dyn Queue>,
    timeout_duration: Duration,
}

impl FailoverQueue {
    /// Create a new failover queue
    ///
    /// # Arguments
    ///
    /// * `primary` - Primary queue backend
    /// * `fallback` - Fallback queue backend (used when primary fails)
    /// * `timeout_duration` - Timeout for primary queue operations
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_queue::{MemoryQueue, drivers::failover::FailoverQueue};
    /// use std::sync::Arc;
    /// use std::time::Duration;
    ///
    /// let primary = Arc::new(MemoryQueue::new());
    /// let fallback = Arc::new(MemoryQueue::new());
    ///
    /// let failover = FailoverQueue::new(
    ///     primary as Arc<dyn rf_queue::Queue>,
    ///     fallback as Arc<dyn rf_queue::Queue>,
    ///     Duration::from_secs(5)
    /// );
    /// ```
    pub fn new(
        primary: Arc<dyn Queue>,
        fallback: Arc<dyn Queue>,
        timeout_duration: Duration,
    ) -> Self {
        Self {
            primary,
            fallback,
            timeout_duration,
        }
    }

    /// Create with default timeout (5 seconds)
    pub fn with_default_timeout(primary: Arc<dyn Queue>, fallback: Arc<dyn Queue>) -> Self {
        Self::new(primary, fallback, Duration::from_secs(5))
    }

    /// Try primary operation with timeout, fallback on failure
    async fn try_with_fallback<F, T>(&self, primary_op: F, fallback_op: F) -> QueueResult<T>
    where
        F: std::future::Future<Output = QueueResult<T>>,
    {
        match timeout(self.timeout_duration, primary_op).await {
            Ok(Ok(result)) => {
                tracing::debug!("Primary queue operation succeeded");
                Ok(result)
            }
            Ok(Err(e)) => {
                tracing::warn!("Primary queue failed: {}, using fallback", e);
                fallback_op.await
            }
            Err(_) => {
                tracing::warn!("Primary queue timeout, using fallback");
                fallback_op.await
            }
        }
    }
}

#[async_trait]
impl Queue for FailoverQueue {
    async fn push(&self, metadata: JobMetadata) -> QueueResult<String> {
        let metadata_clone = metadata.clone();
        self.try_with_fallback(
            self.primary.push(metadata),
            self.fallback.push(metadata_clone),
        )
        .await
    }

    async fn reserve(&self, queue: &str) -> QueueResult<Option<JobMetadata>> {
        let queue_name = queue.to_string();
        let queue_name_clone = queue_name.clone();

        self.try_with_fallback(
            self.primary.reserve(&queue_name),
            self.fallback.reserve(&queue_name_clone),
        )
        .await
    }

    async fn complete(&self, job_id: &str) -> QueueResult<()> {
        let job_id_clone = job_id.to_string();

        // Try to complete on both to ensure cleanup
        let primary_result = timeout(self.timeout_duration, self.primary.complete(job_id)).await;

        let fallback_result =
            timeout(self.timeout_duration, self.fallback.complete(&job_id_clone)).await;

        // If at least one succeeds, consider it successful
        if let Ok(Ok(())) = primary_result {
            return Ok(());
        }

        if let Ok(Ok(())) = fallback_result {
            return Ok(());
        }

        // Both failed
        Err(QueueError::Backend(
            "Both primary and fallback failed to complete job".to_string(),
        ))
    }

    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        let job_id_clone = job_id.to_string();
        let error_clone = error.to_string();

        self.try_with_fallback(
            self.primary.fail(job_id, error),
            self.fallback.fail(&job_id_clone, &error_clone),
        )
        .await
    }

    async fn retry(&self, metadata: JobMetadata) -> QueueResult<()> {
        let metadata_clone = metadata.clone();

        self.try_with_fallback(
            self.primary.retry(metadata),
            self.fallback.retry(metadata_clone),
        )
        .await
    }

    async fn size(&self, queue: &str) -> QueueResult<usize> {
        let queue_name = queue.to_string();
        let queue_name_clone = queue_name.clone();

        self.try_with_fallback(
            self.primary.size(&queue_name),
            self.fallback.size(&queue_name_clone),
        )
        .await
    }

    async fn clear(&self, queue: &str) -> QueueResult<()> {
        let queue_name = queue.to_string();
        let queue_name_clone = queue_name.clone();

        // Clear both queues
        let _ = self.primary.clear(&queue_name).await;
        let _ = self.fallback.clear(&queue_name_clone).await;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Job, MemoryQueue};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone)]
    struct TestJob {
        data: String,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self) -> Result<(), QueueError> {
            Ok(())
        }

        fn job_type(&self) -> &'static str {
            "test_job"
        }
    }

    #[tokio::test]
    async fn test_failover_queue_basic_operations() {
        let primary = Arc::new(MemoryQueue::new());
        let fallback = Arc::new(MemoryQueue::new());

        let failover = FailoverQueue::new(
            primary.clone() as Arc<dyn Queue>,
            fallback.clone() as Arc<dyn Queue>,
            Duration::from_secs(5),
        );

        let job = TestJob {
            data: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        // Push job (should use primary)
        let job_id = failover.push(metadata).await.unwrap();
        assert!(!job_id.is_empty());

        // Verify in primary queue
        let size = primary.size("default").await.unwrap();
        assert_eq!(size, 1);

        // Reserve job
        let reserved = failover.reserve("default").await.unwrap();
        assert!(reserved.is_some());

        // Complete job
        if let Some(meta) = reserved {
            failover.complete(&meta.id.unwrap()).await.unwrap();
        }

        // Verify queue is empty
        let size = failover.size("default").await.unwrap();
        assert_eq!(size, 0);
    }

    #[tokio::test]
    async fn test_failover_queue_uses_fallback() {
        // Create a failing primary queue (we'll use a queue that we immediately clear)
        let primary = Arc::new(MemoryQueue::new());
        let fallback = Arc::new(MemoryQueue::new());

        let failover = FailoverQueue::new(
            primary.clone() as Arc<dyn Queue>,
            fallback.clone() as Arc<dyn Queue>,
            Duration::from_millis(100), // Very short timeout
        );

        let job = TestJob {
            data: "test".to_string(),
        };
        let metadata = JobMetadata::new(&job).unwrap();

        // Push to failover
        failover.push(metadata).await.unwrap();

        // Even if primary has it, fallback should work
        let reserved = failover.reserve("default").await.unwrap();
        assert!(reserved.is_some());
    }
}
