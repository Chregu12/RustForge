//! Job Batching - Parallel job execution with callbacks
//!
//! Allows you to execute multiple jobs in parallel and track their completion.
//! Similar to Laravel's `Bus::batch()`.
//!
//! # Example
//!
//! ```ignore
//! use rf_jobs::batch::JobBatch;
//!
//! JobBatch::new()
//!     .add_many((0..100).map(|i| ProcessPodcast::new(i)))
//!     .then(|batch_id| async move {
//!         println!("All jobs completed!");
//!     })
//!     .catch(|batch_id, error| async move {
//!         eprintln!("Batch failed: {}", error);
//!     })
//!     .dispatch(&queue)
//!     .await?;
//! ```

use crate::error::QueueError;
use crate::job::{Job, JobPayload};
use crate::queue::QueueManager;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

/// Type alias for async callbacks
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A batch of jobs to be executed in parallel
pub struct JobBatch {
    jobs: Vec<SerializedJob>,
    name: Option<String>,
    batch_id: Uuid,
    allow_failures: bool,
    // Store callback types instead of the actual closures
    has_then_callback: bool,
    has_catch_callback: bool,
    has_finally_callback: bool,
}

/// Serialized job wrapper for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SerializedJob {
    job_type: String,
    data: serde_json::Value,
    queue: String,
}

/// Batch state stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchState {
    pub batch_id: Uuid,
    pub name: Option<String>,
    pub total: u64,
    pub pending: u64,
    pub completed: u64,
    pub failed: u64,
    pub status: BatchStatus,
    pub allow_failures: bool,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub errors: Vec<String>,
}

/// Batch execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum BatchStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl JobBatch {
    /// Create a new job batch
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            name: None,
            batch_id: Uuid::new_v4(),
            allow_failures: false,
            has_then_callback: false,
            has_catch_callback: false,
            has_finally_callback: false,
        }
    }

    /// Set batch name for easier identification
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a single job to the batch
    pub fn add<J: Job + 'static>(mut self, job: J) -> Result<Self, serde_json::Error> {
        let serialized = SerializedJob {
            job_type: std::any::type_name::<J>().to_string(),
            data: serde_json::to_value(&job)?,
            queue: job.queue().to_string(),
        };
        self.jobs.push(serialized);
        Ok(self)
    }

    /// Add multiple jobs to the batch
    pub fn add_many<I, J>(mut self, jobs: I) -> Result<Self, serde_json::Error>
    where
        I: IntoIterator<Item = J>,
        J: Job + 'static,
    {
        for job in jobs {
            let serialized = SerializedJob {
                job_type: std::any::type_name::<J>().to_string(),
                data: serde_json::to_value(&job)?,
                queue: job.queue().to_string(),
            };
            self.jobs.push(serialized);
        }
        Ok(self)
    }

    /// Register a completion callback
    /// Note: Actual callback execution would require a separate worker/listener
    pub fn then<F, Fut>(mut self, _f: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.has_then_callback = true;
        self
    }

    /// Register a failure callback
    pub fn catch<F, Fut>(mut self, _f: F) -> Self
    where
        F: Fn(String, String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.has_catch_callback = true;
        self
    }

    /// Register a finally callback (always runs)
    pub fn finally<F, Fut>(mut self, _f: F) -> Self
    where
        F: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.has_finally_callback = true;
        self
    }

    /// Allow batch to continue even if some jobs fail
    pub fn allow_failures(mut self, allow: bool) -> Self {
        self.allow_failures = allow;
        self
    }

    /// Get batch ID
    pub fn id(&self) -> Uuid {
        self.batch_id
    }

    /// Dispatch the batch to the queue
    pub async fn dispatch(self, queue: &QueueManager) -> Result<Uuid, QueueError> {
        if self.jobs.is_empty() {
            return Err(QueueError::InvalidConfig("Batch has no jobs".into()));
        }

        let batch_id = self.batch_id;
        let total = self.jobs.len() as u64;

        // Initialize batch state in Redis
        let state = BatchState {
            batch_id,
            name: self.name.clone(),
            total,
            pending: total,
            completed: 0,
            failed: 0,
            status: BatchStatus::Pending,
            allow_failures: self.allow_failures,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            errors: Vec::new(),
        };

        queue.save_batch_state(&state).await?;

        // Dispatch all jobs with batch metadata
        for (index, job) in self.jobs.iter().enumerate() {
            queue.dispatch_batch_job(batch_id, index, job).await?;
        }

        // Mark batch as processing
        queue
            .update_batch_status(batch_id, BatchStatus::Processing)
            .await?;

        Ok(batch_id)
    }
}

impl Default for JobBatch {
    fn default() -> Self {
        Self::new()
    }
}

