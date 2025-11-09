# API Sketch: rf-jobs - Background Jobs & Queue System

**Status**: Draft
**Phase**: Phase 2 - Modular Rebuild
**PR-Slice**: #7

## Overview

`rf-jobs` provides a production-ready background job processing system with:
- Asynchronous job queue with Redis backend
- Worker pool with configurable concurrency
- Job scheduling (cron-like patterns)
- Retry logic with exponential backoff
- Failed job handling (Dead Letter Queue)
- Job chaining and batching
- Priority queues
- Job progress tracking

**Inspiration**: Laravel's Queue system + Sidekiq + Bull

## Core Concepts

### 1. Job Definition

Jobs are defined as structs implementing the `Job` trait:

```rust
use rf_jobs::{Job, JobContext, JobResult};
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendEmailJob {
    pub to: String,
    pub subject: String,
    pub body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // Send email logic
        println!("Sending email to {}", self.to);

        // Access context
        ctx.log(&format!("Attempt {}/{}", ctx.attempt(), ctx.max_attempts()));

        // Simulate work
        tokio::time::sleep(Duration::from_secs(2)).await;

        Ok(())
    }

    fn queue(&self) -> &str {
        "emails"
    }

    fn max_attempts(&self) -> u32 {
        3
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(60) // 1 minute
    }
}
```

### 2. Job Dispatching

```rust
use rf_jobs::{Queue, QueueManager};

// Dispatch job to queue
let job = SendEmailJob {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
    body: "Thanks for signing up".to_string(),
};

// Simple dispatch
Queue::dispatch(job).await?;

// Dispatch with delay
Queue::dispatch(job)
    .delay(Duration::from_secs(300)) // 5 minutes
    .await?;

// Dispatch to specific queue
Queue::on("high-priority")
    .dispatch(job)
    .await?;

// Dispatch with options
Queue::dispatch(job)
    .on_queue("emails")
    .delay(Duration::from_secs(60))
    .max_attempts(5)
    .await?;
```

### 3. Worker Pool

```rust
use rf_jobs::{WorkerPool, WorkerConfig};

// Configure worker pool
let config = WorkerConfig::default()
    .workers(4)                    // 4 concurrent workers
    .queues(&["default", "emails"]) // Listen to multiple queues
    .timeout(Duration::from_secs(30));

// Start workers
let pool = WorkerPool::new(config).await?;
pool.start().await?;

// Graceful shutdown
pool.shutdown().await?;
```

### 4. Job Scheduling

```rust
use rf_jobs::{Schedule, Scheduler};

// Define scheduled job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyReportJob;

#[async_trait]
impl Job for DailyReportJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // Generate daily report
        Ok(())
    }
}

// Register scheduled jobs
let scheduler = Scheduler::new();

scheduler
    .schedule("0 0 * * *")  // Every day at midnight
    .job(DailyReportJob)
    .await?;

scheduler
    .schedule("*/15 * * * *")  // Every 15 minutes
    .job(CacheCleanupJob)
    .await?;

// Start scheduler
scheduler.start().await?;
```

## API Reference

### Job Trait

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[async_trait]
pub trait Job: Send + Sync + Serialize + for<'de> Deserialize<'de> + Clone {
    /// Execute the job
    async fn handle(&self, ctx: JobContext) -> JobResult;

    /// Queue name (default: "default")
    fn queue(&self) -> &str {
        "default"
    }

    /// Maximum retry attempts (default: 3)
    fn max_attempts(&self) -> u32 {
        3
    }

    /// Backoff duration between retries (default: 60s)
    fn backoff(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Timeout for job execution (default: 60s)
    fn timeout(&self) -> Duration {
        Duration::from_secs(60)
    }

    /// Called when job fails after all retries
    async fn failed(&self, _ctx: JobContext, _error: JobError) {
        // Override to handle failed jobs
    }
}

pub type JobResult = Result<(), JobError>;
```

### JobContext

```rust
#[derive(Debug, Clone)]
pub struct JobContext {
    job_id: Uuid,
    queue: String,
    attempt: u32,
    max_attempts: u32,
    dispatched_at: DateTime<Utc>,
    started_at: DateTime<Utc>,
}

impl JobContext {
    /// Unique job ID
    pub fn job_id(&self) -> Uuid {
        self.job_id
    }

    /// Current attempt number (1-indexed)
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Maximum attempts allowed
    pub fn max_attempts(&self) -> u32 {
        self.max_attempts
    }

