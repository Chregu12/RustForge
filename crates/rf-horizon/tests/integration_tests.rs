//! Integration tests for rf-horizon

use rf_horizon::{
    collector::MetricsCollector, metrics::*, Horizon, HorizonBuilder, JobHistoryStatus,
};
use rf_jobs::QueueManager;

// ==================== Horizon Tests ====================

#[tokio::test]
async fn test_horizon_new() {
    let horizon = Horizon::new();
    let state = horizon.state().await;
    assert!(state.batches.is_empty());
    assert!(state.failed_jobs.is_empty());
    assert!(state.metrics.is_empty());
}

#[tokio::test]
async fn test_horizon_builder() {
    let horizon = Horizon::builder()
        .monitor_queue("test")
        .monitor_queue("emails")
        .failed_job_retention_days(14)
        .metrics_retention_hours(72)
        .enable_dashboard(true)
        .build();

    assert_eq!(horizon.config.monitored_queues.len(), 3); // default + test + emails
    assert_eq!(horizon.config.failed_job_retention_days, 14);
    assert_eq!(horizon.config.metrics_retention_hours, 72);
    assert!(horizon.config.enable_dashboard);
}

#[tokio::test]
async fn test_horizon_record_metrics() {
    let horizon = Horizon::new();
    let metrics = QueueMetrics::new("test");

    horizon
        .update_metrics("test".to_string(), metrics.clone())
        .await;

    let state = horizon.state().await;
    assert_eq!(state.metrics.len(), 1);
    assert!(state.metrics.contains_key("test"));
}

#[tokio::test]
async fn test_horizon_default_config() {
    let horizon = Horizon::new();
    assert_eq!(horizon.config.monitored_queues, vec!["default"]);
    assert_eq!(horizon.config.failed_job_retention_days, 7);
    assert_eq!(horizon.config.metrics_retention_hours, 48);
    assert!(horizon.config.enable_dashboard);
}

// ==================== Metrics Tests ====================

#[test]
fn test_queue_metrics_new() {
    let metrics = QueueMetrics::new("test");
    assert_eq!(metrics.queue_name, "test");
    assert_eq!(metrics.jobs_processed, 0);
    assert_eq!(metrics.jobs_failed, 0);
    assert_eq!(metrics.jobs_pending, 0);
}

#[test]
fn test_queue_metrics_record_success() {
    let mut metrics = QueueMetrics::new("test");
    metrics.record_success(100.0);

    assert_eq!(metrics.jobs_processed, 1);
    assert_eq!(metrics.average_processing_time_ms, 100.0);
}

#[test]
fn test_queue_metrics_record_failure() {
    let mut metrics = QueueMetrics::new("test");
    metrics.record_failure();

    assert_eq!(metrics.jobs_failed, 1);
    assert_eq!(metrics.jobs_processed, 0);
}

#[test]
fn test_queue_metrics_success_rate() {
    let mut metrics = QueueMetrics::new("test");

    // No jobs yet
    assert_eq!(metrics.success_rate(), 1.0);

    // 3 successes
    metrics.record_success(100.0);
    metrics.record_success(100.0);
    metrics.record_success(100.0);

    // 1 failure
    metrics.record_failure();

    // Success rate should be 75% (3/4)
    assert_eq!(metrics.success_rate(), 0.75);
}

#[test]
fn test_queue_metrics_set_pending() {
    let mut metrics = QueueMetrics::new("test");
    metrics.set_pending(42);

    assert_eq!(metrics.jobs_pending, 42);
}

#[test]
fn test_queue_metrics_average_processing_time() {
    let mut metrics = QueueMetrics::new("test");

    metrics.record_success(100.0);
    metrics.record_success(200.0);
    metrics.record_success(300.0);

    // Average should be 200.0
    assert_eq!(metrics.average_processing_time_ms, 200.0);
}

// ==================== Worker Tests ====================

#[test]
fn test_worker_info_new() {
    let worker = WorkerInfo::new("worker-1", "default");

    assert_eq!(worker.id, "worker-1");
    assert_eq!(worker.queue, "default");
    assert_eq!(worker.status, WorkerStatus::Idle);
    assert_eq!(worker.jobs_processed, 0);
}

#[test]
fn test_worker_start_processing() {
    let mut worker = WorkerInfo::new("worker-1", "default");
    worker.start_processing("SendEmailJob");

    match &worker.status {
        WorkerStatus::Processing { job_name } => {
            assert_eq!(job_name, "SendEmailJob");
        }
        _ => panic!("Worker should be processing"),
    }
}

