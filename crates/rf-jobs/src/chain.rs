//! Job Chaining - Sequential job execution
//!
//! Allows you to chain multiple jobs together so they execute sequentially.
//! Similar to Laravel's `Bus::chain()`.
//!
//! # Example
//!
//! ```ignore
//! use rf_jobs::chain::JobChain;
//!
//! JobChain::new()
//!     .then(ProcessPodcast::new())
//!     .then(OptimizePodcast::new())
//!     .then(ReleasePodcast::new())
//!     .dispatch(&queue)
//!     .await?;
//! ```

use crate::error::QueueError;
use crate::job::{Job, JobPayload};
use crate::queue::QueueManager;
use chrono::{DateTime, Utc};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A chain of jobs to be executed sequentially
#[derive(Clone)]
pub struct JobChain {
    jobs: Vec<SerializedJob>,
    name: Option<String>,
    chain_id: Uuid,
}

/// Serialized job wrapper for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SerializedJob {
    job_type: String,
    data: serde_json::Value,
    queue: String,
}

/// Chain state stored in Redis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainState {
    pub chain_id: Uuid,
    pub name: Option<String>,
    pub total_jobs: usize,
    pub current_index: usize,
    pub status: ChainStatus,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Chain execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChainStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

impl JobChain {
    /// Create a new job chain
    pub fn new() -> Self {
        Self {
            jobs: Vec::new(),
            name: None,
            chain_id: Uuid::new_v4(),
        }
    }

    /// Set chain name for easier identification
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Add a job to the chain
    pub fn then<J: Job + 'static>(mut self, job: J) -> Result<Self, serde_json::Error> {
        let serialized = SerializedJob {
            job_type: std::any::type_name::<J>().to_string(),
            data: serde_json::to_value(&job)?,
            queue: job.queue().to_string(),
        };
        self.jobs.push(serialized);
        Ok(self)
    }

    /// Get chain ID
    pub fn id(&self) -> Uuid {
        self.chain_id
    }

    /// Dispatch the chain to the queue
    pub async fn dispatch(self, queue: &QueueManager) -> Result<Uuid, QueueError> {
        if self.jobs.is_empty() {
            return Err(QueueError::InvalidConfig("Chain has no jobs".into()));
        }

        let chain_id = self.chain_id;

        // Initialize chain state in Redis
        let state = ChainState {
            chain_id,
            name: self.name.clone(),
            total_jobs: self.jobs.len(),
            current_index: 0,
            status: ChainStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        };

        queue.save_chain_state(&state).await?;
        queue.save_chain_jobs(chain_id, &self.jobs).await?;

        // Dispatch first job with chain metadata
        if let Some(first_job) = self.jobs.first() {
            queue.dispatch_chain_job(chain_id, 0, first_job).await?;
        }

        Ok(chain_id)
    }
}