    /// Is this the final attempt?
    pub fn is_final_attempt(&self) -> bool {
        self.attempt >= self.max_attempts
    }

    /// Queue name
    pub fn queue(&self) -> &str {
        &self.queue
    }

    /// When job was dispatched
    pub fn dispatched_at(&self) -> DateTime<Utc> {
        self.dispatched_at
    }

    /// When job started executing
    pub fn started_at(&self) -> DateTime<Utc> {
        self.started_at
    }

    /// Log message with job context
    pub fn log(&self, message: &str) {
        tracing::info!(
            job_id = %self.job_id,
            queue = %self.queue,
            attempt = self.attempt,
            "{}",
            message
        );
    }
}
```

### Queue Manager

```rust
use redis::Client;

pub struct QueueManager {
    redis: Client,
    config: QueueConfig,
}

impl QueueManager {
    /// Create new queue manager
    pub async fn new(redis_url: &str) -> Result<Self, QueueError> {
        let redis = Client::open(redis_url)?;
        Ok(Self {
            redis,
            config: QueueConfig::default(),
        })
    }

    /// Dispatch job to queue
    pub async fn dispatch<J: Job>(&self, job: J) -> Result<Uuid, QueueError> {
        self.dispatch_to(job, job.queue()).await
    }

    /// Dispatch job to specific queue
    pub async fn dispatch_to<J: Job>(
        &self,
        job: J,
        queue: &str,
    ) -> Result<Uuid, QueueError> {
        let job_id = Uuid::new_v4();
        let payload = JobPayload {
            id: job_id,
            queue: queue.to_string(),
            data: serde_json::to_value(&job)?,
            attempt: 0,
            max_attempts: job.max_attempts(),
            dispatched_at: Utc::now(),
            available_at: Utc::now(),
        };

        self.push_to_queue(queue, payload).await?;
        Ok(job_id)
    }

    /// Dispatch with delay
    pub async fn dispatch_later<J: Job>(
        &self,
        job: J,
        delay: Duration,
    ) -> Result<Uuid, QueueError> {
        let job_id = Uuid::new_v4();
        let available_at = Utc::now() + chrono::Duration::from_std(delay)?;

        let payload = JobPayload {
            id: job_id,
            queue: job.queue().to_string(),
            data: serde_json::to_value(&job)?,
            attempt: 0,
            max_attempts: job.max_attempts(),
            dispatched_at: Utc::now(),
            available_at,
        };

        self.push_to_delayed_queue(payload).await?;
        Ok(job_id)
    }

    /// Get queue size
    pub async fn size(&self, queue: &str) -> Result<u64, QueueError> {
        let mut conn = self.redis.get_async_connection().await?;
        let size: u64 = redis::cmd("LLEN")
            .arg(format!("queue:{}", queue))
            .query_async(&mut conn)
            .await?;
        Ok(size)
    }

    /// Clear queue
    pub async fn clear(&self, queue: &str) -> Result<(), QueueError> {
        let mut conn = self.redis.get_async_connection().await?;
        redis::cmd("DEL")
            .arg(format!("queue:{}", queue))
            .query_async(&mut conn)
            .await?;
        Ok(())
    }

    /// Get failed jobs
    pub async fn failed_jobs(&self) -> Result<Vec<FailedJob>, QueueError> {
        let mut conn = self.redis.get_async_connection().await?;
        let jobs: Vec<String> = redis::cmd("LRANGE")
            .arg("queue:failed")
            .arg(0)
            .arg(-1)
            .query_async(&mut conn)
            .await?;

        jobs.into_iter()
            .map(|s| serde_json::from_str(&s).map_err(Into::into))
            .collect()
    }

    /// Retry failed job
    pub async fn retry_failed(&self, job_id: Uuid) -> Result<(), QueueError> {
        // Move job from failed queue back to original queue
        todo!()
    }
}
```

### Worker Pool

```rust
pub struct WorkerPool {
    config: WorkerConfig,
    queue_manager: Arc<QueueManager>,
    workers: Vec<Worker>,
    shutdown_tx: broadcast::Sender<()>,
}

impl WorkerPool {
    /// Create new worker pool
    pub async fn new(
        config: WorkerConfig,
        queue_manager: QueueManager,
    ) -> Result<Self, WorkerError> {
        let (shutdown_tx, _) = broadcast::channel(1);
        let queue_manager = Arc::new(queue_manager);

        let mut workers = Vec::new();
        for i in 0..config.workers {
            let worker = Worker::new(
                i,
                config.clone(),
                Arc::clone(&queue_manager),
                shutdown_tx.subscribe(),
            );
            workers.push(worker);
        }

        Ok(Self {
            config,
            queue_manager,
            workers,
            shutdown_tx,
        })
    }