#[test]
fn test_worker_finish_processing() {
    let mut worker = WorkerInfo::new("worker-1", "default");
    worker.start_processing("SendEmailJob");
    worker.finish_processing();

    assert_eq!(worker.status, WorkerStatus::Idle);
    assert_eq!(worker.jobs_processed, 1);
}

#[test]
fn test_worker_is_active() {
    let mut worker = WorkerInfo::new("worker-1", "default");
    assert!(worker.is_active());

    worker.start_processing("TestJob");
    assert!(worker.is_active());

    worker.status = WorkerStatus::Paused;
    assert!(!worker.is_active());

    worker.status = WorkerStatus::Stopped;
    assert!(!worker.is_active());
}

// ==================== Job History Tests ====================

#[test]
fn test_job_history_entry_new() {
    let entry = JobHistoryEntry::new("job-1", "default", "TestJob");

    assert_eq!(entry.id, "job-1");
    assert_eq!(entry.queue, "default");
    assert_eq!(entry.job_name, "TestJob");
    assert_eq!(entry.status, JobHistoryStatus::Pending);
    assert!(entry.completed_at.is_none());
    assert!(entry.error.is_none());
}

#[test]
fn test_job_history_complete() {
    let mut entry = JobHistoryEntry::new("job-1", "default", "TestJob");
    entry.complete();

    assert_eq!(entry.status, JobHistoryStatus::Completed);
    assert!(entry.completed_at.is_some());
    assert!(entry.duration_ms.is_some());
    assert!(entry.error.is_none());
}

#[test]
fn test_job_history_fail() {
    let mut entry = JobHistoryEntry::new("job-1", "default", "TestJob");
    entry.fail("Database connection failed");

    assert_eq!(entry.status, JobHistoryStatus::Failed);
    assert!(entry.completed_at.is_some());
    assert!(entry.duration_ms.is_some());
    assert_eq!(entry.error.as_ref().unwrap(), "Database connection failed");
}

// ==================== Metrics Collector Tests ====================

#[tokio::test]
#[ignore = "requires Redis"]
async fn test_metrics_collector_creation() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(
        queue_manager,
        vec!["default".to_string(), "emails".to_string()],
    );

    let metrics = collector.get_metrics().await;
    assert!(metrics.is_empty()); // No metrics collected yet
}

#[tokio::test]
#[ignore = "requires Redis"]
async fn test_collector_record_job_lifecycle() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(queue_manager, vec!["default".to_string()]);

    // Record job start
    collector
        .record_job_start("job-1", "default", "SendEmailJob")
        .await;

    // Record job success
    collector
        .record_job_success("job-1", "default", 150.0)
        .await;

    let history = collector.get_job_history(10).await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].id, "job-1");
    assert_eq!(history[0].status, JobHistoryStatus::Completed);
}

#[tokio::test]
#[ignore = "requires Redis"]
async fn test_collector_record_job_failure() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(queue_manager, vec!["default".to_string()]);

    collector
        .record_job_start("job-2", "default", "ProcessOrderJob")
        .await;

    collector
        .record_job_failure("job-2", "default", "Database connection failed")
        .await;

    let history = collector.get_job_history(10).await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].status, JobHistoryStatus::Failed);
    assert!(history[0].error.is_some());
}

#[tokio::test]
#[ignore = "requires Redis"]
async fn test_collector_worker_registration() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(queue_manager, vec!["default".to_string()]);

    collector.register_worker("worker-1", "default").await;

    let workers = collector.get_workers().await;
    assert_eq!(workers.len(), 1);
    assert_eq!(workers[0].id, "worker-1");
}

#[tokio::test]
#[ignore = "requires Redis"]
async fn test_collector_aggregate_stats() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(
        queue_manager,
        vec!["default".to_string(), "emails".to_string()],
    );

    // Record some jobs
    for i in 0..5 {
        let job_id = format!("job-{}", i);
        collector
            .record_job_start(&job_id, "default", "TestJob")
            .await;
        collector
            .record_job_success(&job_id, "default", 100.0)
            .await;
    }

    let stats = collector.get_aggregate_stats().await;
    assert_eq!(stats.recent_jobs.len(), 5);
}

// ==================== Builder Pattern Tests ====================

#[test]
fn test_horizon_builder_default() {
    let builder = HorizonBuilder::default();
    let horizon = builder.build();

    assert_eq!(horizon.config.monitored_queues, vec!["default"]);
}