impl Default for JobChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Chain management methods for QueueManager
impl QueueManager {
    /// Save chain state to Redis
    pub async fn save_chain_state(&self, state: &ChainState) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("chain:{}:state", state.chain_id);
        let json = serde_json::to_string(state)?;
        conn.set::<_, _, ()>(&key, json).await?;
        Ok(())
    }

    /// Load chain state from Redis
    pub async fn load_chain_state(&self, chain_id: Uuid) -> Result<ChainState, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("chain:{}:state", chain_id);
        let json: String = conn
            .get(&key)
            .await
            .map_err(|_| QueueError::JobNotFound(chain_id))?;
        let state: ChainState = serde_json::from_str(&json)?;
        Ok(state)
    }

    /// Save chain jobs to Redis
    pub async fn save_chain_jobs(
        &self,
        chain_id: Uuid,
        jobs: &[SerializedJob],
    ) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("chain:{}:jobs", chain_id);
        let json = serde_json::to_string(jobs)?;
        conn.set::<_, _, ()>(&key, json).await?;
        Ok(())
    }

    /// Load chain jobs from Redis
    pub async fn load_chain_jobs(&self, chain_id: Uuid) -> Result<Vec<SerializedJob>, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let key = format!("chain:{}:jobs", chain_id);
        let json: String = conn
            .get(&key)
            .await
            .map_err(|_| QueueError::JobNotFound(chain_id))?;
        let jobs: Vec<SerializedJob> = serde_json::from_str(&json)?;
        Ok(jobs)
    }

    /// Dispatch a chain job with metadata
    pub async fn dispatch_chain_job(
        &self,
        chain_id: Uuid,
        index: usize,
        job: &SerializedJob,
    ) -> Result<Uuid, QueueError> {
        // Create a JobPayload with chain metadata
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

        // Store chain metadata in job payload
        if let Some(obj) = payload.data.as_object_mut() {
            obj.insert("__chain_id".to_string(), serde_json::json!(chain_id));
            obj.insert("__chain_index".to_string(), serde_json::json!(index));
        }

        let job_id = payload.id;
        let queue_name = payload.queue.clone();
        self.push_to_queue(&queue_name, payload).await?;

        Ok(job_id)
    }

    /// Handle chain job completion
    pub async fn handle_chain_job_completion(
        &self,
        chain_id: Uuid,
        index: usize,
    ) -> Result<(), QueueError> {
        // Load chain state
        let mut state = self.load_chain_state(chain_id).await?;

        // Update state
        if state.started_at.is_none() {
            state.started_at = Some(Utc::now());
        }
        state.current_index = index + 1;

        if state.current_index >= state.total_jobs {
            // Chain completed
            state.status = ChainStatus::Completed;
            state.completed_at = Some(Utc::now());
            self.save_chain_state(&state).await?;
        } else {
            // Dispatch next job
            state.status = ChainStatus::Running;
            self.save_chain_state(&state).await?;

            let jobs = self.load_chain_jobs(chain_id).await?;
            if let Some(next_job) = jobs.get(state.current_index) {
                self.dispatch_chain_job(chain_id, state.current_index, next_job)
                    .await?;
            }
        }

        Ok(())
    }

    /// Handle chain job failure
    pub async fn handle_chain_job_failure(
        &self,
        chain_id: Uuid,
        error: String,
    ) -> Result<(), QueueError> {
        let mut state = self.load_chain_state(chain_id).await?;
        state.status = ChainStatus::Failed;
        state.error = Some(error);
        state.completed_at = Some(Utc::now());
        self.save_chain_state(&state).await?;
        Ok(())
    }

    /// Cancel a chain
    pub async fn cancel_chain(&self, chain_id: Uuid) -> Result<(), QueueError> {
        let mut state = self.load_chain_state(chain_id).await?;
        if state.status == ChainStatus::Pending || state.status == ChainStatus::Running {
            state.status = ChainStatus::Cancelled;
            state.completed_at = Some(Utc::now());
            self.save_chain_state(&state).await?;
        }
        Ok(())
    }

    /// Get chain progress
    pub async fn chain_progress(&self, chain_id: Uuid) -> Result<(usize, usize), QueueError> {
        let state = self.load_chain_state(chain_id).await?;
        Ok((state.current_index, state.total_jobs))
    }

    /// Delete chain data
    pub async fn delete_chain(&self, chain_id: Uuid) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;
        let state_key = format!("chain:{}:state", chain_id);
        let jobs_key = format!("chain:{}:jobs", chain_id);

        conn.del::<_, ()>(&state_key).await?;
        conn.del::<_, ()>(&jobs_key).await?;

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
    fn test_chain_creation() {
        let chain = JobChain::new()
            .then(TestJob { value: 1 })
            .unwrap()
            .then(TestJob { value: 2 })
            .unwrap()
            .then(TestJob { value: 3 })
            .unwrap();

        assert_eq!(chain.jobs.len(), 3);
    }

    #[test]
    fn test_chain_with_name() {
        let chain = JobChain::new()
            .name("test-chain")
            .then(TestJob { value: 1 })
            .unwrap();

        assert_eq!(chain.name, Some("test-chain".to_string()));
    }

    #[test]
    fn test_chain_status_transitions() {
        let state = ChainState {
            chain_id: Uuid::new_v4(),
            name: None,
            total_jobs: 3,
            current_index: 0,
            status: ChainStatus::Pending,
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
        };

        assert_eq!(state.status, ChainStatus::Pending);
        assert!(state.started_at.is_none());
    }
}
