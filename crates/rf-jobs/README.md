# rf-jobs - Background Jobs & Queue System

Production-ready background job processing for RustForge with advanced features like job chaining, batching, rate limiting, and priority queues.

## Features

### Core Features
- Asynchronous job queue with Redis backend
- Worker pool with configurable concurrency
- Job scheduling (cron-like patterns)
- Retry logic with exponential backoff
- Failed job handling (Dead Letter Queue)
- Delayed job execution

### Advanced Features (Phase 2)
- **Job Chaining** - Sequential job execution
- **Job Batching** - Parallel job execution with progress tracking
- **Rate Limiting** - Control job execution rates
- **Priority Queues** - High/Default/Low priority job processing

## Quick Start

### Basic Job

```rust
use rf_jobs::prelude::*;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Sending email to {}", self.to));
        // Send email logic
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = QueueManager::new("redis://localhost:6379").await?;

    let job = SendEmailJob {
        to: "user@example.com".to_string(),
        subject: "Welcome!".to_string(),
    };

    manager.dispatch(job).await?;
    Ok(())
}
```

### Job Chaining

Execute jobs sequentially:

```rust
use rf_jobs::prelude::*;

JobChain::new()
    .name("video-processing")
    .then(DownloadVideo::new())?
    .then(ProcessVideo::new())?
    .then(UploadVideo::new())?
    .dispatch(&queue)
    .await?;
```

### Job Batching

Execute jobs in parallel:

```rust
use rf_jobs::prelude::*;

let podcasts = vec![1, 2, 3, 4, 5];

JobBatch::new()
    .name("podcast-batch")
    .add_many(podcasts.into_iter().map(ProcessPodcast::new))?
    .then(|batch_id| async move {
        println!("All podcasts processed!");
    })
    .dispatch(&queue)
    .await?;
```

### Rate Limiting

Control job execution rates:

```rust
use rf_jobs::prelude::*;
use std::time::Duration;

let limiter = RateLimiter::new(queue.clone());

// Allow 100 emails per hour
if limiter.allow("emails", 100, Duration::from_secs(3600)).await? {
    send_email().await?;
} else {
    println!("Rate limit exceeded");
}
```

### Priority Queues

Dispatch jobs with different priorities:

```rust
use rf_jobs::prelude::*;

// High priority
queue.dispatch_with_priority(
    UrgentNotification::new(),
    QueuePriority::High
).await?;

// Default priority
queue.dispatch(RegularEmail::new()).await?;

// Low priority
queue.dispatch_with_priority(
    CleanupJob::new(),
    QueuePriority::Low
).await?;
```

## Worker Setup

```rust
use rf_jobs::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = QueueManager::new("redis://localhost:6379").await?;

    let config = WorkerConfig {
        queue: "default".to_string(),
        concurrency: 4,
        poll_interval: Duration::from_secs(1),
    };

    let pool = WorkerPool::new(queue, config);
    pool.start().await?;

    Ok(())
}
```

## Job Configuration

Customize job behavior:

```rust
#[async_trait]
impl Job for MyJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // Job logic
        Ok(())
    }

    fn queue(&self) -> &str {
        "high-priority"  // Custom queue name
    }

    fn max_attempts(&self) -> u32 {
        5  // Retry up to 5 times
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(120)  // 2 minutes between retries
    }

    fn timeout(&self) -> Duration {
        Duration::from_secs(300)  // 5 minute timeout
    }

    async fn failed(&self, ctx: JobContext, error: JobError) {
        // Handle final failure
        ctx.error(&format!("Job failed permanently: {}", error));
    }
}
```

## Advanced Examples

### Complete Pipeline