    /// Start all workers
    pub async fn start(&mut self) -> Result<(), WorkerError> {
        for worker in &mut self.workers {
            worker.start().await?;
        }
        Ok(())
    }

    /// Graceful shutdown
    pub async fn shutdown(self) -> Result<(), WorkerError> {
        // Signal all workers to stop
        let _ = self.shutdown_tx.send(());

        // Wait for workers to finish current jobs
        for worker in self.workers {
            worker.wait().await?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct WorkerConfig {
    /// Number of concurrent workers
    pub workers: usize,

    /// Queues to listen on (in priority order)
    pub queues: Vec<String>,

    /// Max job execution time
    pub timeout: Duration,

    /// Sleep time when no jobs available
    pub sleep: Duration,

    /// Max concurrent jobs per worker
    pub max_concurrent_jobs: usize,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            workers: num_cpus::get(),
            queues: vec!["default".to_string()],
            timeout: Duration::from_secs(60),
            sleep: Duration::from_secs(1),
            max_concurrent_jobs: 1,
        }
    }
}
```

### Scheduler

```rust
use cron::Schedule;
use std::str::FromStr;

pub struct Scheduler {
    schedules: Vec<ScheduledJob>,
    queue_manager: Arc<QueueManager>,
}

impl Scheduler {
    /// Create new scheduler
    pub fn new(queue_manager: QueueManager) -> Self {
        Self {
            schedules: Vec::new(),
            queue_manager: Arc::new(queue_manager),
        }
    }

    /// Schedule a job
    pub fn schedule<J: Job + 'static>(
        &mut self,
        cron_expr: &str,
    ) -> ScheduleBuilder<J> {
        ScheduleBuilder {
            scheduler: self,
            cron: Schedule::from_str(cron_expr).unwrap(),
            _phantom: PhantomData,
        }
    }

    /// Start scheduler
    pub async fn start(self) -> Result<(), SchedulerError> {
        tokio::spawn(async move {
            self.run().await
        });
        Ok(())
    }

