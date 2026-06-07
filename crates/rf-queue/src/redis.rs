//! Redis queue backend for production
//!
//! Provides a production-ready queue implementation using Redis as the backend.
//! Jobs are stored persistently and survive application restarts.
//!
//! ## Features
//!
//! - **Persistent**: Jobs stored in Redis survive restarts
//! - **Distributed**: Multiple workers can process from same queue
//! - **Delayed Jobs**: Jobs scheduled for future execution
//! - **Failed Jobs**: Failed jobs tracked separately
//! - **Connection Pooling**: Efficient connection management
//!
//! ## Example
//!
//! ```no_run
//! use rf_queue::{RedisQueue, Queue, Job, JobMetadata};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let queue = RedisQueue::new("redis://localhost:6379", "default").await?;
//!
//! // Push job to queue
//! # use serde::{Serialize, Deserialize};
//! # use async_trait::async_trait;
//! # #[derive(Serialize, Deserialize)]
//! # struct SendEmailJob { to: String }
//! # #[async_trait]
//! # impl Job for SendEmailJob {
//! #     async fn handle(&self) -> Result<(), rf_queue::QueueError> { Ok(()) }
//! #     fn job_type(&self) -> &'static str { "send_email" }
//! # }
//! let job = SendEmailJob { to: "user@example.com".to_string() };
//! let metadata = JobMetadata::new(&job)?;
//! queue.push(metadata).await?;
//! # Ok(())
//! # }
//! ```

#[cfg(feature = "redis-backend")]
use crate::error::{QueueError, QueueResult};
#[cfg(feature = "redis-backend")]
use crate::job::JobMetadata;
#[cfg(feature = "redis-backend")]
use crate::queue::Queue;
#[cfg(feature = "redis-backend")]
use async_trait::async_trait;
#[cfg(feature = "redis-backend")]
use deadpool_redis::{Config, Pool, Runtime};
#[cfg(feature = "redis-backend")]
use redis::AsyncCommands;

#[cfg(feature = "redis-backend")]
/// Redis queue backend
///
/// Provides a production-ready queue implementation using Redis.
/// Jobs are stored in Redis lists for reliability and distributed processing.
#[derive(Clone)]
pub struct RedisQueue {
    pool: Pool,
    prefix: String,
}

#[cfg(feature = "redis-backend")]
impl RedisQueue {
    /// Create new Redis queue
    ///
    /// # Arguments
    ///
    /// * `redis_url` - Redis connection URL (e.g., "redis://localhost:6379")
    /// * `prefix` - Queue prefix for namespacing
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use rf_queue::RedisQueue;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let queue = RedisQueue::new("redis://localhost:6379", "myapp").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(redis_url: &str, prefix: &str) -> QueueResult<Self> {
        let cfg = Config::from_url(redis_url);
        let pool = cfg
            .create_pool(Some(Runtime::Tokio1))
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        // Test connection
        let mut conn = pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        redis::cmd("PING")
            .query_async::<String>(&mut conn)
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        tracing::info!(redis_url = %redis_url, prefix = %prefix, "Redis queue initialized");

        Ok(Self {
            pool,
            prefix: prefix.to_string(),
        })
    }

    /// Get queue key for a specific queue name
    fn queue_key(&self, queue: &str) -> String {
        format!("{}:queue:{}", self.prefix, queue)
    }

    /// Get delayed jobs key for a specific queue
    fn delayed_key(&self, queue: &str) -> String {
        format!("{}:delayed:{}", self.prefix, queue)
    }

    /// Get failed jobs key for a specific queue
    fn failed_key(&self, queue: &str) -> String {
        format!("{}:failed:{}", self.prefix, queue)
    }

    /// Get processing jobs key for a specific queue
    fn processing_key(&self, queue: &str) -> String {
        format!("{}:processing:{}", self.prefix, queue)
    }

    /// Get job data key
    fn job_key(&self, job_id: &str) -> String {
        format!("{}:job:{}", self.prefix, job_id)
    }

    /// Move delayed jobs that are ready to execute
    async fn move_delayed_jobs(&self, queue: &str) -> QueueResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let delayed_key = self.delayed_key(queue);
        let queue_key = self.queue_key(queue);

        // Get all delayed jobs with score <= now (timestamp)
        let now = chrono::Utc::now().timestamp();