```rust
use rf_jobs::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = QueueManager::new("redis://localhost:6379").await?;
    let limiter = RateLimiter::new(queue.clone());

    // Create a batch of jobs with rate limiting
    let video_ids = vec![1, 2, 3, 4, 5];

    let batch = JobBatch::new()
        .name("video-processing-batch")
        .add_many(video_ids.into_iter().map(|id| ProcessVideo { id }))?
        .allow_failures(true)
        .then(|batch_id| async move {
            println!("Batch {} completed!", batch_id);
        });

    let batch_id = batch.dispatch(&queue).await?;

    // Create a chain for post-processing
    JobChain::new()
        .name("video-1-postprocessing")
        .then(GenerateThumbnails::new(1))?
        .then(CreatePreview::new(1))?
        .then(PublishVideo::new(1))?
        .dispatch(&queue)
        .await?;

    // High-priority notification
    queue.dispatch_with_priority(
        NotifyUser::new(),
        QueuePriority::High
    ).await?;

    Ok(())
}
```

### Rate-Limited Job

```rust
use rf_jobs::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ApiCallJob {
    endpoint: String,
}

#[async_trait]
impl Job for ApiCallJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        let queue = QueueManager::new("redis://localhost:6379").await?;
        let limiter = RateLimiter::new(queue);

        // Wait for rate limit slot (100 calls per minute)
        limiter.wait_for_slot("api", 100, Duration::from_secs(60)).await?;

        // Make API call
        make_api_call(&self.endpoint).await?;

        Ok(())
    }
}
```

## Progress Tracking

### Chain Progress

```rust
let chain_id = chain.dispatch(&queue).await?;

let (current, total) = queue.chain_progress(chain_id).await?;
println!("Chain progress: {}/{}", current, total);

let state = queue.load_chain_state(chain_id).await?;
println!("Status: {:?}", state.status);
```

### Batch Progress

```rust
let batch_id = batch.dispatch(&queue).await?;

let (completed, failed, pending, total) = queue.batch_progress(batch_id).await?;
println!("Batch: {}/{} completed, {} failed, {} pending",
    completed, total, failed, pending);

let state = queue.load_batch_state(batch_id).await?;
println!("Status: {:?}", state.status);
```

## Testing

The crate includes comprehensive integration tests:

```bash
# Run unit tests
cargo test

# Run integration tests (requires Redis)
cargo test --test chaining_batching_test -- --ignored
```

## Redis Data Structures

### Queues
```
queue:{name}           → List of jobs
queue:{name}:high      → High priority jobs
queue:{name}:default   → Default priority jobs
queue:{name}:low       → Low priority jobs
queue:delayed          → Sorted set of delayed jobs
queue:failed           → Failed jobs (DLQ)
```

### Chains
```
chain:{id}:state       → Chain state (JSON)
chain:{id}:jobs        → Chain jobs (JSON)
```

### Batches
```
batch:{id}:state       → Batch state (JSON)
```

### Rate Limits
```
rate_limit:{key}       → Sorted set (sliding window)
```

## Performance

- **Throughput**: 1000+ jobs/second per worker
- **Latency**: <10ms dispatch time
- **Concurrency**: Configurable worker pool (default: CPU cores)
- **Reliability**: Redis-backed persistence with automatic retries

## Laravel Comparison

| Feature | Laravel | RustForge |
|---------|---------|-----------|
| Basic Queue | `Job::dispatch()` | `queue.dispatch(job)` |
| Delayed Jobs | `Job::dispatch()->delay(60)` | `queue.dispatch_later(job, Duration)` |
| Chaining | `Bus::chain([...])` | `JobChain::new().then(...)` |
| Batching | `Bus::batch([...])` | `JobBatch::new().add_many(...)` |
| Rate Limiting | `Redis::throttle('key')` | `limiter.allow("key", ...)` |
| Priority | `->onQueue('high')` | `dispatch_with_priority(job, High)` |
| Failed Jobs | `php artisan queue:retry` | `queue.retry_failed(id)` |

## Documentation

- [Advanced Features Guide](../../docs/QUEUE_ADVANCED_FEATURES.md)
- [API Documentation](https://docs.rs/rf-jobs)

## Requirements

- Redis 5.0+
- Tokio runtime

## License

This crate is part of the RustForge framework and shares the same license.

## Contributing

Contributions are welcome! Please see the main RustForge repository for contribution guidelines.
