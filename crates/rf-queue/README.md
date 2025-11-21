# rf-queue: Background Job Processing for RustForge

# ⚠️ **DEPRECATED - Use `rf-jobs` Instead**

**This crate is deprecated and will be removed in a future release.**

Please migrate to [`rf-jobs`](../rf-jobs/README.md) which provides:
- **Job Registry System**: Type-safe job execution with dynamic dispatch
- **Fixed Critical Bugs**: Jobs actually execute (not just log), retry preserves payloads
- **Enhanced Features**: Batching, chaining, rate limiting, scheduling
- **Better APIs**: More Laravel-like, better documented

See [MIGRATION.md](MIGRATION.md) for migration instructions.

---

## Legacy Documentation

Production-ready background job processing with multiple backends.

## Features

- **Type-Safe Jobs**: Define jobs with the `Job` trait
- **Multiple Backends**: Memory (development) and Redis (production)
- **Job Retries**: Automatic retry with exponential backoff
- **Delayed Jobs**: Schedule jobs for future execution
- **Distributed**: Redis backend supports multiple workers
- **Persistent**: Jobs survive application restarts (Redis)
- **Worker Pool**: Concurrent job processing
- **Priority Queues**: Job prioritization support

## Installation

```toml
[dependencies]
rf-queue = "0.2.0"

# For Redis backend (production)
rf-queue = { version = "0.2.0", features = ["redis-backend"] }
```

## Quick Start

### Define a Job

```rust
use rf_queue::{Job, QueueError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
    body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // Send email logic
        println!("Sending email to {}", self.to);
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "send_email"
    }

    fn max_retries(&self) -> u32 {
        3 // Optional: customize retry attempts
    }
}
```

### Memory Backend (Development)

```rust
use rf_queue::{MemoryQueue, Queue, JobMetadata};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create queue
    let queue = Arc::new(MemoryQueue::new());

    // Dispatch job
    let job = SendEmailJob {
        to: "user@example.com".to_string(),
        subject: "Welcome".to_string(),
        body: "Welcome to our platform!".to_string(),
    };

    let metadata = JobMetadata::new(&job)?;
    queue.push(metadata).await?;

    println!("Job dispatched!");
    Ok(())
}
```

### Redis Backend (Production)

```rust
use rf_queue::{RedisQueue, Queue, JobMetadata, QueueConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create Redis queue
    let queue = RedisQueue::new("redis://localhost:6379", "myapp").await?;

    // Or use config builder
    let queue = QueueConfig::redis("redis://localhost:6379", "myapp")
        .build()
        .await?;

    // Dispatch job
    let job = SendEmailJob {
        to: "user@example.com".to_string(),
        subject: "Welcome".to_string(),
        body: "Welcome to our platform!".to_string(),
    };

    let metadata = JobMetadata::new(&job)?;
    queue.push(metadata).await?;

    println!("Job dispatched to Redis!");
    Ok(())
}
```

## Configuration

### Using Config Builder

```rust
use rf_queue::QueueConfigBuilder;

let queue = QueueConfigBuilder::new()
    .backend("redis")
    .redis_url("redis://localhost:6379")
    .prefix("myapp")
    .build()
    .await?;
```

### From Environment Variables

```rust
use rf_queue::QueueConfig;

// Reads from REDIS_URL and QUEUE_PREFIX env vars
let queue = QueueConfig::redis_from_env().build().await?;
```

Environment variables:
- `REDIS_URL`: Redis connection URL (default: "redis://localhost:6379")
- `QUEUE_PREFIX`: Queue prefix for namespacing (default: "queue")

## Advanced Features

### Delayed Jobs

Schedule jobs for future execution:

```rust
use std::time::Duration;

let job = SendEmailJob { /* ... */ };

// Execute after 5 minutes
let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(300))?;
queue.push(metadata).await?;
```

### Job Retries

Jobs automatically retry on failure with exponential backoff:

```rust
#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // If this fails, job will be retried
        send_email(&self.to, &self.subject, &self.body).await?;
        Ok(())
    }

    fn max_retries(&self) -> u32 {
        5 // Retry up to 5 times
    }
}
```

Retry delays (exponential backoff):
- Attempt 1: 1 minute
- Attempt 2: 2 minutes
- Attempt 3: 4 minutes
- Attempt 4: 8 minutes
- Attempt 5: 16 minutes

### Worker Pool

Process jobs with multiple workers:

```rust
use rf_queue::Worker;

let worker = Worker::new(queue.clone())
    .concurrency(5) // 5 concurrent workers
    .handle(|job: SendEmailJob| Box::pin(async move {
        job.handle().await
    }));

// Start processing
worker.start().await?;
```

### Multiple Queues

Organize jobs into different queues:

```rust
#[async_trait]
impl Job for SendEmailJob {
    fn queue(&self) -> &str {
        "emails" // Use "emails" queue instead of "default"
    }

    // ...
}

// Reserve from specific queue
let job = queue.reserve("emails").await?;
```

### Priority Jobs

Higher priority jobs are processed first:

```rust
#[async_trait]
impl Job for UrgentNotification {
    fn priority(&self) -> i32 {
        100 // Higher priority (default is 0)
    }

    // ...
}
```

## Redis Backend Features

### Persistence

Jobs are stored in Redis and survive application restarts:

```rust
// Push job
queue.push(metadata).await?;

// Application restarts...

// Job is still there
let job = queue.reserve("default").await?;
```

### Distributed Processing

