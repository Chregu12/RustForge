//! In-memory queue backend for development

use crate::error::{QueueError, QueueResult};
use crate::job::JobMetadata;
use crate::queue::Queue;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::Mutex;

/// In-memory queue backend
#[derive(Clone)]
pub struct MemoryQueue {
    queues: Arc<Mutex<HashMap<String, VecDeque<JobMetadata>>>>,
    failed: Arc<Mutex<HashMap<String, JobMetadata>>>,
}

impl MemoryQueue {
    /// Create new memory queue
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
            failed: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for MemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Queue for MemoryQueue {
    async fn push(&self, metadata: JobMetadata) -> QueueResult<String> {
        let job_id = metadata.id.clone();
        let queue_name = metadata.queue.clone();

        let mut queues = self.queues.lock().await;
        queues
            .entry(queue_name)
            .or_insert_with(VecDeque::new)
            .push_back(metadata);

        tracing::debug!(job_id = %job_id, "Job pushed to memory queue");
        Ok(job_id)
    }

    async fn reserve(&self, queue: &str) -> QueueResult<Option<JobMetadata>> {
        let mut queues = self.queues.lock().await;

        if let Some(queue_jobs) = queues.get_mut(queue) {
            // Find first job that should execute
            if let Some(pos) = queue_jobs.iter().position(|j| j.should_execute()) {
                let mut metadata = queue_jobs.remove(pos).unwrap();
                metadata.mark_attempt();
                return Ok(Some(metadata));
            }
        }

        Ok(None)
    }

    async fn complete(&self, job_id: &str) -> QueueResult<()> {
        tracing::debug!(job_id = %job_id, "Job completed");
        Ok(())
    }

    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        // Store in failed jobs
        let mut failed = self.failed.lock().await;

        // We don't have the full metadata here, so just log
        tracing::warn!(job_id = %job_id, error = %error, "Job failed");

        Ok(())
    }

    async fn retry(&self, metadata: JobMetadata) -> QueueResult<()> {
        if !metadata.can_retry() {
            return Err(QueueError::JobFailed(
                "Max retries exceeded".to_string(),
            ));
        }

        self.push(metadata).await?;
        Ok(())
    }

    async fn size(&self, queue: &str) -> QueueResult<usize> {
        let queues = self.queues.lock().await;
        Ok(queues.get(queue).map(|q| q.len()).unwrap_or(0))
    }

    async fn clear(&self, queue: &str) -> QueueResult<()> {
        let mut queues = self.queues.lock().await;
        queues.remove(queue);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::job::Job;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestJob {
        message: String,
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
    async fn test_push_and_reserve() {
        let queue = MemoryQueue::new();
        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();

        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());
        assert_eq!(reserved.unwrap().job_type, "test_job");
    }

    #[tokio::test]
    async fn test_queue_size() {
        let queue = MemoryQueue::new();
        let job = TestJob {
            message: "test".to_string(),
        };

        assert_eq!(queue.size("default").await.unwrap(), 0);

        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();

        assert_eq!(queue.size("default").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_clear_queue() {
        let queue = MemoryQueue::new();
        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();

        queue.clear("default").await.unwrap();
        assert_eq!(queue.size("default").await.unwrap(), 0);
    }
}
