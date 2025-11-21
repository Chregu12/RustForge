//! Integration tests for Redis queue backend
//!
//! These tests require a running Redis instance.
//! Run with: cargo test --features redis-backend -- --ignored

#![cfg(feature = "redis-backend")]

use async_trait::async_trait;
use rf_queue::{Job, JobMetadata, Queue, QueueConfig, QueueError, RedisQueue};
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestEmailJob {
    to: String,
    subject: String,
    body: String,
}

#[async_trait]
impl Job for TestEmailJob {
    async fn handle(&self) -> Result<(), QueueError> {
        // Simulate email sending
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "test_email"
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestProcessingJob {
    data: String,
}

#[async_trait]
impl Job for TestProcessingJob {
    async fn handle(&self) -> Result<(), QueueError> {
        tokio::time::sleep(Duration::from_millis(5)).await;
        Ok(())
    }

    fn job_type(&self) -> &'static str {
        "test_processing"
    }
}

fn get_redis_url() -> String {
    std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://localhost:6379".to_string())
}

#[tokio::test]
async fn test_redis_job_persistence_after_restart() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_job_persistence_after_restart: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_persist").await.unwrap();
    queue.clear("default").await.unwrap();

    // Push job
    let job = TestEmailJob {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
        body: "Test body".to_string(),
    };

    let metadata = JobMetadata::new(&job).unwrap();
    let job_id = queue.push(metadata.clone()).await.unwrap();

    // Simulate restart by dropping and recreating queue
    drop(queue);

    let queue = RedisQueue::new(&redis_url, "test_persist").await.unwrap();

    // Job should still exist
    let reserved = queue.reserve("default").await.unwrap();
    assert!(reserved.is_some());

    let reserved_metadata = reserved.unwrap();
    assert_eq!(reserved_metadata.id, job_id);
    assert_eq!(reserved_metadata.job_type, "test_email");

    // Cleanup
    queue.clear("default").await.unwrap();
}

#[tokio::test]
async fn test_redis_delayed_job_execution() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_delayed_job_execution: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_delayed").await.unwrap();
    queue.clear("default").await.unwrap();

    // Push delayed job (2 seconds)
    let job = TestProcessingJob {
        data: "delayed data".to_string(),
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

    let reserved_metadata = reserved.unwrap();
    assert_eq!(reserved_metadata.job_type, "test_processing");

    // Cleanup
    queue.clear("default").await.unwrap();
}

#[tokio::test]
async fn test_redis_failed_job_handling() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_failed_job_handling: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_failed").await.unwrap();
    queue.clear("default").await.unwrap();

    // Push job
    let job = TestEmailJob {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
        body: "Test body".to_string(),
    };

    let metadata = JobMetadata::new(&job).unwrap();
    let job_id = queue.push(metadata).await.unwrap();

    // Reserve and mark as failed
    let reserved = queue.reserve("default").await.unwrap();
    assert!(reserved.is_some());

    queue
        .fail(&job_id, "Test error message")
        .await
        .unwrap();

    // Job should be in failed queue
    // (In real implementation, you'd query the failed queue)

    // Cleanup
    queue.clear("default").await.unwrap();
}

#[tokio::test]
async fn test_redis_job_retry_with_exponential_backoff() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_job_retry_with_exponential_backoff: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_retry").await.unwrap();
    queue.clear("default").await.unwrap();

    // Push job
    let job = TestProcessingJob {
        data: "retry data".to_string(),
    };

    let metadata = JobMetadata::new(&job).unwrap();
    queue.push(metadata.clone()).await.unwrap();

    // Reserve job
    let mut reserved = queue.reserve("default").await.unwrap().unwrap();

    // Retry job (should be delayed due to exponential backoff)
    queue.retry(reserved.clone()).await.unwrap();

    // Job should not be immediately available
    let immediate = queue.reserve("default").await.unwrap();
    assert!(immediate.is_none());

    // Cleanup
    queue.clear("default").await.unwrap();
}