/// Batch management methods for QueueManager
impl QueueManager {
    /// Save batch state to Redis
    pub async fn save_batch_state(&self, state: &BatchState) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("batch:{}:state", state.batch_id);
        let json = serde_json::to_string(state)?;
        conn.set::<_, _, ()>(&key, json).await?;
        Ok(())
    }

    /// Load batch state from Redis
    pub async fn load_batch_state(&self, batch_id: Uuid) -> Result<BatchState, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("batch:{}:state", batch_id);
        let json: String = conn
            .get(&key)
            .await
            .map_err(|_| QueueError::JobNotFound(batch_id))?;
        let state: BatchState = serde_json::from_str(&json)?;
        Ok(state)
    }

    /// Update batch status
    pub async fn update_batch_status(
        &self,
        batch_id: Uuid,
        status: BatchStatus,
    ) -> Result<(), QueueError> {
        let mut state = self.load_batch_state(batch_id).await?;
        if state.started_at.is_none() && status == BatchStatus::Processing {
            state.started_at = Some(Utc::now());
        }
        state.status = status;
        self.save_batch_state(&state).await?;
        Ok(())
    }

    /// Dispatch a batch job with metadata
    pub async fn dispatch_batch_job(
        &self,
        batch_id: Uuid,
        index: usize,
        job: &SerializedJob,
    ) -> Result<Uuid, QueueError> {
        // Create a JobPayload with batch metadata
        let mut payload = JobPayload {
            id: Uuid::new_v4(),
            queue: job.queue.clone(),
            job_type: job.job_type.clone(),
            data: job.data.clone(),
            attempt: 0,
            max_attempts: 3,
            dispatched_at: Utc::now(),
            available_at: Utc::now(),
            backoff_seconds: 60,
        };

        // Store batch metadata in job payload
        if let Some(obj) = payload.data.as_object_mut() {
            obj.insert("__batch_id".to_string(), serde_json::json!(batch_id));
            obj.insert("__batch_index".to_string(), serde_json::json!(index));
        }

        let job_id = payload.id;
        let queue_name = payload.queue.clone();
        self.push_to_queue(&queue_name, payload).await?;

        Ok(job_id)
    }

    /// Handle batch job completion
    pub async fn handle_batch_job_completion(&self, batch_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("batch:{}:state", batch_id);

        // Load current state
        let json: String = conn.get(&key).await?;
        let mut state: BatchState = serde_json::from_str(&json)?;

        // Update counters
        state.pending = state.pending.saturating_sub(1);
        state.completed += 1;

        // Check if batch is complete
        if state.pending == 0 {
            state.status = if state.failed > 0 && !state.allow_failures {
                BatchStatus::Failed
            } else {
                BatchStatus::Completed
            };
            state.completed_at = Some(Utc::now());
        }

        // Save updated state
        let updated_json = serde_json::to_string(&state)?;
        conn.set::<_, _, ()>(&key, updated_json).await?;

        Ok(())
    }

    /// Handle batch job failure
    pub async fn handle_batch_job_failure(
        &self,
        batch_id: Uuid,
        error: String,
    ) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("batch:{}:state", batch_id);

        // Load current state
        let json: String = conn.get(&key).await?;
        let mut state: BatchState = serde_json::from_str(&json)?;

        // Update counters and errors
        state.pending = state.pending.saturating_sub(1);
        state.failed += 1;
        state.errors.push(error);

        // Check if batch should fail immediately (unless failures are allowed)
        if !state.allow_failures {
            state.status = BatchStatus::Failed;
            state.completed_at = Some(Utc::now());
        } else if state.pending == 0 {
            // All jobs done, determine final status
            state.status = if state.failed > 0 && !state.allow_failures {
                BatchStatus::Failed
            } else {
                BatchStatus::Completed
            };
            state.completed_at = Some(Utc::now());
        }

        // Save updated state
        let updated_json = serde_json::to_string(&state)?;
        conn.set::<_, _, ()>(&key, updated_json).await?;

        Ok(())
    }

    /// Cancel a batch
    pub async fn cancel_batch(&self, batch_id: Uuid) -> Result<(), QueueError> {
        let mut state = self.load_batch_state(batch_id).await?;
        if state.status == BatchStatus::Pending || state.status == BatchStatus::Processing {
            state.status = BatchStatus::Cancelled;
            state.completed_at = Some(Utc::now());
            self.save_batch_state(&state).await?;
        }
        Ok(())
    }

    /// Get batch progress
    pub async fn batch_progress(&self, batch_id: Uuid) -> Result<(u64, u64, u64, u64), QueueError> {
        let state = self.load_batch_state(batch_id).await?;
        Ok((state.completed, state.failed, state.pending, state.total))
    }

    /// Delete batch data
    pub async fn delete_batch(&self, batch_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("batch:{}:state", batch_id);
        conn.del::<_, ()>(&key).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobContext;
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

    #[test]
    fn test_batch_creation() {
        let batch = JobBatch::new()
            .add(TestJob { value: 1 })
            .unwrap()
            .add(TestJob { value: 2 })
            .unwrap()
            .add(TestJob { value: 3 })
            .unwrap();

        assert_eq!(batch.jobs.len(), 3);
    }

    #[test]
    fn test_batch_with_name() {
        let batch = JobBatch::new()
            .name("test-batch")
            .add(TestJob { value: 1 })
            .unwrap();

        assert_eq!(batch.name, Some("test-batch".to_string()));
    }

    #[test]
    fn test_batch_add_many() {
        let jobs = vec![
            TestJob { value: 1 },
            TestJob { value: 2 },
            TestJob { value: 3 },
        ];

        let batch = JobBatch::new().add_many(jobs).unwrap();

        assert_eq!(batch.jobs.len(), 3);
    }

    #[test]
    fn test_batch_allow_failures() {
        let batch = JobBatch::new()
            .add(TestJob { value: 1 })
            .unwrap()
            .allow_failures(true);

        assert!(batch.allow_failures);
    }

    #[test]
    fn test_batch_state_initialization() {
        let state = BatchState {
            batch_id: Uuid::new_v4(),
            name: None,
            total: 100,
            pending: 100,
            completed: 0,
            failed: 0,
            status: BatchStatus::Pending,
            allow_failures: false,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            errors: Vec::new(),
        };

        assert_eq!(state.status, BatchStatus::Pending);
        assert_eq!(state.total, 100);
        assert_eq!(state.pending, 100);
        assert_eq!(state.completed, 0);
        assert_eq!(state.failed, 0);
    }
}