        loop {
            // Get jobs ready to execute (with score <= now)
            let jobs: Vec<(String, f64)> = conn
                .zrangebyscore_withscores(&delayed_key, 0, now)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;

            if jobs.is_empty() {
                break;
            }

            // Move first job to ready queue
            if let Some((job_data, _score)) = jobs.first() {
                // Remove from delayed set
                let removed: i32 = conn
                    .zrem(&delayed_key, job_data)
                    .await
                    .map_err(|e| QueueError::Backend(e.to_string()))?;

                if removed > 0 {
                    // Add to ready queue
                    let _: () = conn
                        .lpush(&queue_key, job_data)
                        .await
                        .map_err(|e| QueueError::Backend(e.to_string()))?;

                    tracing::debug!(queue = %queue, "Moved delayed job to ready queue");
                }
            } else {
                break;
            }
        }

        Ok(())
    }
}

#[cfg(feature = "redis-backend")]
#[async_trait]
impl Queue for RedisQueue {
    async fn push(&self, metadata: JobMetadata) -> QueueResult<String> {
        let job_id = metadata.id.clone();
        let queue_name = metadata.queue.clone();

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let job_data = metadata.to_bytes()?;
        let job_data_str = String::from_utf8(job_data.clone())
            .map_err(|e| QueueError::SerializationError(e.to_string()))?;

        // Store job data
        let job_key = self.job_key(&job_id);
        conn.set_ex(&job_key, job_data, 86400) // 24 hours TTL
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        // If delayed, add to delayed sorted set
        if let Some(execute_at) = metadata.execute_at {
            let delayed_key = self.delayed_key(&queue_name);
            let score = execute_at.timestamp() as f64;

            conn.zadd(&delayed_key, &job_data_str, score)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;

            tracing::debug!(
                job_id = %job_id,
                queue = %queue_name,
                execute_at = %execute_at,
                "Job scheduled for delayed execution"
            );
        } else {
            // Add to ready queue
            let queue_key = self.queue_key(&queue_name);

            conn.lpush(&queue_key, &job_data_str)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;

            tracing::debug!(
                job_id = %job_id,
                queue = %queue_name,
                "Job pushed to ready queue"
            );
        }

        Ok(job_id)
    }

    async fn reserve(&self, queue: &str) -> QueueResult<Option<JobMetadata>> {
        // First, move any delayed jobs that are ready
        self.move_delayed_jobs(queue).await?;

        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let queue_key = self.queue_key(queue);
        let processing_key = self.processing_key(queue);

        // Use BRPOPLPUSH for atomic move from queue to processing
        // Timeout of 1 second
        let result: Option<String> = conn
            .brpoplpush(&queue_key, &processing_key, 1.0)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        if let Some(job_data) = result {
            let metadata = JobMetadata::from_bytes(job_data.as_bytes())?;

            tracing::debug!(
                job_id = %metadata.id,
                queue = %queue,
                "Job reserved for processing"
            );

            Ok(Some(metadata))
        } else {
            Ok(None)
        }
    }

    async fn complete(&self, job_id: &str) -> QueueResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let job_key = self.job_key(job_id);

        // Read job data first so we can remove the entry from the processing list
        let job_data: Option<Vec<u8>> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        if let Some(data) = job_data {
            let metadata = JobMetadata::from_bytes(&data)?;
            let processing_key = self.processing_key(&metadata.queue);

            // Remove job from the processing list (count=1 removes the first match)
            let _: i64 = redis::cmd("LREM")
                .arg(&processing_key)
                .arg(1i64)
                .arg(String::from_utf8_lossy(&data).as_ref())
                .query_async(&mut *conn)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;
        }

        // Delete job data
        conn.del(&job_key)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        tracing::debug!(job_id = %job_id, "Job completed and removed from processing list");