#[test]
fn test_horizon_builder_multiple_queues() {
    let horizon = Horizon::builder()
        .monitor_queue("emails")
        .monitor_queue("notifications")
        .monitor_queue("reports")
        .build();

    assert_eq!(horizon.config.monitored_queues.len(), 4); // default + 3 more
}

#[test]
fn test_horizon_builder_custom_retention() {
    let horizon = Horizon::builder()
        .failed_job_retention_days(30)
        .metrics_retention_hours(168)
        .build();

    assert_eq!(horizon.config.failed_job_retention_days, 30);
    assert_eq!(horizon.config.metrics_retention_hours, 168);
}

// ==================== State Management Tests ====================

#[tokio::test]
async fn test_horizon_state_batches() {
    use rf_horizon::{BatchProgress, BatchStatus};

    let horizon = Horizon::new();

    let progress = BatchProgress {
        batch_id: "batch-1".to_string(),
        name: "test-batch".to_string(),
        total_jobs: 100,
        pending_jobs: 75,
        failed_jobs: 5,
        status: BatchStatus::Processing,
        created_at: chrono::Utc::now(),
        finished_at: None,
    };

    horizon.record_batch("batch-1".to_string(), progress).await;

    let state = horizon.state().await;
    assert_eq!(state.batches.len(), 1);
    assert!(state.batches.contains_key("batch-1"));
}

#[tokio::test]
async fn test_horizon_multiple_metrics() {
    let horizon = Horizon::new();

    for i in 0..5 {
        let queue = format!("queue-{}", i);
        let metrics = QueueMetrics::new(&queue);
        horizon.update_metrics(queue, metrics).await;
    }

    let state = horizon.state().await;
    assert_eq!(state.metrics.len(), 5);
}

// ==================== Additional Coverage Tests ====================

#[test]
fn test_worker_status_variants() {
    let idle = WorkerStatus::Idle;
    let processing = WorkerStatus::Processing {
        job_name: "TestJob".to_string(),
    };
    let paused = WorkerStatus::Paused;
    let stopped = WorkerStatus::Stopped;

    assert!(matches!(idle, WorkerStatus::Idle));
    assert!(matches!(processing, WorkerStatus::Processing { .. }));
    assert!(matches!(paused, WorkerStatus::Paused));
    assert!(matches!(stopped, WorkerStatus::Stopped));
}

#[test]
fn test_job_history_status_variants() {
    assert_eq!(JobHistoryStatus::Pending, JobHistoryStatus::Pending);
    assert_eq!(JobHistoryStatus::Processing, JobHistoryStatus::Processing);
    assert_eq!(JobHistoryStatus::Completed, JobHistoryStatus::Completed);
    assert_eq!(JobHistoryStatus::Failed, JobHistoryStatus::Failed);
}

#[test]
fn test_metrics_with_multiple_successes() {
    let mut metrics = QueueMetrics::new("test");

    for i in 0..10 {
        metrics.record_success((i * 10) as f64);
    }

    assert_eq!(metrics.jobs_processed, 10);
    assert!(metrics.average_processing_time_ms > 0.0);
}

#[test]
fn test_metrics_with_mixed_results() {
    let mut metrics = QueueMetrics::new("test");

    // 7 successes
    for _ in 0..7 {
        metrics.record_success(100.0);
    }

    // 3 failures
    for _ in 0..3 {
        metrics.record_failure();
    }

    assert_eq!(metrics.jobs_processed, 7);
    assert_eq!(metrics.jobs_failed, 3);
    assert_eq!(metrics.success_rate(), 0.7);
}

#[tokio::test]
async fn test_collector_job_history_limit() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(queue_manager, vec!["default".to_string()]);

    // Add 15 jobs
    for i in 0..15 {
        collector
            .record_job_start(format!("job-{}", i), "default", "TestJob")
            .await;
    }

    // Request only 10
    let history = collector.get_job_history(10).await;
    assert_eq!(history.len(), 10);
}

#[tokio::test]
async fn test_collector_cleanup_workers() {
    let queue_manager = QueueManager::new("redis://localhost:6379")
        .await
        .expect("Failed to create queue manager");
    let collector = MetricsCollector::new(queue_manager, vec!["default".to_string()]);

    collector.register_worker("worker-1", "default").await;

    // Cleanup with a timeout of 0 seconds (should remove all workers)
    collector.cleanup_workers(0).await;

    let workers = collector.get_workers().await;
    assert_eq!(workers.len(), 0);
}
