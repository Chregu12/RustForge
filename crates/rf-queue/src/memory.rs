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
    /// Jobs that have been reserved but not yet completed/failed/retried.
    ///
    /// `reserve` removes a job from its deque, so its full metadata would be
    /// lost on failure. Tracking it here (keyed by job id) lets `fail` persist
    /// the *real* metadata into the dead-letter map, since the [`Queue::fail`]
    /// signature only carries the job id and error string.
    in_flight: Arc<Mutex<HashMap<String, JobMetadata>>>,
    failed: Arc<Mutex<HashMap<String, JobMetadata>>>,
}

impl MemoryQueue {
    /// Create new memory queue
    pub fn new() -> Self {
        Self {
            queues: Arc::new(Mutex::new(HashMap::new())),
            in_flight: Arc::new(Mutex::new(HashMap::new())),
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
        let reserved = {
            let mut queues = self.queues.lock().await;

            if let Some(queue_jobs) = queues.get_mut(queue) {
                // Among the ready jobs, pick the highest priority. Using a strict
                // `>` when comparing keeps FIFO order among equal priorities (we
                // only ever replace the candidate for a *strictly* higher one).
                let mut best: Option<(usize, i32)> = None;
                for (i, job) in queue_jobs.iter().enumerate() {
                    if job.should_execute() {
                        let take = match best {
                            None => true,
                            Some((_, best_priority)) => job.priority > best_priority,
                        };
                        if take {
                            best = Some((i, job.priority));
                        }
                    }
                }

                if let Some((pos, _)) = best {
                    let mut metadata = queue_jobs.remove(pos).unwrap();
                    metadata.mark_attempt();
                    Some(metadata)
                } else {
                    None
                }
            } else {
                None
            }
        };

        // Track the reserved job so a later `fail` can persist its real metadata.
        if let Some(metadata) = &reserved {
            self.in_flight
                .lock()
                .await
                .insert(metadata.id.clone(), metadata.clone());
        }

        Ok(reserved)
    }

    async fn complete(&self, job_id: &str) -> QueueResult<()> {
        self.in_flight.lock().await.remove(job_id);
        tracing::debug!(job_id = %job_id, "Job completed");
        Ok(())
    }

    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        // Move the job out of the in-flight set into the dead-letter map, keeping
        // its full metadata so it is observable via `failed()`. If we never saw
        // the job reserved (e.g. a direct `fail` call), persist a minimal record
        // rather than silently dropping it.
        let mut metadata = self
            .in_flight
            .lock()
            .await
            .remove(job_id)
            .unwrap_or_else(|| JobMetadata {
                id: job_id.to_string(),
                job_type: String::new(),
                handler_key: String::new(),
                data: Vec::new(),
                queue: String::new(),
                attempts: 0,
                max_retries: 0,
                priority: 0,
                timeout_secs: 0,
                created_at: chrono::Utc::now(),
                execute_at: None,
                last_error: None,
            });
        metadata.mark_error(error.to_string());

        self.failed
            .lock()
            .await
            .insert(job_id.to_string(), metadata);

        tracing::warn!(job_id = %job_id, error = %error, "Job failed permanently");
        Ok(())
    }

    async fn retry(&self, metadata: JobMetadata) -> QueueResult<()> {
        if !metadata.can_retry() {
            return Err(QueueError::JobFailed("Max retries exceeded".to_string()));
        }

        // No longer in flight: it goes back onto the deque for another attempt.
        self.in_flight.lock().await.remove(&metadata.id);
        self.push(metadata).await?;
        Ok(())
    }

    async fn failed(&self) -> QueueResult<Vec<JobMetadata>> {
        Ok(self.failed.lock().await.values().cloned().collect())
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
