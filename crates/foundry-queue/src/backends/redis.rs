use async_trait::async_trait;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;

use crate::backends::QueueBackend;
use crate::error::{QueueError, QueueResult};
use crate::job::{Job, JobStatus};

/// Redis queue backend (for production)
pub struct RedisBackend {
    pool: Pool,
    prefix: String,
}

impl RedisBackend {
    /// Create a new Redis backend with default prefix
    pub fn new(url: impl Into<String>) -> QueueResult<Self> {
        Self::with_prefix(url, "queue:")
    }

    /// Create a new Redis backend with custom prefix
    pub fn with_prefix(url: impl Into<String>, prefix: impl Into<String>) -> QueueResult<Self> {
        let cfg = Config {
            url: Some(url.into()),
            ..Default::default()
        };

        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| QueueError::Connection(e.to_string()))?;

        Ok(Self {
            pool,
            prefix: prefix.into(),
        })
    }

    /// Create from environment variables
    pub fn from_env() -> QueueResult<Self> {
        let url = std::env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let prefix = std::env::var("QUEUE_PREFIX")
            .unwrap_or_else(|_| "queue:".to_string());
        Self::with_prefix(url, prefix)
    }

    /// Make a Redis key with prefix
    fn make_key(&self, suffix: &str) -> String {
        format!("{}{}", self.prefix, suffix)
    }

    /// Get queue key for a specific queue name
    fn queue_key(&self, queue: &str) -> String {
        self.make_key(&format!("queues:{}", queue))
    }

    /// Get delayed queue key
    fn delayed_key(&self) -> String {
        self.make_key("delayed")
    }

    /// Get failed queue key
    fn failed_key(&self) -> String {
        self.make_key("failed")
    }

    /// Get job storage key
    fn job_key(&self, job_id: &str) -> String {
        self.make_key(&format!("jobs:{}", job_id))
    }
}

#[async_trait]
impl QueueBackend for RedisBackend {
    async fn push(&self, job: Job) -> QueueResult<()> {
        let mut conn = self.pool.get().await?;

        // Store job data
        let job_data = job.to_bytes()?;
        let job_key = self.job_key(&job.id);
        conn.set::<_, _, ()>(&job_key, job_data).await?;

        // Handle delayed jobs
        if job.status == JobStatus::Delayed {
            if let Some(execute_at) = job.execute_at {
                // Use sorted set with execution time as score
                let delayed_key = self.delayed_key();
                conn.zadd::<_, _, _, ()>(&delayed_key, &job.id, execute_at)
                    .await?;
            }
            return Ok(());
        }

        // Add to queue using list (LPUSH for FIFO with RPOP)
        let queue_key = self.queue_key(&job.queue);

        // If job has priority, use sorted set instead
        if job.priority != 0 {
            let priority_key = format!("{}:priority", queue_key);
            conn.zadd::<_, _, _, ()>(&priority_key, &job.id, -job.priority)
                .await?;
        } else {
            conn.lpush::<_, _, ()>(&queue_key, &job.id).await?;
        }

        Ok(())
    }

    async fn pop(&self) -> QueueResult<Option<Job>> {
        self.pop_from("default").await
    }

    async fn pop_from(&self, queue: &str) -> QueueResult<Option<Job>> {
        let mut conn = self.pool.get().await?;
        let queue_key = self.queue_key(queue);

        // First check priority queue
        let priority_key = format!("{}:priority", queue_key);

        // Try to get highest priority job
        let job_id: Option<String> = redis::cmd("ZPOPMIN")
            .arg(&priority_key)
            .query_async(&mut *conn)
            .await
            .ok()
            .and_then(|v: Vec<String>| v.first().cloned());

        let job_id = if let Some(id) = job_id {
            Some(id)
        } else {
            // Fall back to regular FIFO queue
            conn.rpop::<_, Option<String>>(&queue_key, None).await?
        };

        if let Some(id) = job_id {
            let job_key = self.job_key(&id);
            let job_data: Option<Vec<u8>> = conn.get(&job_key).await?;

            if let Some(data) = job_data {
                let job = Job::from_bytes(&data)?;
                return Ok(Some(job));
            }
        }

        Ok(None)
    }

    async fn size(&self) -> QueueResult<usize> {
        let mut conn = self.pool.get().await?;

        // Get all queue keys
        let pattern = self.queue_key("*");
        let keys: Vec<String> = conn.keys(&pattern).await?;

        let mut total = 0;
        for key in keys {
            // Check if it's a priority queue (sorted set)
            if key.ends_with(":priority") {
                let count: usize = conn.zcard(&key).await?;
                total += count;
            } else {
                let count: usize = conn.llen(&key).await?;
                total += count;
            }
        }

        Ok(total)
    }

    async fn size_of(&self, queue: &str) -> QueueResult<usize> {
        let mut conn = self.pool.get().await?;
        let queue_key = self.queue_key(queue);
        let priority_key = format!("{}:priority", queue_key);

        let list_size: usize = conn.llen(&queue_key).await?;
        let priority_size: usize = conn.zcard(&priority_key).await.unwrap_or(0);

        Ok(list_size + priority_size)
    }

