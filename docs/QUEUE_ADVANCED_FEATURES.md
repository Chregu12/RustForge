# Queue Advanced Features

This guide covers the advanced features of the RustForge Queue system, including Job Chaining, Batching, Rate Limiting, and Priority Queues.

## Table of Contents

- [Job Chaining](#job-chaining)
- [Job Batching](#job-batching)
- [Rate Limiting](#rate-limiting)
- [Priority Queues](#priority-queues)
- [Best Practices](#best-practices)

---

## Job Chaining

Job chaining allows you to execute multiple jobs sequentially, where each job runs only after the previous one completes successfully. This is similar to Laravel's `Bus::chain()`.

### Basic Usage

```rust
use rf_jobs::prelude::*;

// Create a chain of jobs
JobChain::new()
    .then(ProcessPodcast::new())?
    .then(OptimizePodcast::new())?
    .then(ReleasePodcast::new())?
    .dispatch(&queue)
    .await?;
```

### Named Chains

Give your chains descriptive names for easier tracking:

```rust
JobChain::new()
    .name("podcast-processing-pipeline")
    .then(ProcessPodcast::new())?
    .then(OptimizePodcast::new())?
    .dispatch(&queue)
    .await?;
```

### Chain Progress Tracking

Monitor the progress of your chain:

```rust
let chain_id = chain.dispatch(&queue).await?;

// Later, check progress
let (current, total) = queue.chain_progress(chain_id).await?;
println!("Chain progress: {}/{}", current, total);
```

### Chain State Management

```rust
// Load chain state
let state = queue.load_chain_state(chain_id).await?;
println!("Status: {:?}", state.status);
println!("Current job: {}/{}", state.current_index, state.total_jobs);

// Cancel a running chain
queue.cancel_chain(chain_id).await?;

// Clean up completed chain
queue.delete_chain(chain_id).await?;
```

### Chain Status

Chains can have the following statuses:

- `Pending` - Chain created but not yet started
- `Running` - Chain is currently executing jobs
- `Completed` - All jobs completed successfully
- `Failed` - A job in the chain failed
- `Cancelled` - Chain was manually cancelled

### Error Handling

If any job in the chain fails, the entire chain stops:

```rust
impl Job for ProcessPodcast {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        // If this fails, the chain stops
        process_audio(&self.file).await?;
        Ok(())
    }

    async fn failed(&self, ctx: JobContext, error: JobError) {
        // Handle chain failure
        ctx.error(&format!("Chain failed at ProcessPodcast: {}", error));
    }
}
```

---

## Job Batching

Job batching allows you to execute multiple jobs in parallel and track their collective completion. This is similar to Laravel's `Bus::batch()`.

### Basic Usage

```rust
use rf_jobs::prelude::*;

// Create a batch of jobs
JobBatch::new()
    .add(ProcessPodcast::new(1))?
    .add(ProcessPodcast::new(2))?
    .add(ProcessPodcast::new(3))?
    .dispatch(&queue)
    .await?;
```

### Adding Multiple Jobs

```rust
// Add many jobs at once
let podcasts = vec![1, 2, 3, 4, 5];

JobBatch::new()
    .add_many(podcasts.into_iter().map(ProcessPodcast::new))?
    .dispatch(&queue)
    .await?;
```

### Named Batches

```rust
JobBatch::new()
    .name("podcast-batch-2024-01")
    .add_many(podcasts.into_iter().map(ProcessPodcast::new))?
    .dispatch(&queue)
    .await?;
```

### Completion Callbacks

**Note:** Callbacks are registered but actual execution would require a separate batch worker/listener system.

```rust
JobBatch::new()
    .add_many(jobs)?
    .then(|batch_id| async move {
        println!("Batch {} completed successfully!", batch_id);
    })
    .catch(|batch_id, error| async move {
        eprintln!("Batch {} failed: {}", batch_id, error);
    })
    .finally(|batch_id| async move {
        println!("Batch {} finished (success or failure)", batch_id);
    })
    .dispatch(&queue)
    .await?;
```

### Allow Failures

By default, if one job fails, the entire batch is marked as failed. You can change this:

```rust
JobBatch::new()
    .add_many(jobs)?
    .allow_failures(true) // Batch completes even if some jobs fail
    .dispatch(&queue)
    .await?;
```

### Batch Progress Tracking

```rust
let batch_id = batch.dispatch(&queue).await?;

// Check progress
let (completed, failed, pending, total) = queue.batch_progress(batch_id).await?;
println!("Progress: {}/{} completed, {} failed, {} pending",
    completed, total, failed, pending);
```

### Batch State Management

```rust
// Load batch state
let state = queue.load_batch_state(batch_id).await?;
println!("Status: {:?}", state.status);
println!("Completed: {}/{}", state.completed, state.total);
println!("Failed: {}", state.failed);

// Cancel a batch
queue.cancel_batch(batch_id).await?;

// Clean up
queue.delete_batch(batch_id).await?;
```

### Batch Status

Batches can have the following statuses:

- `Pending` - Batch created but no jobs started yet
- `Processing` - Jobs are being executed
- `Completed` - All jobs completed (successfully or with allowed failures)
- `Failed` - Batch failed (only if `allow_failures` is false)
- `Cancelled` - Batch was manually cancelled

---

## Rate Limiting

Rate limiting controls how many times a job can execute within a given time window. This is useful for respecting API rate limits, preventing abuse, or controlling resource usage.

### Basic Usage

```rust
use rf_jobs::prelude::*;
use std::time::Duration;

let limiter = RateLimiter::new(queue.clone());

// Allow 10 emails per minute
if limiter.allow("emails", 10, Duration::from_secs(60)).await? {
    // Execute job
    send_email(&recipient).await?;
} else {
    // Rate limit exceeded
    ctx.warn("Rate limit exceeded, skipping email");
}
```

### In Job Implementation

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SendEmailJob {
    to: String,
    subject: String,
}

#[async_trait]
impl Job for SendEmailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        let limiter = RateLimiter::new(queue);

        // Check rate limit
        if !limiter.allow("emails", 100, Duration::from_secs(3600)).await? {
            // Rate limit exceeded, requeue for later
            return Err(JobError::Custom("Rate limit exceeded".into()));
        }

        // Proceed with sending email
        send_email(&self.to, &self.subject).await?;
        Ok(())
    }
}
```

### Wait for Availability

Instead of failing, you can wait until a slot becomes available:

```rust
// This will block until rate limit allows
limiter.wait_for_slot("api_calls", 100, Duration::from_secs(60)).await?;

// Now execute the job
make_api_call().await?;
```

### Check Remaining Slots

```rust
let remaining = limiter.remaining("api_calls", 100, Duration::from_secs(60)).await?;
println!("Remaining API calls: {}", remaining);
```

### Retry After

Get the time until the next slot becomes available:

```rust
match limiter.retry_after("api_calls", 100, Duration::from_secs(60)).await? {
    Some(ms) => {
        println!("Rate limit exceeded, retry after {} ms", ms);
        tokio::time::sleep(Duration::from_millis(ms)).await;
    }
    None => {
        println!("Slots available");
    }
}
```

### Reset Rate Limit

```rust
// Reset rate limit for a key
limiter.reset("api_calls").await?;
```

### Acquire Multiple Slots

```rust
// Try to acquire 5 slots at once
let acquired = limiter.acquire("bulk_api", 5, 100, Duration::from_secs(60)).await?;
println!("Acquired {} slots", acquired);
```

### Rate Limiting Patterns

#### Per-User Rate Limiting

```rust
let user_key = format!("user:{}:emails", user_id);
limiter.allow(&user_key, 10, Duration::from_secs(3600)).await?;
```

#### Per-API Endpoint Rate Limiting

```rust
let endpoint_key = format!("api:{}", endpoint_name);
limiter.allow(&endpoint_key, 1000, Duration::from_secs(60)).await?;
```

#### Global Rate Limiting

```rust
limiter.allow("global:api", 10000, Duration::from_secs(3600)).await?;
```

---

## Priority Queues

Priority queues allow you to control the order in which jobs are processed. High-priority jobs are always processed before lower-priority jobs.

### Priority Levels

RustForge provides three priority levels:

- `QueuePriority::High` - Processed first
- `QueuePriority::Default` - Standard priority
- `QueuePriority::Low` - Processed last

### Dispatching with Priority

```rust
use rf_jobs::prelude::*;

// Dispatch to high priority queue
queue.dispatch_with_priority(
    UrgentNotification::new(),
    QueuePriority::High
).await?;

// Dispatch to default priority
queue.dispatch_with_priority(
    RegularEmail::new(),
    QueuePriority::Default
).await?;

// Dispatch to low priority
queue.dispatch_with_priority(
    AnalyticsJob::new(),
    QueuePriority::Low
).await?;
```

### Dispatching to Named Queue with Priority

```rust
queue.dispatch_on(
    ProcessImage::new(),
    "images",
    QueuePriority::High
).await?;
```

### Worker Configuration for Priority Queues

Workers should use `pop_with_priority` to respect priority ordering:

```rust
// Worker automatically pops from high -> default -> low
let payload = queue.pop_with_priority("default", Duration::from_secs(5)).await?;
```

### Priority Queue Patterns

#### Critical vs. Background Jobs

```rust
// Critical user-facing job
queue.dispatch_with_priority(
    SendPasswordResetEmail::new(),
    QueuePriority::High
).await?;

// Background analytics
queue.dispatch_with_priority(
    GenerateAnalyticsReport::new(),
    QueuePriority::Low
).await?;
```

#### Time-Sensitive Jobs

```rust
// Real-time notification
queue.dispatch_with_priority(
    PushNotification::new(),
    QueuePriority::High
).await?;

// Scheduled cleanup
queue.dispatch_with_priority(
    CleanupTempFiles::new(),
    QueuePriority::Low
).await?;
```

#### Multi-Tenant Priority

```rust
// Premium customer jobs
if user.is_premium() {
    queue.dispatch_with_priority(job, QueuePriority::High).await?;
} else {
    queue.dispatch_with_priority(job, QueuePriority::Default).await?;
}
```

---

## Best Practices

### Job Chaining

1. **Keep Chains Short**: Long chains are harder to debug and maintain. Consider breaking into multiple smaller chains.

2. **Handle Failures Gracefully**: Implement the `failed()` method to clean up resources when chains fail.

3. **Use Descriptive Names**: Name your chains to make monitoring and debugging easier.

4. **Clean Up**: Delete completed chain data to avoid Redis bloat.

### Job Batching

1. **Batch Size**: Keep batches to a reasonable size (100-1000 jobs). For larger sets, create multiple batches.

2. **Monitor Progress**: Use progress tracking to display status to users.

3. **Allow Failures When Appropriate**: For non-critical batch operations, allow failures to let other jobs complete.

4. **Idempotency**: Ensure batch jobs are idempotent in case they need to be retried.

### Rate Limiting

1. **Choose Appropriate Windows**: Match rate limit windows to your API provider's limits.

2. **Use Specific Keys**: Create unique keys for different rate limit types.

3. **Handle Exceeded Limits**: Decide whether to fail, wait, or requeue when limits are exceeded.

4. **Monitor Usage**: Track remaining slots to predict when limits will be hit.

### Priority Queues

1. **Use Sparingly**: Too many high-priority jobs defeats the purpose.

2. **Reserve High Priority**: Only use `High` priority for truly critical jobs.

3. **Separate Queues**: Consider using separate named queues for different job types rather than relying solely on priority.

4. **Worker Scaling**: Scale workers based on high-priority queue depth.

### General Tips

1. **Monitoring**: Implement monitoring for chain/batch completion rates.

2. **Cleanup**: Regularly clean up old chain/batch data from Redis.

3. **Testing**: Use the provided integration tests as examples for testing your implementations.

4. **Resource Limits**: Set appropriate timeouts and retry limits to prevent resource exhaustion.

---

## Complete Example

Here's a complete example using all features:

```rust
use rf_jobs::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProcessVideoJob {
    video_id: u64,
}

#[async_trait]
impl Job for ProcessVideoJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        let queue = QueueManager::new("redis://localhost:6379").await?;
        let limiter = RateLimiter::new(queue.clone());

        // Check rate limit for video processing API
        if !limiter.allow("video_api", 10, Duration::from_secs(60)).await? {
            return Err(JobError::Custom("Rate limit exceeded".into()));
        }

        // Process video
        process_video(self.video_id).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = QueueManager::new("redis://localhost:6379").await?;

    // Create a batch of video processing jobs
    let video_ids = vec![1, 2, 3, 4, 5];

    let batch = JobBatch::new()
        .name("video-processing-batch")
        .add_many(video_ids.into_iter().map(|id| ProcessVideoJob { video_id: id }))?
        .allow_failures(true)
        .then(|batch_id| async move {
            println!("All videos processed!");
        });

    let batch_id = batch.dispatch(&queue).await?;

    // For the first video, create a chain of post-processing jobs
    JobChain::new()
        .name("video-1-postprocessing")
        .then(GenerateThumbnails::new(1))?
        .then(CreatePreview::new(1))?
        .then(PublishVideo::new(1))?
        .dispatch(&queue)
        .await?;

    // Dispatch urgent notification with high priority
    queue.dispatch_with_priority(
        NotifyUser::new(),
        QueuePriority::High
    ).await?;

    Ok(())
}
```

---

## Laravel Comparison

| Feature | Laravel | RustForge |
|---------|---------|-----------|
| **Chaining** | `Bus::chain([...])->dispatch()` | `JobChain::new().then(...).dispatch()` |
| **Batching** | `Bus::batch([...])->dispatch()` | `JobBatch::new().add_many(...).dispatch()` |
| **Rate Limiting** | `Redis::throttle('key')->allow(10)` | `limiter.allow("key", 10, duration)` |
| **Priority** | `Job::dispatch()->onQueue('high')` | `dispatch_with_priority(job, High)` |
| **Progress** | `$batch->progress()` | `batch_progress(batch_id)` |
| **Callbacks** | `->then(fn() => ...)` | `.then(\|id\| async move { ... })` |

---

## Redis Data Structure

### Chains

```
chain:{id}:state     → JSON: ChainState
chain:{id}:jobs      → JSON: Vec<SerializedJob>
```

### Batches

```
batch:{id}:state     → JSON: BatchState
```

### Rate Limits

```
rate_limit:{key}     → Sorted Set (timestamp-based sliding window)
```

### Priority Queues

```
queue:{name}:high    → List
queue:{name}:default → List
queue:{name}:low     → List
```

---

## Troubleshooting

### Chain Not Progressing

- Check if jobs are failing silently
- Verify Redis connection is stable
- Ensure workers are calling `handle_chain_job_completion`

### Batch Jobs Not Completing

- Verify `allow_failures` setting
- Check if workers are calling batch completion handlers
- Monitor Redis for orphaned batch state

### Rate Limit Not Working

- Ensure system clocks are synchronized
- Check Redis connectivity
- Verify rate limit keys are unique and consistent

### Priority Not Respected

- Ensure workers use `pop_with_priority`
- Verify queue names match priority format
- Check that high-priority queue isn't empty

---

For more information, see the [API documentation](https://docs.rs/rf-jobs) and [examples](../examples).