        Ok(())
    }

    async fn fail(&self, job_id: &str, error: &str) -> QueueResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let job_key = self.job_key(job_id);

        // Get job data
        let job_data: Option<Vec<u8>> = conn
            .get(&job_key)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        if let Some(data) = job_data {
            let mut metadata = JobMetadata::from_bytes(&data)?;
            let processing_key = self.processing_key(&metadata.queue);
            let failed_key = self.failed_key(&metadata.queue);

            metadata.mark_error(error.to_string());

            // Remove from processing list before moving to failed queue
            let _: i64 = redis::cmd("LREM")
                .arg(&processing_key)
                .arg(1i64)
                .arg(String::from_utf8_lossy(&data).as_ref())
                .query_async(&mut *conn)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;

            // Store in failed queue
            let failed_data = metadata.to_bytes()?;
            conn.lpush(&failed_key, failed_data)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;

            // Delete original job data
            conn.del(&self.job_key(job_id))
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;

            tracing::warn!(
                job_id = %job_id,
                error = %error,
                queue = %metadata.queue,
                "Job marked as failed and removed from processing list"
            );
        }

        Ok(())
    }

    async fn retry(&self, mut metadata: JobMetadata) -> QueueResult<()> {
        if !metadata.can_retry() {
            return Err(QueueError::JobFailed("Max retries exceeded".to_string()));
        }

        // Remove from processing list before re-enqueuing to avoid duplicates
        {
            let mut conn = self
                .pool
                .get()
                .await
                .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

            let processing_key = self.processing_key(&metadata.queue);
            let current_data = metadata
                .to_bytes()
                .and_then(|b| String::from_utf8(b).map_err(|e| QueueError::SerializationError(e.to_string())))?;

            let _: i64 = redis::cmd("LREM")
                .arg(&processing_key)
                .arg(1i64)
                .arg(&current_data)
                .query_async(&mut *conn)
                .await
                .map_err(|e| QueueError::Backend(e.to_string()))?;
        }

        metadata.mark_attempt();

        // Calculate exponential backoff delay, capping the exponent to prevent overflow.
        // At attempt 63+ the cap kicks in and keeps the delay at ~4.6×10^15 seconds (effectively infinite).
        let exp = (metadata.attempts.saturating_sub(1)).min(62);
        let delay_secs = (2u64.pow(exp)).saturating_mul(60); // 1min, 2min, 4min, etc.
        let execute_at = chrono::Utc::now() + chrono::Duration::seconds(delay_secs as i64);
        metadata.execute_at = Some(execute_at);

        tracing::info!(
            job_id = %metadata.id,
            attempt = %metadata.attempts,
            delay_secs = %delay_secs,
            "Retrying job with exponential backoff"
        );

        self.push(metadata).await?;
        Ok(())
    }

    async fn size(&self, queue: &str) -> QueueResult<usize> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let queue_key = self.queue_key(queue);

        let size: usize = conn
            .llen(&queue_key)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        Ok(size)
    }

    async fn clear(&self, queue: &str) -> QueueResult<()> {
        let mut conn = self
            .pool
            .get()
            .await
            .map_err(|e| QueueError::ConnectionError(e.to_string()))?;

        let queue_key = self.queue_key(queue);
        let delayed_key = self.delayed_key(queue);
        let processing_key = self.processing_key(queue);
        let failed_key = self.failed_key(queue);

        // Delete all queue-related keys
        conn.del(&[&queue_key, &delayed_key, &processing_key, &failed_key])
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;

        tracing::info!(queue = %queue, "Queue cleared");

        Ok(())
    }
}

#[cfg(all(test, feature = "redis-backend"))]
mod tests {
    use super::*;
    use crate::job::Job;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::time::Duration;

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

    async fn create_test_queue() -> RedisQueue {
        let redis_url =
            std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string());
        RedisQueue::new(&redis_url, "test").await.unwrap()
    }

    #[tokio::test]
    async fn test_redis_push_and_reserve() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_push_and_reserve: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let queue = create_test_queue().await;
        queue.clear("default").await.unwrap();

        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        let job_id = queue.push(metadata).await.unwrap();

        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());

        let reserved_metadata = reserved.unwrap();
        assert_eq!(reserved_metadata.id, job_id);
        assert_eq!(reserved_metadata.job_type, "test_job");
    }

    #[tokio::test]
    async fn test_redis_queue_size() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_queue_size: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let queue = create_test_queue().await;
        queue.clear("default").await.unwrap();

        assert_eq!(queue.size("default").await.unwrap(), 0);

        let job = TestJob {
            message: "test".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();

        assert_eq!(queue.size("default").await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_redis_delayed_jobs() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_delayed_jobs: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let queue = create_test_queue().await;
        queue.clear("default").await.unwrap();

        let job = TestJob {
            message: "delayed".to_string(),
        };

        let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(2)).unwrap();
        queue.push(metadata).await.unwrap();

        // Should not be available immediately
        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_none());

        // Wait for delay
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Should now be available
        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());
    }

    #[tokio::test]
    async fn test_redis_job_persistence() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_job_persistence: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let queue = create_test_queue().await;
        queue.clear("default").await.unwrap();

        let job = TestJob {
            message: "persistent".to_string(),
        };

        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();

        // Create new queue instance (simulate restart)
        drop(queue);
        let queue = create_test_queue().await;

        // Job should still be there
        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_some());
        assert_eq!(reserved.unwrap().job_type, "test_job");
    }

    #[tokio::test]
    async fn test_redis_retry_with_backoff() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_redis_retry_with_backoff: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let queue = create_test_queue().await;
        queue.clear("default").await.unwrap();

        let job = TestJob {
            message: "retry".to_string(),
        };

        let mut metadata = JobMetadata::new(&job).unwrap();

        // First retry
        queue.retry(metadata.clone()).await.unwrap();
        assert_eq!(metadata.attempts, 0); // Not incremented until retry

        // Job should be delayed
        let reserved = queue.reserve("default").await.unwrap();
        assert!(reserved.is_none()); // Not available yet due to backoff
    }
}
