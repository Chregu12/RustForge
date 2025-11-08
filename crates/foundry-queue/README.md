# Foundry Queue

Production-ready job queue system with multiple backend support (Redis, In-Memory).

## Features

- **Multiple Backends**: Redis for production, In-Memory for development
- **Type-Safe API**: Generic methods for serializable job payloads
- **Delayed Jobs**: Schedule jobs for future execution
- **Job Retry**: Automatic retry with configurable attempts
- **Worker Process**: Background job processing
- **Job Priority**: Support for job prioritization
- **Connection Pooling**: Efficient Redis connection management
- **Error Handling**: Comprehensive error types and retry logic

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
foundry-queue = { path = "../foundry-queue" }
```

## Quick Start

### Basic Usage

```rust
use foundry_queue::prelude::*;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), QueueError> {
    // Create queue manager from environment
    let queue = QueueManager::from_env()?;

    // Dispatch a job
    let job = Job::new("send_email")
        .with_payload(json!({
            "to": "user@example.com",
            "subject": "Welcome!",
            "body": "Thanks for signing up!"
        }));

    let job_id = queue.dispatch(job).await?;
    println!("Job dispatched: {}", job_id);

    Ok(())
}
```

### Worker Process

```rust
use foundry_queue::prelude::*;
use foundry_queue::worker::handlers::EchoHandler;

#[tokio::main]
async fn main() -> Result<(), QueueError> {
    let queue = QueueManager::from_env()?;

    // Create worker
    let mut worker = Worker::new(queue);

    // Register job handlers
    worker.register_handler("send_email", EchoHandler);

    // Run worker
    let stats = worker.run().await?;

    println!("Processed: {}, Failed: {}", stats.processed, stats.failed);

    Ok(())
}
```

## Configuration

### Environment Variables

```bash
# Queue driver (memory or redis)
QUEUE_DRIVER=redis

# Redis connection
REDIS_URL=redis://127.0.0.1:6379

# Queue prefix (optional)
QUEUE_PREFIX=queue:

# Default queue name (optional)
QUEUE_DEFAULT=default

# Worker timeout in seconds (optional)
QUEUE_TIMEOUT=300
```

### Programmatic Configuration

```rust
use foundry_queue::*;

// In-memory queue (development)
let config = QueueConfig::memory();
let queue = QueueManager::from_config(config)?;

// Redis queue (production)
let config = QueueConfig::redis("redis://127.0.0.1:6379");
let queue = QueueManager::from_config(config)?;
```

## Advanced Features

### Delayed Jobs

```rust
use std::time::Duration;

let job = Job::new("cleanup")
    .with_payload(json!({"type": "old_files"}))
    .with_delay(Duration::from_secs(3600)); // Execute in 1 hour

queue.dispatch(job).await?;
```

### Job Priority

```rust
let high_priority = Job::new("urgent_task")
    .with_priority(10);

let low_priority = Job::new("background_task")
    .with_priority(1);

queue.dispatch(high_priority).await?;
queue.dispatch(low_priority).await?;
```

### Custom Retry Logic

```rust
let job = Job::new("api_call")
    .with_max_attempts(5)
    .with_timeout(Duration::from_secs(30));

queue.dispatch(job).await?;
```

### Multiple Queues

```rust
// Dispatch to different queues
let email_job = Job::new("send_email")
    .on_queue("emails");

let report_job = Job::new("generate_report")
    .on_queue("reports");

queue.dispatch(email_job).await?;
queue.dispatch(report_job).await?;

// Worker processing multiple queues
let worker_config = WorkerConfig {
    queues: vec!["emails".to_string(), "reports".to_string()],
    ..Default::default()
};

let worker = Worker::with_config(queue, worker_config);
```

### Custom Job Handlers

```rust
use async_trait::async_trait;
use foundry_queue::worker::{JobHandler, JobHandlerRegistry};

struct EmailHandler {
    smtp_host: String,
}