Multiple workers can process from the same queue:

```rust
// Worker 1
let queue = RedisQueue::new("redis://localhost:6379", "myapp").await?;
let worker = Worker::new(queue).concurrency(5);

// Worker 2 (different process/machine)
let queue = RedisQueue::new("redis://localhost:6379", "myapp").await?;
let worker = Worker::new(queue).concurrency(5);

// Both workers process from same queue
```

### Failed Jobs

Failed jobs are tracked in Redis:

```rust
// Job fails and exceeds retries
queue.fail(&job_id, "Error message").await?;

// Failed jobs stored in: queue:{prefix}:failed:{queue_name}
```

## Performance

Redis backend performance characteristics:

- **Throughput**: 10,000+ jobs/sec
- **Latency**: ~1ms per operation
- **Persistence**: All jobs persisted to Redis
- **Distributed**: Supports multiple workers

Memory backend performance (development):

- **Throughput**: 50,000+ jobs/sec
- **Latency**: ~0.01ms per operation
- **Persistence**: None (in-memory only)
- **Distributed**: No (single process only)

## Examples

### Email Sending Service

```rust
use rf_queue::{Job, JobMetadata, QueueConfig};

#[derive(Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
    body: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // Use your email service
        let mailer = get_mailer();
        mailer.send(&self.to, &self.subject, &self.body).await?;
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "send_email"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = QueueConfig::redis_from_env().build().await?;

    // Dispatch welcome email
    let job = SendEmailJob {
        to: "user@example.com".to_string(),
        subject: "Welcome!".to_string(),
        body: "Thanks for signing up!".to_string(),
    };

    let metadata = JobMetadata::new(&job)?;
    queue.push(metadata).await?;

    Ok(())
}
```

### Image Processing Pipeline

```rust
#[derive(Serialize, Deserialize)]
struct ProcessImageJob {
    image_url: String,
    user_id: u64,
}

#[async_trait]
impl Job for ProcessImageJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // Download image
        let image = download_image(&self.image_url).await?;

        // Process (resize, optimize, etc.)
        let processed = process_image(image)?;

        // Upload to storage
        let final_url = upload_image(processed).await?;

        // Update database
        update_user_avatar(self.user_id, final_url).await?;

        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "process_image"
    }

    fn queue(&self) -> &str {
        "images" // Dedicated queue for image processing
    }

    fn max_retries(&self) -> u32 {
        5
    }
}
```

## Testing

### Unit Tests

```rust
#[tokio::test]
async fn test_job_processing() {
    let queue = MemoryQueue::new();

    let job = SendEmailJob { /* ... */ };
    let metadata = JobMetadata::new(&job).unwrap();
    queue.push(metadata).await.unwrap();

    let reserved = queue.reserve("default").await.unwrap();
    assert!(reserved.is_some());
}
```

### Integration Tests (Redis)

```rust
#[tokio::test]
#[ignore] // Requires Redis
async fn test_redis_persistence() {
    let queue = RedisQueue::new("redis://localhost:6379", "test").await.unwrap();

    // Push job
    let job = SendEmailJob { /* ... */ };
    let metadata = JobMetadata::new(&job).unwrap();
    queue.push(metadata).await.unwrap();

    // Simulate restart
    drop(queue);
    let queue = RedisQueue::new("redis://localhost:6379", "test").await.unwrap();

    // Job should still be there
    let reserved = queue.reserve("default").await.unwrap();
    assert!(reserved.is_some());
}
```

Run Redis tests:
```bash
# Start Redis
docker run -d -p 6379:6379 redis:7

# Run tests
cargo test --features redis-backend -- --ignored
```

## Comparison: Memory vs Redis

| Feature | Memory Backend | Redis Backend |
|---------|---------------|---------------|
| **Use Case** | Development, Testing | Production |
| **Persistence** | No (lost on restart) | Yes (survives restart) |
| **Distributed** | No (single process) | Yes (multiple workers) |
| **Performance** | Very High (50k+ ops/sec) | High (10k+ ops/sec) |
| **Setup** | None | Requires Redis |
| **Configuration** | `MemoryQueue::new()` | `RedisQueue::new(url, prefix)` |

## Best Practices

### 1. Use Redis in Production

Always use Redis backend in production for persistence and distributed processing:

```rust
let queue = if cfg!(debug_assertions) {
    QueueConfig::memory().build().await?
} else {
    QueueConfig::redis_from_env().build().await?
};
```

### 2. Handle Errors Gracefully

Always handle errors in job handlers:

```rust
#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        match send_email(&self.to, &self.subject).await {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::error!("Failed to send email: {}", e);
                Err(QueueError::JobFailed(e.to_string()))
            }
        }
    }
}
```

### 3. Use Appropriate Retry Limits

Set retry limits based on job type:

```rust
// Critical jobs: more retries
fn max_retries(&self) -> u32 { 5 }

// Non-critical jobs: fewer retries
fn max_retries(&self) -> u32 { 1 }
```

### 4. Monitor Queue Size

Monitor queue size to detect issues:

```rust
let size = queue.size("default").await?;
if size > 10000 {
    tracing::warn!("Queue backlog: {} jobs", size);
}
```

### 5. Use Multiple Queues

Separate different job types into different queues:

```rust
// High priority
fn queue(&self) -> &str { "critical" }

// Normal priority
fn queue(&self) -> &str { "default" }

// Low priority
fn queue(&self) -> &str { "background" }
```

## License

MIT OR Apache-2.0
