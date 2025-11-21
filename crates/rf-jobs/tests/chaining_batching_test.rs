//! Integration tests for Job Chaining, Batching, Rate Limiting, and Priority Queues

use async_trait::async_trait;
use rf_jobs::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;

// Test job for counters
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CounterJob {
    id: u32,
}

#[async_trait]
impl Job for CounterJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Executing CounterJob {}", self.id));
        // Small delay to simulate work
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

// Simple test job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestJob {
    value: i32,
}

#[async_trait]
impl Job for TestJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Executing TestJob with value {}", self.value));
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

// Rate limited job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RateLimitedJob {
    id: u32,
}

#[async_trait]
impl Job for RateLimitedJob {
    async fn handle(&self, _ctx: JobContext) -> JobResult {
        // Job implementation
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(())
    }
}

// Priority test job
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PriorityJob {
    id: u32,
    priority: String,
}

#[async_trait]
impl Job for PriorityJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log(&format!("Executing PriorityJob {} with priority {}", self.id, self.priority));
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }
}

#[tokio::test]
async fn test_job_chain_creation() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_job_chain_creation: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let chain = JobChain::new()
        .name("test-chain")
        .then(TestJob { value: 1 })
        .expect("Failed to add job")
        .then(TestJob { value: 2 })
        .expect("Failed to add job")
        .then(TestJob { value: 3 })
        .expect("Failed to add job");

    let chain_id = chain.dispatch(&manager).await.expect("Failed to dispatch chain");

    // Verify chain was created
    assert!(!chain_id.is_nil());

    // Load chain state
    let state = manager.load_chain_state(chain_id).await.expect("Failed to load chain state");
    assert_eq!(state.total_jobs, 3);
    assert_eq!(state.current_index, 0);
    assert_eq!(state.status, ChainStatus::Pending);
    assert_eq!(state.name, Some("test-chain".to_string()));

    // Cleanup
    manager.delete_chain(chain_id).await.expect("Failed to delete chain");
}

#[tokio::test]
async fn test_job_chain_progress() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_job_chain_progress: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let chain = JobChain::new()
        .then(TestJob { value: 1 })
        .expect("Failed to add job")
        .then(TestJob { value: 2 })
        .expect("Failed to add job")
        .then(TestJob { value: 3 })
        .expect("Failed to add job");

    let chain_id = chain.dispatch(&manager).await.expect("Failed to dispatch chain");

    // Check initial progress
    let (current, total) = manager.chain_progress(chain_id).await.expect("Failed to get progress");
    assert_eq!(current, 0);
    assert_eq!(total, 3);

    // Simulate job completion
    manager.handle_chain_job_completion(chain_id, 0).await.expect("Failed to complete job");

    let (current, total) = manager.chain_progress(chain_id).await.expect("Failed to get progress");
    assert_eq!(current, 1);
    assert_eq!(total, 3);

    // Cleanup
    manager.delete_chain(chain_id).await.expect("Failed to delete chain");
}

#[tokio::test]
async fn test_job_batch_creation() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_job_batch_creation: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let jobs: Vec<TestJob> = (0..10).map(|i| TestJob { value: i }).collect();

    let batch = JobBatch::new()
        .name("test-batch")
        .add_many(jobs)
        .expect("Failed to add jobs");

    let batch_id = batch.dispatch(&manager).await.expect("Failed to dispatch batch");

    // Verify batch was created
    assert!(!batch_id.is_nil());

    // Load batch state
    let state = manager.load_batch_state(batch_id).await.expect("Failed to load batch state");
    assert_eq!(state.total, 10);
    assert_eq!(state.pending, 10);
    assert_eq!(state.completed, 0);
    assert_eq!(state.status, BatchStatus::Processing);
    assert_eq!(state.name, Some("test-batch".to_string()));

    // Cleanup
    manager.delete_batch(batch_id).await.expect("Failed to delete batch");
}

