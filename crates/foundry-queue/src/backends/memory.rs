use async_trait::async_trait;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use rustc_hash::FxHashMap;

use crate::backends::QueueBackend;
use crate::error::QueueResult;
use crate::job::{Job, JobStatus};

/// In-memory queue backend (for development and testing)
#[derive(Clone, Default)]
pub struct MemoryBackend {
    queues: Arc<Mutex<FxHashMap<String, VecDeque<Job>>>>,
    jobs: Arc<Mutex<FxHashMap<String, Job>>>,
    delayed: Arc<Mutex<Vec<Job>>>,
    failed: Arc<Mutex<Vec<Job>>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }

    fn insert_job(&self, job: &Job) {
        self.jobs.lock().unwrap().insert(job.id.clone(), job.clone());
    }
}

#[async_trait]
impl QueueBackend for MemoryBackend {
    async fn push(&self, job: Job) -> QueueResult<()> {
        // Store job by ID
        self.insert_job(&job);

        // Handle delayed jobs
        if job.status == JobStatus::Delayed {
            self.delayed.lock().unwrap().push(job);
            return Ok(());
        }

        // Add to queue
        let queue_name = job.queue.clone();
        self.queues
            .lock()
            .unwrap()
            .entry(queue_name)
            .or_insert_with(VecDeque::new)
            .push_back(job);

        Ok(())
    }

    async fn pop(&self) -> QueueResult<Option<Job>> {
        self.pop_from("default").await
    }

    async fn pop_from(&self, queue: &str) -> QueueResult<Option<Job>> {
        let job = self
            .queues
            .lock()
            .unwrap()
            .get_mut(queue)
            .and_then(|q| q.pop_front());

        Ok(job)
    }

    async fn size(&self) -> QueueResult<usize> {
        Ok(self
            .queues
            .lock()
            .unwrap()
            .values()
            .map(|q| q.len())
            .sum())
    }

    async fn size_of(&self, queue: &str) -> QueueResult<usize> {
        Ok(self
            .queues
            .lock()
            .unwrap()
            .get(queue)
            .map(|q| q.len())
            .unwrap_or(0))
    }

    async fn clear(&self) -> QueueResult<()> {
        self.queues.lock().unwrap().clear();
        self.jobs.lock().unwrap().clear();
        self.delayed.lock().unwrap().clear();
        self.failed.lock().unwrap().clear();
        Ok(())
    }

    async fn clear_queue(&self, queue: &str) -> QueueResult<()> {
        self.queues.lock().unwrap().remove(queue);
        Ok(())
    }

    async fn get(&self, job_id: &str) -> QueueResult<Option<Job>> {
        Ok(self.jobs.lock().unwrap().get(job_id).cloned())
    }

    async fn delete(&self, job_id: &str) -> QueueResult<bool> {
        Ok(self.jobs.lock().unwrap().remove(job_id).is_some())
    }

    async fn update(&self, job: &Job) -> QueueResult<()> {
        self.insert_job(job);

        // If job failed, add to failed queue
        if job.status == JobStatus::Failed {
            self.failed.lock().unwrap().push(job.clone());
        }

        Ok(())
    }

    async fn get_failed(&self) -> QueueResult<Vec<Job>> {
        Ok(self.failed.lock().unwrap().clone())
    }

    async fn get_ready_delayed(&self) -> QueueResult<Vec<Job>> {
        let delayed = self.delayed.lock().unwrap();
        Ok(delayed
            .iter()
            .filter(|job| job.should_execute())
            .cloned()
            .collect())
    }

    async fn release_delayed(&self, job: &Job) -> QueueResult<()> {
        // Remove from delayed
        self.delayed.lock().unwrap().retain(|j| j.id != job.id);

        // Add to regular queue
        let mut updated_job = job.clone();
        updated_job.status = JobStatus::Pending;
        self.push(updated_job).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Duration;

    #[tokio::test]
    async fn test_push_pop() {
        let backend = MemoryBackend::new();
        let job = Job::new("test").with_payload(json!({"key": "value"}));

        backend.push(job.clone()).await.unwrap();
        assert_eq!(backend.size().await.unwrap(), 1);

        let popped = backend.pop().await.unwrap();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().id, job.id);
        assert_eq!(backend.size().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_multiple_queues() {
        let backend = MemoryBackend::new();
        let job1 = Job::new("test1").on_queue("high");
        let job2 = Job::new("test2").on_queue("low");

        backend.push(job1).await.unwrap();
        backend.push(job2).await.unwrap();

        assert_eq!(backend.size_of("high").await.unwrap(), 1);
        assert_eq!(backend.size_of("low").await.unwrap(), 1);
        assert_eq!(backend.size().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_delayed_jobs() {
        let backend = MemoryBackend::new();
        let job = Job::new("delayed").with_delay(Duration::from_secs(60));

        backend.push(job.clone()).await.unwrap();

        // Should not be in regular queue
        assert_eq!(backend.size().await.unwrap(), 0);

        // Should be in delayed
        let delayed = backend.get_ready_delayed().await.unwrap();
        assert_eq!(delayed.len(), 0); // Not ready yet

        // Get the job by ID
        let retrieved = backend.get(&job.id).await.unwrap();
        assert!(retrieved.is_some());
    }

    #[tokio::test]
    async fn test_failed_jobs() {
        let backend = MemoryBackend::new();
        let mut job = Job::new("test");
        job.mark_failed();

        backend.update(&job).await.unwrap();

        let failed = backend.get_failed().await.unwrap();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].status, JobStatus::Failed);
    }

    #[tokio::test]
    async fn test_clear() {
        let backend = MemoryBackend::new();
        backend.push(Job::new("test1")).await.unwrap();
        backend.push(Job::new("test2")).await.unwrap();

        assert_eq!(backend.size().await.unwrap(), 2);

        backend.clear().await.unwrap();
        assert_eq!(backend.size().await.unwrap(), 0);
    }
}
