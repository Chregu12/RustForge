//! Queue management with Redis backend

use crate::error::QueueError;
use crate::job::{FailedJob, Job, JobPayload};
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use serde_json;
use std::time::Duration;
use uuid::Uuid;

/// Queue manager for job dispatching and retrieval
#[derive(Clone)]
pub struct QueueManager {
    pool: Pool,
}

impl QueueManager {
    /// Create new queue manager
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rf_jobs::QueueManager;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = QueueManager::new("redis://localhost:6379").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(redis_url: &str) -> Result<Self, QueueError> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Pool creation failed",
                e.to_string(),
            ))))?;

        Ok(Self { pool })
    }

    /// Dispatch job to its default queue
    ///
    /// # Example
    ///
    /// ```ignore
    /// # use rf_jobs::{QueueManager, Job};
    /// # async fn example(manager: QueueManager, job: impl Job) -> Result<(), Box<dyn std::error::Error>> {
    /// let job_id = manager.dispatch(job).await?;
    /// println!("Dispatched job: {}", job_id);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn dispatch<J: Job>(&self, job: J) -> Result<Uuid, QueueError> {
        let queue = job.queue().to_string();
        self.dispatch_to(job, &queue).await
    }

    /// Dispatch job to specific queue
    pub async fn dispatch_to<J: Job>(
        &self,
        job: J,
        queue: &str,
    ) -> Result<Uuid, QueueError> {
        let payload = JobPayload::new(job)?;
        let job_id = payload.id;

        self.push_to_queue(queue, payload).await?;

        Ok(job_id)
    }

    /// Dispatch job with delay
    pub async fn dispatch_later<J: Job>(
        &self,
        job: J,
        delay: Duration,
    ) -> Result<Uuid, QueueError> {
        let mut payload = JobPayload::new(job)?;
        let job_id = payload.id;

        // Set available_at to future time
        payload.available_at = chrono::Utc::now()
            + chrono::Duration::from_std(delay)
                .map_err(|_| QueueError::InvalidConfig("Invalid delay duration".into()))?;

        self.push_to_delayed_queue(payload).await?;

        Ok(job_id)
    }

    /// Push job payload to queue
    async fn push_to_queue(
        &self,
        queue: &str,
        payload: JobPayload,
    ) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let queue_key = format!("queue:{}", queue);
        let json = serde_json::to_string(&payload)?;

        conn.rpush(&queue_key, json).await?;

        Ok(())
    }

    /// Push job to delayed queue (sorted set by available_at)
    async fn push_to_delayed_queue(&self, payload: JobPayload) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let json = serde_json::to_string(&payload)?;
        let score = payload.available_at.timestamp();

        conn.zadd("queue:delayed", json, score).await?;

        Ok(())
    }

    /// Pop job from queue (blocking)
    pub async fn pop(&self, queue: &str, timeout: Duration) -> Result<Option<JobPayload>, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let queue_key = format!("queue:{}", queue);

        // Use BLPOP for blocking pop
        let result: Option<(String, String)> = conn
            .blpop(&queue_key, timeout.as_secs() as f64)
            .await?;

        match result {
            Some((_key, json)) => {
                let payload: JobPayload = serde_json::from_str(&json)?;
                Ok(Some(payload))
            }
            None => Ok(None),
        }
    }

    /// Pop job from queue (non-blocking)
    pub async fn pop_nowait(&self, queue: &str) -> Result<Option<JobPayload>, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let queue_key = format!("queue:{}", queue);
        let result: Option<String> = conn.lpop(&queue_key, None).await?;

        match result {
            Some(json) => {
                let payload: JobPayload = serde_json::from_str(&json)?;
                Ok(Some(payload))
            }
            None => Ok(None),
        }
    }

    /// Move delayed jobs that are now available
    pub async fn move_delayed_jobs(&self) -> Result<u64, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let now = chrono::Utc::now().timestamp();

        // Get all jobs with score <= now
        let jobs: Vec<String> = conn
            .zrangebyscore("queue:delayed", 0, now)
            .await?;

        let mut moved = 0;

        for json in jobs {
            let payload: JobPayload = serde_json::from_str(&json)?;

            // Move to appropriate queue
            self.push_to_queue(&payload.queue, payload.clone()).await?;

            // Remove from delayed queue
            conn.zrem("queue:delayed", &json).await?;

            moved += 1;
        }

        Ok(moved)
    }

    /// Get queue size
    pub async fn size(&self, queue: &str) -> Result<u64, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let queue_key = format!("queue:{}", queue);
        let size: u64 = conn.llen(&queue_key).await?;

        Ok(size)
    }

    /// Clear queue
    pub async fn clear(&self, queue: &str) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let queue_key = format!("queue:{}", queue);
        conn.del(&queue_key).await?;

        Ok(())
    }

    /// Add job to failed queue
    pub async fn add_failed_job(
        &self,
        payload: JobPayload,
        error: String,
    ) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let failed = FailedJob::new(payload, error);
        let json = serde_json::to_string(&failed)?;

        conn.rpush("queue:failed", json).await?;

        Ok(())
    }

    /// Get failed jobs
    pub async fn failed_jobs(&self) -> Result<Vec<FailedJob>, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let jobs: Vec<String> = conn.lrange("queue:failed", 0, -1).await?;

        jobs.into_iter()
            .map(|s| serde_json::from_str(&s).map_err(Into::into))
            .collect()
    }

    /// Retry failed job by ID
    pub async fn retry_failed(&self, job_id: Uuid) -> Result<(), QueueError> {
        let failed_jobs = self.failed_jobs().await?;

        for (idx, failed) in failed_jobs.iter().enumerate() {
            if failed.payload.id == job_id {
                // Remove from failed queue
                let mut conn = self.pool.get().await.map_err(|e| {
                    QueueError::ConnectionError(redis::RedisError::from((
                        redis::ErrorKind::IoError,
                        "Failed to get connection",
                        e.to_string(),
                    )))
                })?;

                let json = serde_json::to_string(failed)?;
                conn.lrem("queue:failed", 1, &json).await?;

                // Reset attempt counter
                let mut payload = failed.payload.clone();
                payload.attempt = 0;

                // Push back to original queue
                let queue = payload.queue.clone();
                self.push_to_queue(&queue, payload).await?;

                return Ok(());
            }
        }

        Err(QueueError::JobNotFound(job_id))
    }

    /// Clear failed jobs
    pub async fn clear_failed(&self) -> Result<(), QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        conn.del("queue:failed").await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Job;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        value: i32,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self, _ctx: crate::JobContext) -> crate::JobResult {
            Ok(())
        }
    }

    // Note: These tests require Redis to be running
    // They are marked with #[ignore] by default

    #[tokio::test]
    #[ignore]
    async fn test_queue_dispatch() {
        let manager = QueueManager::new("redis://localhost:6379")
            .await
            .unwrap();

        let job = TestJob { value: 42 };
        let job_id = manager.dispatch(job).await.unwrap();

        assert!(!job_id.is_nil());
    }

    #[tokio::test]
    #[ignore]
    async fn test_queue_size() {
        let manager = QueueManager::new("redis://localhost:6379")
            .await
            .unwrap();

        manager.clear("test").await.unwrap();

        let job = TestJob { value: 42 };
        manager.dispatch_to(job, "test").await.unwrap();

        let size = manager.size("test").await.unwrap();
        assert_eq!(size, 1);
    }

    #[tokio::test]
    #[ignore]
    async fn test_queue_pop() {
        let manager = QueueManager::new("redis://localhost:6379")
            .await
            .unwrap();

        manager.clear("test").await.unwrap();

        let job = TestJob { value: 42 };
        manager.dispatch_to(job, "test").await.unwrap();

        let payload = manager.pop_nowait("test").await.unwrap();
        assert!(payload.is_some());
    }
}