    async fn clear(&self) -> QueueResult<()> {
        let mut conn = self.pool.get().await?;

        // Get all keys with our prefix
        let pattern = self.make_key("*");
        let keys: Vec<String> = conn.keys(&pattern).await?;

        if !keys.is_empty() {
            conn.del::<_, ()>(&keys).await?;
        }

        Ok(())
    }

    async fn clear_queue(&self, queue: &str) -> QueueResult<()> {
        let mut conn = self.pool.get().await?;
        let queue_key = self.queue_key(queue);
        let priority_key = format!("{}:priority", queue_key);

        conn.del::<_, ()>(&[&queue_key, &priority_key]).await?;

        Ok(())
    }

    async fn get(&self, job_id: &str) -> QueueResult<Option<Job>> {
        let mut conn = self.pool.get().await?;
        let job_key = self.job_key(job_id);

        let job_data: Option<Vec<u8>> = conn.get(&job_key).await?;

        if let Some(data) = job_data {
            let job = Job::from_bytes(&data)?;
            Ok(Some(job))
        } else {
            Ok(None)
        }
    }

    async fn delete(&self, job_id: &str) -> QueueResult<bool> {
        let mut conn = self.pool.get().await?;
        let job_key = self.job_key(job_id);

        let deleted: i32 = conn.del(&job_key).await?;
        Ok(deleted > 0)
    }

    async fn update(&self, job: &Job) -> QueueResult<()> {
        let mut conn = self.pool.get().await?;

        // Update job data
        let job_data = job.to_bytes()?;
        let job_key = self.job_key(&job.id);
        conn.set::<_, _, ()>(&job_key, job_data).await?;

        // If job failed, add to failed set
        if job.status == JobStatus::Failed {
            let failed_key = self.failed_key();
            conn.sadd::<_, _, ()>(&failed_key, &job.id).await?;
        }

        Ok(())
    }

    async fn get_failed(&self) -> QueueResult<Vec<Job>> {
        let mut conn = self.pool.get().await?;
        let failed_key = self.failed_key();

        let job_ids: Vec<String> = conn.smembers(&failed_key).await?;

        let mut jobs = Vec::new();
        for job_id in job_ids {
            if let Some(job) = self.get(&job_id).await? {
                jobs.push(job);
            }
        }

        Ok(jobs)
    }

    async fn get_ready_delayed(&self) -> QueueResult<Vec<Job>> {
        let mut conn = self.pool.get().await?;
        let delayed_key = self.delayed_key();
        let now = chrono::Utc::now().timestamp();

        // Get all jobs with score (execute_at) <= now
        let job_ids: Vec<String> = conn
            .zrangebyscore(&delayed_key, "-inf", now)
            .await?;

        let mut jobs = Vec::new();
        for job_id in job_ids {
            if let Some(job) = self.get(&job_id).await? {
                if job.should_execute() {
                    jobs.push(job);
                }
            }
        }

        Ok(jobs)
    }

    async fn release_delayed(&self, job: &Job) -> QueueResult<()> {
        let mut conn = self.pool.get().await?;
        let delayed_key = self.delayed_key();

        // Remove from delayed set
        conn.zrem::<_, _, ()>(&delayed_key, &job.id).await?;

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

    async fn create_test_backend() -> QueueResult<RedisBackend> {
        RedisBackend::new("redis://127.0.0.1:6379")
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_push_pop() {
        let backend = create_test_backend().await.unwrap();
        backend.clear().await.unwrap();

        let job = Job::new("test").with_payload(json!({"key": "value"}));
        backend.push(job.clone()).await.unwrap();

        let size = backend.size().await.unwrap();
        assert_eq!(size, 1);

        let popped = backend.pop().await.unwrap();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().name, "test");

        backend.clear().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_priority() {
        let backend = create_test_backend().await.unwrap();
        backend.clear().await.unwrap();

        let low = Job::new("low").with_priority(1);
        let high = Job::new("high").with_priority(10);

        backend.push(low).await.unwrap();
        backend.push(high.clone()).await.unwrap();

        // High priority should be popped first
        let popped = backend.pop().await.unwrap();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().name, "high");

        backend.clear().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_delayed() {
        let backend = create_test_backend().await.unwrap();
        backend.clear().await.unwrap();

        let job = Job::new("delayed").with_delay(Duration::from_secs(60));
        backend.push(job.clone()).await.unwrap();

        // Should not be in regular queue
        let size = backend.size().await.unwrap();
        assert_eq!(size, 0);

        // Should be retrievable by ID
        let retrieved = backend.get(&job.id).await.unwrap();
        assert!(retrieved.is_some());

        backend.clear().await.unwrap();
    }

    #[tokio::test]
    #[ignore] // Requires Redis to be running
    async fn test_redis_failed_jobs() {
        let backend = create_test_backend().await.unwrap();
        backend.clear().await.unwrap();

        let mut job = Job::new("test");
        job.mark_failed();

        backend.update(&job).await.unwrap();

        let failed = backend.get_failed().await.unwrap();
        assert!(!failed.is_empty());

        backend.clear().await.unwrap();
    }
}
