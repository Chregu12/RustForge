//! Real-time metrics collection for queue monitoring

use crate::metrics::{JobHistoryEntry, QueueMetrics, WorkerInfo};
use chrono::Utc;
use rf_jobs::QueueManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{interval, Duration};

/// Metrics collector that periodically gathers queue statistics
#[derive(Clone)]
pub struct MetricsCollector {
    queue_manager: QueueManager,
    metrics: Arc<RwLock<HashMap<String, QueueMetrics>>>,
    job_history: Arc<RwLock<Vec<JobHistoryEntry>>>,
    workers: Arc<RwLock<Vec<WorkerInfo>>>,
    monitored_queues: Vec<String>,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(queue_manager: QueueManager, monitored_queues: Vec<String>) -> Self {
        Self {
            queue_manager,
            metrics: Arc::new(RwLock::new(HashMap::new())),
            job_history: Arc::new(RwLock::new(Vec::new())),
            workers: Arc::new(RwLock::new(Vec::new())),
            monitored_queues,
        }
    }

    /// Start collecting metrics in the background
    pub async fn start(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(5));

            loop {
                interval.tick().await;
                if let Err(e) = self.collect().await {
                    eprintln!("Metrics collection error: {}", e);
                }
            }
        })
    }

    /// Collect metrics for all monitored queues
    async fn collect(&self) -> anyhow::Result<()> {
        let mut metrics_map = self.metrics.write().await;

        for queue_name in &self.monitored_queues {
            // Get queue size
            let pending = self.queue_manager.size(queue_name).await.unwrap_or(0);

            // Update or create metrics
            let metrics = metrics_map
                .entry(queue_name.clone())
                .or_insert_with(|| QueueMetrics::new(queue_name));

            metrics.set_pending(pending);
        }

        // Cleanup old job history (keep last 1000 entries)
        let mut history = self.job_history.write().await;
        if history.len() > 1000 {
            let len = history.len();
            history.drain(0..len - 1000);
        }

        Ok(())
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> HashMap<String, QueueMetrics> {
        self.metrics.read().await.clone()
    }

    /// Get job history
    pub async fn get_job_history(&self, limit: usize) -> Vec<JobHistoryEntry> {
        let history = self.job_history.read().await;
        history.iter().rev().take(limit).cloned().collect()
    }

    /// Get worker information
    pub async fn get_workers(&self) -> Vec<WorkerInfo> {
        self.workers.read().await.clone()
    }

    /// Record a job start
    pub async fn record_job_start(
        &self,
        job_id: impl Into<String>,
        queue: impl Into<String>,
        job_name: impl Into<String>,
    ) {
        let mut history = self.job_history.write().await;
        history.push(JobHistoryEntry::new(job_id, queue, job_name));
    }

    /// Record a job success
    pub async fn record_job_success(
        &self,
        job_id: &str,
        queue: &str,
        processing_time_ms: f64,
    ) {
        // Update metrics
        let mut metrics_map = self.metrics.write().await;
        if let Some(metrics) = metrics_map.get_mut(queue) {
            metrics.record_success(processing_time_ms);
        }

        // Update job history
        let mut history = self.job_history.write().await;
        if let Some(entry) = history.iter_mut().find(|e| e.id == job_id) {
            entry.complete();
        }
    }

    /// Record a job failure
    pub async fn record_job_failure(&self, job_id: &str, queue: &str, error: impl Into<String>) {
        // Update metrics
        let mut metrics_map = self.metrics.write().await;
        if let Some(metrics) = metrics_map.get_mut(queue) {
            metrics.record_failure();
        }

        // Update job history
        let mut history = self.job_history.write().await;
        if let Some(entry) = history.iter_mut().find(|e| e.id == job_id) {
            entry.fail(error);
        }
    }

    /// Register a worker
    pub async fn register_worker(&self, worker_id: impl Into<String>, queue: impl Into<String>) {
        let mut workers = self.workers.write().await;
        workers.push(WorkerInfo::new(worker_id, queue));
    }

    /// Update worker status
    pub async fn update_worker_status(
        &self,
        worker_id: &str,
        processing: Option<String>,
    ) {
        let mut workers = self.workers.write().await;
        if let Some(worker) = workers.iter_mut().find(|w| w.id == worker_id) {
            if let Some(job_name) = processing {
                worker.start_processing(job_name);
            } else {
                worker.finish_processing();
            }
        }
    }

    /// Remove inactive workers
    pub async fn cleanup_workers(&self, timeout_secs: i64) {
        let mut workers = self.workers.write().await;
        let now = Utc::now();
        workers.retain(|w| {
            (now - w.last_activity).num_seconds() < timeout_secs
        });
    }

    /// Get metrics for a specific queue
    pub async fn get_queue_metrics(&self, queue_name: &str) -> Option<QueueMetrics> {
        self.metrics.read().await.get(queue_name).cloned()
    }

    /// Calculate aggregate statistics
    pub async fn get_aggregate_stats(&self) -> AggregateStats {
        let metrics_map = self.metrics.read().await;
        let history = self.job_history.read().await;

        let mut total_processed = 0u64;
        let mut total_failed = 0u64;
        let mut total_pending = 0u64;
        let mut total_throughput = 0.0;

        for metrics in metrics_map.values() {
            total_processed += metrics.jobs_processed;
            total_failed += metrics.jobs_failed;
            total_pending += metrics.jobs_pending;
            total_throughput += metrics.throughput_per_minute;
        }

        let recent_jobs = history.iter().rev().take(100).cloned().collect();

        AggregateStats {
            total_processed,
            total_failed,
            total_pending,
            total_throughput,
            recent_jobs,
            queues: metrics_map.len(),
        }
    }
}

/// Aggregate statistics across all queues
#[derive(Debug, Clone, serde::Serialize)]
pub struct AggregateStats {
    pub total_processed: u64,
    pub total_failed: u64,
    pub total_pending: u64,
    pub total_throughput: f64,
    pub recent_jobs: Vec<JobHistoryEntry>,
    pub queues: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobHistoryStatus;

    #[tokio::test]
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
    async fn test_record_job_lifecycle() {
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
    async fn test_record_job_failure() {
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
    async fn test_worker_registration() {
        let queue_manager = QueueManager::new("redis://localhost:6379")
            .await
            .expect("Failed to create queue manager");
        let collector = MetricsCollector::new(queue_manager, vec!["default".to_string()]);

        collector
            .register_worker("worker-1", "default")
            .await;

        let workers = collector.get_workers().await;
        assert_eq!(workers.len(), 1);
        assert_eq!(workers[0].id, "worker-1");
    }

    #[tokio::test]
    async fn test_aggregate_stats() {
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
}