    async fn run(self) {
        loop {
            let now = Utc::now();

            for scheduled_job in &self.schedules {
                if let Some(next) = scheduled_job.schedule.upcoming(Utc).next() {
                    if next <= now {
                        // Dispatch job
                        let job = (scheduled_job.job_factory)();
                        if let Err(e) = self.queue_manager.dispatch(job).await {
                            tracing::error!("Failed to dispatch scheduled job: {}", e);
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}

pub struct ScheduleBuilder<'a, J> {
    scheduler: &'a mut Scheduler,
    cron: Schedule,
    _phantom: PhantomData<J>,
}

impl<'a, J: Job + 'static> ScheduleBuilder<'a, J> {
    pub fn job(self, job: J) {
        let scheduled = ScheduledJob {
            schedule: self.cron,
            job_factory: Box::new(move || Box::new(job.clone())),
        };
        self.scheduler.schedules.push(scheduled);
    }
}
```

### Job Chaining

```rust
use rf_jobs::{Chain, Queue};

// Define jobs
let job1 = ProcessImageJob { path: "image.jpg" };
let job2 = GenerateThumbnailJob { path: "image.jpg" };
let job3 = SendNotificationJob { message: "Processing complete" };

// Chain jobs
Chain::new()
    .then(job1)
    .then(job2)
    .then(job3)
    .dispatch()
    .await?;

// On failure, chain stops
```

### Job Batching

```rust
use rf_jobs::{Batch, Queue};

let jobs: Vec<SendEmailJob> = users
    .iter()
    .map(|user| SendEmailJob {
        to: user.email.clone(),
        subject: "Newsletter".to_string(),
        body: "...".to_string(),
    })
    .collect();

// Dispatch batch
let batch = Batch::new(jobs)
    .name("newsletter-campaign")
    .then(|batch| {
        // Called when all jobs complete
        println!("Batch {} completed", batch.id);
    })
    .catch(|batch, error| {
        // Called if any job fails
        println!("Batch {} failed: {}", batch.id, error);
    })
    .finally(|batch| {
        // Always called
        println!("Batch {} finished", batch.id);
    });

batch.dispatch().await?;

// Check batch progress
let progress = batch.progress().await?;
println!("Progress: {}/{}",  progress.processed, progress.total);
```

## Job Lifecycle

```
┌─────────────┐
│  Dispatched │
└──────┬──────┘
       │
       v
┌─────────────┐
│  In Queue   │ ◄──── Delayed jobs become available
└──────┬──────┘
       │
       v
┌─────────────┐
│  Processing │
└──────┬──────┘
       │
       ├──► Success ──► Complete
       │
       └──► Failure ──┬──► Retry (if attempts < max)
                      │
                      └──► Failed (Dead Letter Queue)
```

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum JobError {
    #[error("Job execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Job timeout after {0:?}")]
    Timeout(Duration),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Redis error: {0}")]
    RedisError(#[from] redis::RedisError),

    #[error("Custom error: {0}")]
    Custom(String),
}

// In job implementation
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // Transient error - will retry
        if random() < 0.3 {
            return Err(JobError::Custom("Network error".into()));
        }

        // Success
        Ok(())
    }

    async fn failed(&self, ctx: JobContext, error: JobError) {
        // Called after all retries exhausted
        tracing::error!(
            job_id = %ctx.job_id(),
            error = %error,
            "Job permanently failed"
        );

        // Send alert, log to monitoring, etc.
    }
}
```

## Configuration

```rust
// config/default.toml
[jobs]
redis_url = "redis://localhost:6379"
default_queue = "default"
max_attempts = 3
backoff_seconds = 60

[jobs.worker]
workers = 4
queues = ["default", "emails", "high-priority"]
timeout_seconds = 60
sleep_seconds = 1

[jobs.scheduler]
enabled = true
timezone = "UTC"
```

## Testing

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_job_dispatch() {
        let manager = QueueManager::new("redis://localhost:6379")
            .await
            .unwrap();

        let job = TestJob { value: 42 };
        let job_id = manager.dispatch(job).await.unwrap();

        assert!(!job_id.is_nil());
    }

    #[tokio::test]
    async fn test_job_execution() {
        let job = TestJob { value: 42 };
        let ctx = JobContext::new();

        let result = job.handle(ctx).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_retry_logic() {
        let manager = QueueManager::new("redis://localhost:6379")
            .await
            .unwrap();

        let job = FailingJob { attempts_to_fail: 2 };
        manager.dispatch(job).await.unwrap();

        // Start worker
        let pool = WorkerPool::new(
            WorkerConfig::default().workers(1),
            manager,
        ).await.unwrap();

        pool.start().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Job should succeed on 3rd attempt
        // Check job completed successfully
    }
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_job_lifecycle() {
    // 1. Setup Redis
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .unwrap();

    // 2. Dispatch job
    let job = SendEmailJob {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
        body: "Test body".to_string(),
    };
    let job_id = manager.dispatch(job).await.unwrap();

    // 3. Start worker
    let pool = WorkerPool::new(
        WorkerConfig::default().workers(1),
        manager,
    ).await.unwrap();
    pool.start().await.unwrap();

    // 4. Wait for processing
    tokio::time::sleep(Duration::from_secs(3)).await;

    // 5. Verify job completed
    // (Check logs, database, or job status)

    // 6. Shutdown
    pool.shutdown().await.unwrap();
}
```

## Performance Considerations

### 1. Queue Throughput

- **Target**: 1,000+ jobs/second per worker
- **Optimization**: Pipeline Redis commands
- **Monitoring**: Track job latency and throughput

### 2. Memory Usage

- **Job Payload Size**: Keep under 1KB
- **Worker Memory**: ~10MB per worker
- **Redis Memory**: Depends on queue depth

### 3. Concurrency

```rust
// Adjust workers based on workload
WorkerConfig::default()
    .workers(num_cpus::get() * 2)  // CPU-bound jobs
    .workers(100)                   // I/O-bound jobs
```

### 4. Batching

```rust
// Process multiple jobs in single Redis operation
let jobs: Vec<JobPayload> = manager
    .pop_batch("default", 10)  // Pop 10 jobs at once
    .await?;
```

## Security Considerations

### 1. Job Payload Validation

```rust
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // Validate email address
        if !is_valid_email(&self.to) {
            return Err(JobError::Custom("Invalid email".into()));
        }

        Ok(())
    }
}
```

### 2. Rate Limiting

```rust
// Limit job dispatch rate
Queue::dispatch(job)
    .rate_limit("send-email", 100, Duration::from_secs(60))  // 100/min
    .await?;
```

### 3. Job Encryption (for sensitive data)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedJob {
    #[serde(with = "encrypted")]
    sensitive_data: String,
}
```

## Monitoring & Observability

### 1. Metrics

```rust
// Job metrics
- jobs_dispatched_total (counter)
- jobs_processed_total (counter)
- jobs_failed_total (counter)
- job_duration_seconds (histogram)
- queue_depth (gauge)
- worker_utilization (gauge)
```

### 2. Logging

```rust
impl Job for MyJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log("Starting job processing");

        // ... work ...

        ctx.log("Job completed successfully");
        Ok(())
    }
}
```

### 3. Tracing

```rust
#[tracing::instrument(skip(self, ctx))]
async fn handle(&self, ctx: JobContext) -> JobResult {
    tracing::info!("Processing job");
    // Automatically includes span context
    Ok(())
}
```

## Advanced Features

### 1. Job Uniqueness

```rust
Queue::dispatch(job)
    .unique_for(Duration::from_secs(3600))  // Only one per hour
    .await?;
```

### 2. Job Priority

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

Queue::dispatch(job)
    .priority(Priority::High)
    .await?;
```

### 3. Job Dependencies

```rust
let job1 = ProcessImageJob { ... };
let job1_id = Queue::dispatch(job1).await?;

let job2 = GenerateThumbnailJob { ... };
Queue::dispatch(job2)
    .depends_on(job1_id)  // Wait for job1 to complete
    .await?;
```

### 4. Job Progress Tracking

```rust
impl Job for LongRunningJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        for i in 0..100 {
            // Update progress
            ctx.progress(i, 100).await?;

            // Do work
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(())
    }
}

// Monitor progress
let progress = queue.job_progress(job_id).await?;
println!("Progress: {}%", progress.percentage);
```

## Migration from Laravel

| Laravel | rf-jobs | Status |
|---------|---------|--------|
| `Job` trait | `Job` trait | ✅ Equivalent |
| `dispatch()` | `Queue::dispatch()` | ✅ Equivalent |
| `delay()` | `.delay()` | ✅ Equivalent |
| Queue workers | `WorkerPool` | ✅ Equivalent |
| Scheduled tasks | `Scheduler` | ✅ Equivalent |
| Job chaining | `Chain` | ✅ Equivalent |
| Job batching | `Batch` | ✅ Equivalent |
| Failed jobs | DLQ | ✅ Equivalent |
| Retry logic | Built-in | ✅ Equivalent |
| Rate limiting | `.rate_limit()` | ⏳ Future |
| Horizon (UI) | - | ⏳ Future |

## Example: Complete Job System

```rust
use rf_jobs::prelude::*;

// 1. Define jobs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendWelcomeEmail {
    user_id: Uuid,
    email: String,
}

#[async_trait]
impl Job for SendWelcomeEmail {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Sending welcome email to {}", self.email));

