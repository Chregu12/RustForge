//! Queue management with Redis backend

use crate::error::QueueError;
use crate::job::{FailedJob, Job, JobPayload};
use crate::routing::JobRouter;
use deadpool_redis::{Config, Pool, Runtime};
use redis::AsyncCommands;
use serde_json;
use std::time::Duration;
use uuid::Uuid;

/// Queue priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueuePriority {
    High,
    Default,
    Low,
}

impl QueuePriority {
    /// Get the queue name suffix for this priority
    pub fn suffix(&self) -> &str {
        match self {
            QueuePriority::High => "high",
            QueuePriority::Default => "default",
            QueuePriority::Low => "low",
        }
    }

    /// Get priority order (lower number = higher priority)
    pub fn order(&self) -> u8 {
        match self {
            QueuePriority::High => 0,
            QueuePriority::Default => 1,
            QueuePriority::Low => 2,
        }
    }
}

/// Queue manager for job dispatching and retrieval
#[derive(Clone)]
pub struct QueueManager {
    pub(crate) pool: Pool,
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
        let pool = cfg.create_pool(Some(Runtime::Tokio1)).map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Pool creation failed",
                e.to_string(),
            )))
        })?;

        Ok(Self { pool })
    }

    /// Resolve the base queue for a job type.
    ///
    /// A route registered via [`JobRouter`](crate::JobRouter) wins over the
    /// job's [`Job::queue`](crate::Job::queue) default. An explicit
    /// [`dispatch_to`](Self::dispatch_to) call bypasses this and always wins.
    fn resolve_queue<J: Job>(job: &J) -> String {
        match JobRouter::resolve(std::any::type_name::<J>()) {
            // TODO: honor `route.connection` once QueueManager supports
            // multiple named connections; for now we only route the queue.
            Some(route) => route.queue,
            None => job.queue().to_string(),
        }
    }

    /// Dispatch job to its default queue
    ///
    /// If a route is registered for the job's type via
    /// [`JobRouter`](crate::JobRouter), that route's queue is used instead of
    /// the [`Job::queue`](crate::Job::queue) default.
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
        let queue = Self::resolve_queue(&job);
        self.dispatch_to(job, &queue).await
    }

    /// Dispatch job to specific queue
    pub async fn dispatch_to<J: Job>(&self, job: J, queue: &str) -> Result<Uuid, QueueError> {
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

    /// Dispatch job to specific queue with priority
    pub async fn dispatch_on<J: Job>(
        &self,
        job: J,
        queue: &str,
        priority: QueuePriority,
    ) -> Result<Uuid, QueueError> {
        let queue_name = format!("{}:{}", queue, priority.suffix());
        self.dispatch_to(job, &queue_name).await
    }

    /// Dispatch job with priority (to its default queue)
    pub async fn dispatch_with_priority<J: Job>(
        &self,
        job: J,
        priority: QueuePriority,
    ) -> Result<Uuid, QueueError> {
        let base_queue = Self::resolve_queue(&job);
        let queue_name = format!("{}:{}", base_queue, priority.suffix());
        self.dispatch_to(job, &queue_name).await
    }

    /// Push raw job payload to queue
    ///
    /// This method is used internally for retrying failed jobs,
    /// preserving the original payload and attempt counter.
    pub async fn push_raw(&self, queue: &str, payload: JobPayload) -> Result<(), QueueError> {
        self.push_to_queue(queue, payload).await
    }

    /// Push job payload to queue
    pub(crate) async fn push_to_queue(
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

        conn.rpush::<_, _, ()>(&queue_key, json).await?;

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

        conn.zadd::<_, _, _, ()>("queue:delayed", json, score)
            .await?;

        Ok(())
    }

    /// Pop job from queue (blocking)
    pub async fn pop(
        &self,
        queue: &str,
        timeout: Duration,
    ) -> Result<Option<JobPayload>, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        let queue_key = format!("queue:{}", queue);

        // Use BLPOP for blocking pop
        let result: Option<(String, String)> =
            conn.blpop(&queue_key, timeout.as_secs() as f64).await?;

        match result {
            Some((_key, json)) => {
                let payload: JobPayload = serde_json::from_str(&json)?;
                Ok(Some(payload))
            }
            None => Ok(None),
        }
    }

    /// Pop job from queue with priority support (blocking)
    ///
    /// Tries to pop from high priority first, then default, then low
    pub async fn pop_with_priority(
        &self,
        queue: &str,
        timeout: Duration,
    ) -> Result<Option<JobPayload>, QueueError> {
        let mut conn = self.pool.get().await.map_err(|e| {
            QueueError::ConnectionError(redis::RedisError::from((
                redis::ErrorKind::IoError,
                "Failed to get connection",
                e.to_string(),
            )))
        })?;

        // Build queue keys in priority order
        let queue_keys = vec![
            format!("queue:{}:high", queue),
            format!("queue:{}:default", queue),
            format!("queue:{}:low", queue),
        ];

        // Use BLPOP with multiple keys (Redis pops from first non-empty queue)
        let result: Option<(String, String)> =
            conn.blpop(&queue_keys, timeout.as_secs() as f64).await?;

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
        let jobs: Vec<String> = conn.zrangebyscore("queue:delayed", 0, now).await?;

        let mut moved = 0;

        for json in jobs {
            let payload: JobPayload = serde_json::from_str(&json)?;

            // Move to appropriate queue
            self.push_to_queue(&payload.queue, payload.clone()).await?;

            // Remove from delayed queue
            conn.zrem::<_, _, ()>("queue:delayed", &json).await?;

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
        conn.del::<_, ()>(&queue_key).await?;

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

        conn.rpush::<_, _, ()>("queue:failed", json).await?;

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

        for (_idx, failed) in failed_jobs.iter().enumerate() {
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
                conn.lrem::<_, _, ()>("queue:failed", 1, &json).await?;

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

        conn.del::<_, ()>("queue:failed").await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::routing::JobRouter;
    use crate::Job;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};
    use std::sync::Mutex;

    /// Serializes every test that mutates the process-global route registry to
    /// prevent flaky cross-test races.
    static ROUTE_TEST_GUARD: Mutex<()> = Mutex::new(());

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob {
        value: i32,
    }

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self, _ctx: crate::JobContext) -> crate::JobResult {
            Ok(())
        }

        fn queue(&self) -> &str {
            "default"
        }
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct RoutedTestJob {
        value: i32,
    }

    #[async_trait]
    impl Job for RoutedTestJob {
        async fn handle(&self, _ctx: crate::JobContext) -> crate::JobResult {
            Ok(())
        }

        fn queue(&self) -> &str {
            "default"
        }
    }

    /// Check if Redis is available for testing
    async fn redis_available() -> bool {
        match redis::Client::open("redis://localhost:6379") {
            Ok(client) => {
                match client.get_multiplexed_async_connection().await {
                    Ok(_) => true,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    // Note: These tests require Redis to be running
    // They are marked with #[ignore] by default

    #[tokio::test]
    async fn test_queue_dispatch() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_queue_dispatch: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let manager = QueueManager::new("redis://localhost:6379").await.unwrap();

        let job = TestJob { value: 42 };
        let job_id = manager.dispatch(job).await.unwrap();

        assert!(!job_id.is_nil());
    }

    #[tokio::test]
    async fn test_queue_size() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_queue_size: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let manager = QueueManager::new("redis://localhost:6379").await.unwrap();

        manager.clear("test").await.unwrap();

        let job = TestJob { value: 42 };
        manager.dispatch_to(job, "test").await.unwrap();

        let size = manager.size("test").await.unwrap();
        assert_eq!(size, 1);
    }

    #[tokio::test]
    async fn test_queue_pop() {
        if !redis_available().await {
            eprintln!("⏭️  Skipping test_queue_pop: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }
        let manager = QueueManager::new("redis://localhost:6379").await.unwrap();

        manager.clear("test").await.unwrap();

        let job = TestJob { value: 42 };
        manager.dispatch_to(job, "test").await.unwrap();

        let payload = manager.pop_nowait("test").await.unwrap();
        assert!(payload.is_some());
    }

    #[test]
    fn test_resolve_queue_uses_registered_route() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        // Without a route, falls back to `Job::queue()`.
        let job = RoutedTestJob { value: 1 };
        assert_eq!(QueueManager::resolve_queue(&job), "default");

        // A registered route wins over the `Job::queue()` default.
        JobRouter::route::<RoutedTestJob>("routed");
        assert_eq!(QueueManager::resolve_queue(&job), "routed");

        JobRouter::clear();
    }

    #[tokio::test]
    async fn test_dispatch_lands_in_routed_queue() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        if !redis_available().await {
            eprintln!("⏭️  Skipping test_dispatch_lands_in_routed_queue: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            JobRouter::clear();
            return;
        }
        let manager = QueueManager::new("redis://localhost:6379").await.unwrap();
        manager.clear("routed-q").await.unwrap();
        manager.clear("default").await.unwrap();

        JobRouter::route::<RoutedTestJob>("routed-q");

        let job = RoutedTestJob { value: 42 };
        manager.dispatch(job).await.unwrap();

        // The routed queue received the job; the default queue did not.
        assert_eq!(manager.size("routed-q").await.unwrap(), 1);

        let payload = manager.pop_nowait("routed-q").await.unwrap();
        assert!(payload.is_some());
        assert_eq!(payload.unwrap().queue, "default"); // payload.queue still from Job::queue()

        JobRouter::clear();
    }

    #[tokio::test]
    async fn test_dispatch_to_overrides_route() {
        let _guard = ROUTE_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        JobRouter::clear();

        if !redis_available().await {
            eprintln!("⏭️  Skipping test_dispatch_to_overrides_route: Redis not available");
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            JobRouter::clear();
            return;
        }
        let manager = QueueManager::new("redis://localhost:6379").await.unwrap();
        manager.clear("routed-q").await.unwrap();
        manager.clear("explicit-q").await.unwrap();

        // Even with a route registered, an explicit `dispatch_to` wins.
        JobRouter::route::<RoutedTestJob>("routed-q");

        let job = RoutedTestJob { value: 7 };
        manager.dispatch_to(job, "explicit-q").await.unwrap();

        assert_eq!(manager.size("explicit-q").await.unwrap(), 1);
        assert_eq!(manager.size("routed-q").await.unwrap(), 0);

        JobRouter::clear();
    }
}