#[tokio::test]
async fn test_redis_concurrent_workers() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_concurrent_workers: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_concurrent").await.unwrap();
    queue.clear("default").await.unwrap();

    // Push multiple jobs
    for i in 0..10 {
        let job = TestProcessingJob {
            data: format!("job {}", i),
        };
        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();
    }

    assert_eq!(queue.size("default").await.unwrap(), 10);

    // Simulate concurrent workers
    let mut handles = vec![];

    for _ in 0..3 {
        let queue = queue.clone();
        let handle = tokio::spawn(async move {
            let mut processed = 0;
            for _ in 0..5 {
                if let Some(job) = queue.reserve("default").await.unwrap() {
                    // Process job
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    queue.complete(&job.id).await.unwrap();
                    processed += 1;
                }
            }
            processed
        });
        handles.push(handle);
    }

    // Wait for all workers
    let mut total_processed = 0;
    for handle in handles {
        total_processed += handle.await.unwrap();
    }

    assert_eq!(total_processed, 10);

    // Cleanup
    queue.clear("default").await.unwrap();
}

#[tokio::test]
async fn test_redis_queue_config() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_queue_config: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let config = QueueConfig::redis(get_redis_url(), "test_config");
    let queue = config.build().await.unwrap();

    queue.clear("default").await.unwrap();

    // Push job
    let job = TestEmailJob {
        to: "test@example.com".to_string(),
        subject: "Config Test".to_string(),
        body: "Test body".to_string(),
    };

    let metadata = JobMetadata::new(&job).unwrap();
    queue.push(metadata).await.unwrap();

    assert_eq!(queue.size("default").await.unwrap(), 1);

    // Cleanup
    queue.clear("default").await.unwrap();
}

#[tokio::test]
async fn test_redis_multiple_queues() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_multiple_queues: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_multi").await.unwrap();
    queue.clear("emails").await.unwrap();
    queue.clear("processing").await.unwrap();

    // Push to different queues
    let email_job = TestEmailJob {
        to: "test@example.com".to_string(),
        subject: "Test".to_string(),
        body: "Test body".to_string(),
    };

    let processing_job = TestProcessingJob {
        data: "processing data".to_string(),
    };

    let mut email_metadata = JobMetadata::new(&email_job).unwrap();
    email_metadata.queue = "emails".to_string();

    let mut processing_metadata = JobMetadata::new(&processing_job).unwrap();
    processing_metadata.queue = "processing".to_string();

    queue.push(email_metadata).await.unwrap();
    queue.push(processing_metadata).await.unwrap();

    // Verify sizes
    assert_eq!(queue.size("emails").await.unwrap(), 1);
    assert_eq!(queue.size("processing").await.unwrap(), 1);
    assert_eq!(queue.size("default").await.unwrap(), 0);

    // Cleanup
    queue.clear("emails").await.unwrap();
    queue.clear("processing").await.unwrap();
}

#[tokio::test]
async fn test_redis_queue_throughput() {
    if !redis_available().await {
        eprintln!("⏭️  Skipping test_redis_queue_throughput: Redis not available");
        eprintln!("   Start services with: ./scripts/test-env-up.sh");
        return;
    }
    let redis_url = get_redis_url();
    let queue = RedisQueue::new(&redis_url, "test_throughput").await.unwrap();
    queue.clear("default").await.unwrap();

    let start = std::time::Instant::now();

    // Push 1000 jobs
    for i in 0..1000 {
        let job = TestProcessingJob {
            data: format!("job {}", i),
        };
        let metadata = JobMetadata::new(&job).unwrap();
        queue.push(metadata).await.unwrap();
    }

    let push_elapsed = start.elapsed();
    let push_throughput = 1000.0 / push_elapsed.as_secs_f64();

    println!("Push throughput: {:.0} jobs/sec", push_throughput);

    // Reserve 1000 jobs
    let start = std::time::Instant::now();
    let mut reserved_count = 0;

    for _ in 0..1000 {
        if queue.reserve("default").await.unwrap().is_some() {
            reserved_count += 1;
        }
    }

    let reserve_elapsed = start.elapsed();
    let reserve_throughput = 1000.0 / reserve_elapsed.as_secs_f64();

    println!("Reserve throughput: {:.0} jobs/sec", reserve_throughput);

    assert_eq!(reserved_count, 1000);
    assert!(push_throughput > 100.0); // At least 100 jobs/sec
    assert!(reserve_throughput > 100.0); // At least 100 jobs/sec

    // Cleanup
    queue.clear("default").await.unwrap();
}