        // Send email
        // ...

        Ok(())
    }

    fn queue(&self) -> &str {
        "emails"
    }
}

// 2. Setup queue system
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize queue manager
    let manager = QueueManager::new("redis://localhost:6379").await?;

    // Dispatch jobs
    manager.dispatch(SendWelcomeEmail {
        user_id: Uuid::new_v4(),
        email: "user@example.com".to_string(),
    }).await?;

    // Start workers
    let pool = WorkerPool::new(
        WorkerConfig::default()
            .workers(4)
            .queues(&["default", "emails"]),
        manager.clone(),
    ).await?;

    pool.start().await?;

    // Start scheduler
    let mut scheduler = Scheduler::new(manager);
    scheduler
        .schedule("0 0 * * *")
        .job(DailyReportJob);
    scheduler.start().await?;

    // Keep running
    tokio::signal::ctrl_c().await?;

    // Graceful shutdown
    pool.shutdown().await?;

    Ok(())
}
```

## Summary

**rf-jobs** provides a complete background job processing system:

- ✅ **Job Queue**: Redis-backed async queue
- ✅ **Worker Pool**: Concurrent job processing
- ✅ **Scheduler**: Cron-like scheduled jobs
- ✅ **Retry Logic**: Exponential backoff
- ✅ **Failed Jobs**: Dead Letter Queue
- ✅ **Job Chaining**: Sequential job execution
- ✅ **Job Batching**: Process multiple jobs together
- ✅ **Monitoring**: Comprehensive metrics and logging
- ✅ **Type-Safe**: Compile-time job validation

**Next**: Implementation of rf-jobs crate