#[tokio::test]
async fn test_job_batch_progress() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_job_batch_progress: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let jobs: Vec<TestJob> = (0..5).map(|i| TestJob { value: i }).collect();

    let batch = JobBatch::new()
        .add_many(jobs)
        .expect("Failed to add jobs");

    let batch_id = batch.dispatch(&manager).await.expect("Failed to dispatch batch");

    // Check initial progress
    let (completed, failed, pending, total) = manager
        .batch_progress(batch_id)
        .await
        .expect("Failed to get progress");
    assert_eq!(completed, 0);
    assert_eq!(failed, 0);
    assert_eq!(pending, 5);
    assert_eq!(total, 5);

    // Simulate job completion
    manager.handle_batch_job_completion(batch_id).await.expect("Failed to complete job");
    manager.handle_batch_job_completion(batch_id).await.expect("Failed to complete job");

    let (completed, failed, pending, total) = manager
        .batch_progress(batch_id)
        .await
        .expect("Failed to get progress");
    assert_eq!(completed, 2);
    assert_eq!(failed, 0);
    assert_eq!(pending, 3);
    assert_eq!(total, 5);

    // Cleanup
    manager.delete_batch(batch_id).await.expect("Failed to delete batch");
}

#[tokio::test]
async fn test_batch_allow_failures() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_batch_allow_failures: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let jobs: Vec<TestJob> = (0..3).map(|i| TestJob { value: i }).collect();

    let batch = JobBatch::new()
        .add_many(jobs)
        .expect("Failed to add jobs")
        .allow_failures(true);

    let batch_id = batch.dispatch(&manager).await.expect("Failed to dispatch batch");

    // Simulate one job failure
    manager
        .handle_batch_job_failure(batch_id, "Test error".to_string())
        .await
        .expect("Failed to handle job failure");

    // Complete remaining jobs
    manager.handle_batch_job_completion(batch_id).await.expect("Failed to complete job");
    manager.handle_batch_job_completion(batch_id).await.expect("Failed to complete job");

    let state = manager.load_batch_state(batch_id).await.expect("Failed to load batch state");
    assert_eq!(state.status, BatchStatus::Completed); // Should complete despite failure
    assert_eq!(state.failed, 1);
    assert_eq!(state.completed, 2);

    // Cleanup
    manager.delete_batch(batch_id).await.expect("Failed to delete batch");
}

#[tokio::test]
async fn test_rate_limiter_allow() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_rate_limiter_allow: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let limiter = RateLimiter::new(manager.clone());

    // Reset to ensure clean state
    limiter.reset("test_key").await.expect("Failed to reset");

    // Should allow first 5 requests
    for i in 0..5 {
        let allowed = limiter
            .allow("test_key", 5, Duration::from_secs(60))
            .await
            .expect("Failed to check rate limit");
        assert!(allowed, "Request {} should be allowed", i);
    }

    // 6th request should be denied
    let allowed = limiter
        .allow("test_key", 5, Duration::from_secs(60))
        .await
        .expect("Failed to check rate limit");
    assert!(!allowed, "6th request should be denied");

    // Cleanup
    limiter.reset("test_key").await.expect("Failed to reset");
}

#[tokio::test]
async fn test_rate_limiter_remaining() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_rate_limiter_remaining: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let limiter = RateLimiter::new(manager.clone());

    limiter.reset("test_remaining").await.expect("Failed to reset");

    // Check initial remaining
    let remaining = limiter
        .remaining("test_remaining", 10, Duration::from_secs(60))
        .await
        .expect("Failed to get remaining");
    assert_eq!(remaining, 10);

    // Use 3 slots
    for _ in 0..3 {
        limiter
            .allow("test_remaining", 10, Duration::from_secs(60))
            .await
            .expect("Failed to allow");
    }

    // Check remaining
    let remaining = limiter
        .remaining("test_remaining", 10, Duration::from_secs(60))
        .await
        .expect("Failed to get remaining");
    assert_eq!(remaining, 7);

    // Cleanup
    limiter.reset("test_remaining").await.expect("Failed to reset");
}

