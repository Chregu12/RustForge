# Migration Guide: rf-queue → rf-jobs

This guide helps you migrate from the deprecated `rf-queue` to `rf-jobs`.

## Why Migrate?

The `rf-queue` crate has critical bugs that have been fixed in `rf-jobs`:

### Critical Issues Fixed in rf-jobs

1. **Jobs Actually Execute** ❌→✅
   - **Before**: Worker only logged "Would execute job..." instead of running it
   - **After**: Jobs are properly deserialized and executed via registry

2. **Retry Preserves Payload** ❌→✅
   - **Before**: Retry dispatched `DummyJob`, losing original data
   - **After**: Original payload preserved with all metadata

3. **Type-Safe Execution** ❌→✅
   - **Before**: No job registry, no dynamic dispatch
   - **After**: Registry maps job types to handlers

## Migration Steps

### Step 1: Update Dependencies

**Before:**
```toml
[dependencies]
rf-queue = { version = "0.2.0", features = ["redis-backend"] }
```

**After:**
```toml
[dependencies]
rf-jobs = "0.1"
```

### Step 2: Update Job Trait

The job trait has changed to support the registry system.

**Before (rf-queue):**
```rust
use rf_queue::{Job, QueueError};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // Send email
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "send_email"
    }

    fn max_retries(&self) -> u32 {
        3
    }
}
```

**After (rf-jobs):**
```rust
use rf_jobs::prelude::*;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
}

#[async_trait]
impl JobWithRegistry for SendEmailJob {
    fn job_type(&self) -> &'static str {
        "send_email"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        // Use context for logging
        ctx.log(&format!("Sending email to {}", self.to));

        // Send email
        Ok(())
    }

    fn max_attempts(&self) -> u32 {
        3
    }

    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Exponential
    }
}
```

**Key Changes:**
- Trait name: `Job` → `JobWithRegistry`
- Handler signature: `handle(&self)` → `handle(&self, ctx: JobContext)`
- Return type: `Result<(), QueueError>` → `JobResult`
- Method name: `max_retries()` → `max_attempts()`
- Add `Clone` derive (required)
- New: `backoff_strategy()` method

### Step 3: Create Job Registry

**Before (rf-queue):**
```rust
// No registry needed
let queue = MemoryQueue::new();
```

**After (rf-jobs):**
```rust
// Create and register jobs
let mut registry = JobRegistry::new();
registry.register::<SendEmailJob>("send_email");
registry.register::<ProcessImageJob>("process_image");
// ... register all your job types
```

### Step 4: Update Queue Creation

**Before (rf-queue):**
```rust
use rf_queue::{RedisQueue, QueueConfig};

let queue = RedisQueue::new("redis://localhost:6379", "myapp").await?;

// Or with config
let queue = QueueConfig::redis("redis://localhost:6379", "myapp")
    .build()
    .await?;
```

**After (rf-jobs):**
```rust
use rf_jobs::QueueManager;

let manager = QueueManager::new("redis://localhost:6379").await?;
```

**Key Changes:**
- Type: `RedisQueue` → `QueueManager`
- No prefix parameter (can use queue names instead)
- Simpler API

### Step 5: Update Job Dispatching

**Before (rf-queue):**
```rust
use rf_queue::{JobMetadata, Queue};

let job = SendEmailJob { /* ... */ };
let metadata = JobMetadata::new(&job)?;
queue.push(metadata).await?;
```

**After (rf-jobs):**
```rust
let job = SendEmailJob { /* ... */ };
let job_id = manager.dispatch(job).await?;
```

**Key Changes:**
- No manual `JobMetadata` creation
- Direct `dispatch()` method
- Returns job ID

### Step 6: Update Worker Pool

**Before (rf-queue):**
```rust
use rf_queue::Worker;

let worker = Worker::new(queue.clone())
    .concurrency(5)
    .handle(|job: SendEmailJob| Box::pin(async move {
        job.handle().await
    }));

worker.start().await?;
```

**After (rf-jobs):**
```rust
use rf_jobs::{WorkerConfig, WorkerPool};

let config = WorkerConfig::default()
    .workers(5)
    .queues(&["default"]);

let mut pool = WorkerPool::new(config, manager, registry).await?;
pool.start().await?;

// Graceful shutdown
pool.shutdown().await?;
```

**Key Changes:**
- Type: `Worker` → `WorkerPool`
- Configuration via `WorkerConfig`
- Requires registry
- Explicit shutdown method

### Step 7: Update Delayed Jobs

**Before (rf-queue):**
```rust
use std::time::Duration;

let metadata = JobMetadata::new_delayed(&job, Duration::from_secs(300))?;
queue.push(metadata).await?;
```

**After (rf-jobs):**
```rust
use std::time::Duration;

let job_id = manager.dispatch_later(job, Duration::from_secs(300)).await?;
```

### Step 8: Update Error Handling

**Before (rf-queue):**
```rust
use rf_queue::QueueError;

fn my_handler(&self) -> Result<(), QueueError> {
    Err(QueueError::JobFailed("error".to_string()))
}
```

**After (rf-jobs):**
```rust
use rf_jobs::{JobResult, JobError};

async fn handle(&self, ctx: JobContext) -> JobResult {
    Err(JobError::ExecutionFailed("error".to_string()))
}
```