#[async_trait]
impl JobHandler for EmailHandler {
    async fn handle(&self, job: &Job) -> QueueResult<Option<Value>> {
        // Extract email details from job payload
        let to = job.payload["to"].as_str().unwrap();
        let subject = job.payload["subject"].as_str().unwrap();

        // Send email logic here
        println!("Sending email to {} with subject: {}", to, subject);

        Ok(Some(json!({"sent": true})))
    }
}

// Register handler
let mut worker = Worker::new(queue);
worker.register_handler("send_email", EmailHandler {
    smtp_host: "smtp.example.com".to_string(),
});
```

## Architecture

### Backend Trait

All backends implement the `QueueBackend` trait:

```rust
#[async_trait]
pub trait QueueBackend: Send + Sync {
    async fn push(&self, job: Job) -> QueueResult<()>;
    async fn pop(&self) -> QueueResult<Option<Job>>;
    async fn size(&self) -> QueueResult<usize>;
    async fn clear(&self) -> QueueResult<()>;
    // ... more methods
}
```

### Available Backends

1. **MemoryBackend**: In-memory storage for development/testing
2. **RedisBackend**: Production-ready Redis backend with connection pooling

## Redis Backend Details

### Features

- **Connection Pooling**: Uses `deadpool-redis` for efficient connection management
- **Atomic Operations**: Uses Redis lists and sorted sets for reliable job storage
- **Priority Queues**: Sorted sets for priority-based job processing
- **Delayed Jobs**: Sorted sets with timestamp scores for delayed execution
- **Failed Job Tracking**: Separate set for failed jobs

### Redis Data Structures

- `queue:queues:{name}` - List of pending jobs (FIFO)
- `queue:queues:{name}:priority` - Sorted set for priority jobs
- `queue:jobs:{id}` - Hash of job data
- `queue:delayed` - Sorted set of delayed jobs
- `queue:failed` - Set of failed job IDs

## Integration with Foundry

### Using with foundry-infra

```rust
use foundry_infra::RedisQueue;
use foundry_plugins::QueuePort;

// Create Redis queue adapter
let queue: Arc<dyn QueuePort> = Arc::new(RedisQueue::from_env()?);

// Use in CommandContext
let ctx = CommandContext {
    queue,
    // ... other fields
};
```

## Testing

### Unit Tests

```bash
cargo test -p foundry-queue
```

### Integration Tests (requires Redis)

```bash
# Start Redis
docker run -d -p 6379:6379 redis:latest

# Run tests
cargo test -p foundry-queue --all-features -- --include-ignored
```

## Performance

### Benchmarks

The Redis backend has been optimized for:
- **Throughput**: 1000+ jobs/second dispatch rate
- **Latency**: < 5ms average job dispatch time
- **Connection Pooling**: Reuses connections efficiently
- **Memory**: Minimal overhead with streaming processing

### Production Recommendations

1. **Use Redis** for production workloads
2. **Configure connection pool** size based on concurrency needs
3. **Monitor queue depth** to prevent backlog
4. **Set appropriate timeouts** for long-running jobs
5. **Use multiple workers** for high-throughput scenarios

## Migration Guide

### From In-Memory to Redis

1. Update configuration:
```bash
# Before
QUEUE_DRIVER=memory

# After
QUEUE_DRIVER=redis
REDIS_URL=redis://your-redis-host:6379
```

2. No code changes required - the `QueueManager` handles backend selection automatically.

3. Start Redis instance:
```bash
docker run -d -p 6379:6379 redis:latest
```

## Error Handling

The queue system provides detailed error types:

```rust
pub enum QueueError {
    NotFound(String),
    Serialization(String),
    Deserialization(String),
    Connection(String),
    Redis(String),
    InvalidJob(String),
    QueueFull(String),
    Worker(String),
    Timeout(String),
    Config(String),
    Other(String),
}
```

## Examples

See the `examples/` directory for:
- Basic queue usage
- Worker implementation
- Custom job handlers
- Priority queues
- Delayed jobs

## License

MIT OR Apache-2.0