#[tokio::test]
async fn test_priority_queue_dispatch() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_priority_queue_dispatch: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    // Dispatch jobs with different priorities
    let high_id = manager
        .dispatch_with_priority(PriorityJob { id: 1, priority: "high".to_string() }, QueuePriority::High)
        .await
        .expect("Failed to dispatch high priority job");

    let default_id = manager
        .dispatch_with_priority(PriorityJob { id: 2, priority: "default".to_string() }, QueuePriority::Default)
        .await
        .expect("Failed to dispatch default priority job");

    let low_id = manager
        .dispatch_with_priority(PriorityJob { id: 3, priority: "low".to_string() }, QueuePriority::Low)
        .await
        .expect("Failed to dispatch low priority job");

    assert!(!high_id.is_nil());
    assert!(!default_id.is_nil());
    assert!(!low_id.is_nil());

    // Verify jobs are in correct queues
    let high_size = manager.size("default:high").await.expect("Failed to get queue size");
    let default_size = manager.size("default:default").await.expect("Failed to get queue size");
    let low_size = manager.size("default:low").await.expect("Failed to get queue size");

    assert_eq!(high_size, 1);
    assert_eq!(default_size, 1);
    assert_eq!(low_size, 1);

    // Cleanup
    manager.clear("default:high").await.expect("Failed to clear queue");
    manager.clear("default:default").await.expect("Failed to clear queue");
    manager.clear("default:low").await.expect("Failed to clear queue");
}

#[tokio::test]
async fn test_priority_queue_pop_order() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_priority_queue_pop_order: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    // Clear queues
    manager.clear("test:high").await.expect("Failed to clear queue");
    manager.clear("test:default").await.expect("Failed to clear queue");
    manager.clear("test:low").await.expect("Failed to clear queue");

    // Dispatch in reverse priority order
    manager
        .dispatch_on(TestJob { value: 3 }, "test", QueuePriority::Low)
        .await
        .expect("Failed to dispatch");

    manager
        .dispatch_on(TestJob { value: 2 }, "test", QueuePriority::Default)
        .await
        .expect("Failed to dispatch");

    manager
        .dispatch_on(TestJob { value: 1 }, "test", QueuePriority::High)
        .await
        .expect("Failed to dispatch");

    // Pop should return high priority job first
    let payload = manager
        .pop_with_priority("test", Duration::from_secs(1))
        .await
        .expect("Failed to pop")
        .expect("No job found");

    let job: TestJob = payload.deserialize().expect("Failed to deserialize");
    assert_eq!(job.value, 1); // High priority job

    // Cleanup
    manager.clear("test:high").await.expect("Failed to clear queue");
    manager.clear("test:default").await.expect("Failed to clear queue");
    manager.clear("test:low").await.expect("Failed to clear queue");
}

#[tokio::test]
async fn test_chain_cancellation() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_chain_cancellation: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let chain = JobChain::new()
        .then(TestJob { value: 1 })
        .expect("Failed to add job")
        .then(TestJob { value: 2 })
        .expect("Failed to add job");

    let chain_id = chain.dispatch(&manager).await.expect("Failed to dispatch chain");

    // Cancel the chain
    manager.cancel_chain(chain_id).await.expect("Failed to cancel chain");

    // Verify chain is cancelled
    let state = manager.load_chain_state(chain_id).await.expect("Failed to load chain state");
    assert_eq!(state.status, ChainStatus::Cancelled);

    // Cleanup
    manager.delete_chain(chain_id).await.expect("Failed to delete chain");
}

#[tokio::test]
async fn test_batch_cancellation() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_batch_cancellation: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to connect to Redis");

    let batch = JobBatch::new()
        .add(TestJob { value: 1 })
        .expect("Failed to add job")
        .add(TestJob { value: 2 })
        .expect("Failed to add job");

    let batch_id = batch.dispatch(&manager).await.expect("Failed to dispatch batch");

    // Cancel the batch
    manager.cancel_batch(batch_id).await.expect("Failed to cancel batch");

    // Verify batch is cancelled
    let state = manager.load_batch_state(batch_id).await.expect("Failed to load batch state");
    assert_eq!(state.status, BatchStatus::Cancelled);

    // Cleanup
    manager.delete_batch(batch_id).await.expect("Failed to delete batch");
}