## Complete Example

Here's a complete before/after comparison:

### Before (rf-queue)

```rust
use rf_queue::{Job, QueueError, RedisQueue, Worker, JobMetadata, Queue};
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        println!("Sending to {}", self.to);
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "send_email"
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = Arc::new(RedisQueue::new("redis://localhost:6379", "app").await?);

    // Dispatch
    let job = SendEmailJob { to: "user@example.com".to_string() };
    let metadata = JobMetadata::new(&job)?;
    queue.push(metadata).await?;

    // Worker
    let worker = Worker::new(Arc::clone(&queue))
        .concurrency(5)
        .handle(|job: SendEmailJob| Box::pin(async move {
            job.handle().await
        }));

    worker.start().await?;
    Ok(())
}
```

### After (rf-jobs)

```rust
use rf_jobs::prelude::*;
use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
}

#[async_trait]
impl JobWithRegistry for SendEmailJob {
    fn job_type(&self) -> &'static str {
        "send_email"
    }

    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Sending to {}", self.to));
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Registry
    let mut registry = JobRegistry::new();
    registry.register::<SendEmailJob>("send_email");

    // Queue
    let manager = QueueManager::new("redis://localhost:6379").await?;

    // Dispatch
    let job = SendEmailJob { to: "user@example.com".to_string() };
    manager.dispatch(job).await?;

    // Worker
    let config = WorkerConfig::default()
        .workers(5)
        .queues(&["default"]);

    let mut pool = WorkerPool::new(config, manager, registry).await?;
    pool.start().await?;

    // Graceful shutdown
    tokio::time::sleep(Duration::from_secs(10)).await;
    pool.shutdown().await?;

    Ok(())
}
```

## Feature Mapping

| rf-queue Feature | rf-jobs Equivalent |
|------------------|-------------------|
| `Job` trait | `JobWithRegistry` trait |
| `QueueError` | `JobError` |
| `JobMetadata` | Handled internally |
| `RedisQueue` | `QueueManager` |
| `MemoryQueue` | Not needed (use Redis) |
| `Worker` | `WorkerPool` |
| `QueueConfig` | Direct constructor |
| `max_retries()` | `max_attempts()` |
| `handle()` | `handle(ctx)` |
| Manual retry | Automatic with backoff |

## New Features in rf-jobs

Take advantage of these new features:

### 1. Job Context

```rust
async fn handle(&self, ctx: JobContext) -> JobResult {
    ctx.log("Starting job");

    // Access job metadata
    println!("Job ID: {}", ctx.job_id);
    println!("Attempt: {}/{}", ctx.attempt, ctx.max_attempts);
    println!("Queue: {}", ctx.queue);

    Ok(())
}
```

### 2. Backoff Strategies

```rust
fn backoff_strategy(&self) -> BackoffStrategy {
    BackoffStrategy::Exponential  // Default
    // BackoffStrategy::Linear
    // BackoffStrategy::Fixed
}
```

### 3. Failed Job Callback

```rust
async fn failed(&self, ctx: JobContext, error: &JobError) {
    ctx.log(&format!("Job permanently failed: {}", error));
    // Send notification, log to database, etc.
}
```

### 4. Job Batching

```rust
let batch = JobBatch::new("email_campaign")
    .add(SendEmailJob { to: "user1@example.com".to_string() })
    .add(SendEmailJob { to: "user2@example.com".to_string() });

manager.dispatch_batch(batch).await?;
```

### 5. Job Chaining

```rust
let chain = JobChain::new()
    .then(ProcessImageJob { url: "image.jpg".to_string() })
    .then(GenerateThumbnailJob { size: "128x128".to_string() })
    .then(UploadToS3Job { bucket: "images".to_string() });

manager.dispatch_chain(chain).await?;
```

## Troubleshooting

### Jobs Not Executing

**Problem**: Jobs are queued but not executing.

**Solution**: Make sure you registered the job type:
```rust
registry.register::<YourJob>("your_job_type");
```

### Serialization Errors

**Problem**: `SerializationError` when dispatching jobs.

**Solution**: Ensure job struct derives `Serialize` and `Deserialize`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct YourJob { /* ... */ }
```

### Type Name Mismatch

**Problem**: `Unknown job type` error.

**Solution**: The `job_type()` string must match the registration:
```rust
fn job_type(&self) -> &'static str {
    "your_job"  // Must match registry.register::<YourJob>("your_job")
}
```

## Need Help?

- Check [rf-jobs examples](../rf-jobs/examples/)
- Read [rf-jobs documentation](../rf-jobs/README.md)
- Open an issue on GitHub

## Migration Checklist

- [ ] Update `Cargo.toml` dependencies
- [ ] Change `Job` trait to `JobWithRegistry`
- [ ] Update `handle()` signature to accept `JobContext`
- [ ] Create `JobRegistry` and register all job types
- [ ] Replace `RedisQueue` with `QueueManager`
- [ ] Update job dispatching (remove `JobMetadata`)
- [ ] Update worker pool creation
- [ ] Test all job types execute correctly
- [ ] Test retry logic works
- [ ] Test delayed jobs work
- [ ] Remove `rf-queue` dependency

---

**Important**: Once migrated, thoroughly test your job processing to ensure all jobs execute correctly. The new system actually runs jobs (the old one just logged!).
