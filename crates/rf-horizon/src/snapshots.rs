//! Metric snapshots for historical data and graphing

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::metrics::QueueMetrics;

/// A point-in-time snapshot of queue metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub timestamp: DateTime<Utc>,
    pub queues: HashMap<String, QueueSnapshot>,
}

impl MetricSnapshot {
    /// Create a new snapshot
    pub fn new() -> Self {
        Self {
            timestamp: Utc::now(),
            queues: HashMap::new(),
        }
    }

    /// Create snapshot from current metrics
    pub fn from_metrics(metrics: &HashMap<String, QueueMetrics>) -> Self {
        let queues = metrics
            .iter()
            .map(|(name, m)| (name.clone(), QueueSnapshot::from(m)))
            .collect();

        Self {
            timestamp: Utc::now(),
            queues,
        }
    }

    /// Add queue snapshot
    pub fn add_queue(&mut self, name: String, snapshot: QueueSnapshot) {
        self.queues.insert(name, snapshot);
    }

    /// Get queue snapshot
    pub fn get_queue(&self, name: &str) -> Option<&QueueSnapshot> {
        self.queues.get(name)
    }
}

impl Default for MetricSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

/// Snapshot of a single queue's metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueSnapshot {
    pub pending: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub throughput: f64,       // Jobs per minute
    pub avg_wait_time: f64,    // Seconds
    pub avg_runtime: f64,      // Seconds
    pub success_rate: f64,     // 0.0 to 1.0
}

impl QueueSnapshot {
    /// Create new queue snapshot
    pub fn new() -> Self {
        Self {
            pending: 0,
            processing: 0,
            completed: 0,
            failed: 0,
            throughput: 0.0,
            avg_wait_time: 0.0,
            avg_runtime: 0.0,
            success_rate: 1.0,
        }
    }

    /// Calculate total jobs
    pub fn total_jobs(&self) -> u64 {
        self.completed + self.failed
    }
}

impl Default for QueueSnapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl From<&QueueMetrics> for QueueSnapshot {
    fn from(metrics: &QueueMetrics) -> Self {
        Self {
            pending: metrics.jobs_pending,
            processing: 0, // TODO: Track from workers
            completed: metrics.jobs_processed,
            failed: metrics.jobs_failed,
            throughput: metrics.throughput_per_minute,
            avg_wait_time: metrics.average_wait_time_ms / 1000.0, // Convert to seconds
            avg_runtime: metrics.average_processing_time_ms / 1000.0,
            success_rate: metrics.success_rate(),
        }
    }
}

/// Store for maintaining historical snapshots
pub struct SnapshotStore {
    snapshots: Arc<RwLock<VecDeque<MetricSnapshot>>>,
    max_snapshots: usize,
}

impl SnapshotStore {
    /// Create a new snapshot store
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: Arc::new(RwLock::new(VecDeque::with_capacity(max_snapshots))),
            max_snapshots,
        }
    }

    /// Create with default capacity (1 hour at 1 snapshot/minute = 60)
    pub fn with_default_capacity() -> Self {
        Self::new(60)
    }

    /// Create for hourly retention (24 hours at 1 snapshot/minute = 1440)
    pub fn with_hourly_retention() -> Self {
        Self::new(1440)
    }

    /// Create for daily retention (7 days at 1 snapshot/minute = 10080)
    pub fn with_daily_retention() -> Self {
        Self::new(10080)
    }

    /// Take a snapshot of current metrics
    pub async fn take_snapshot(&self, metrics: &HashMap<String, QueueMetrics>) {
        let snapshot = MetricSnapshot::from_metrics(metrics);
        let mut snapshots = self.snapshots.write().await;

        snapshots.push_back(snapshot);

        // Remove old snapshots if we exceed capacity
        while snapshots.len() > self.max_snapshots {
            snapshots.pop_front();
        }
    }

    /// Get all snapshots
    pub async fn all(&self) -> Vec<MetricSnapshot> {
        self.snapshots.read().await.iter().cloned().collect()
    }

    /// Get snapshots within a time range
    pub async fn range(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<MetricSnapshot> {
        self.snapshots
            .read()
            .await
            .iter()
            .filter(|s| s.timestamp >= from && s.timestamp <= to)
            .cloned()
            .collect()
    }

    /// Get snapshots for the last N minutes
    pub async fn last_minutes(&self, minutes: i64) -> Vec<MetricSnapshot> {
        let from = Utc::now() - chrono::Duration::minutes(minutes);
        let to = Utc::now();
        self.range(from, to).await
    }

    /// Get snapshots for the last N hours
    pub async fn last_hours(&self, hours: i64) -> Vec<MetricSnapshot> {
        let from = Utc::now() - chrono::Duration::hours(hours);
        let to = Utc::now();
        self.range(from, to).await
    }

    /// Get the most recent snapshot
    pub async fn latest(&self) -> Option<MetricSnapshot> {
        self.snapshots.read().await.back().cloned()
    }

    /// Get the oldest snapshot
    pub async fn oldest(&self) -> Option<MetricSnapshot> {
        self.snapshots.read().await.front().cloned()
    }

    /// Get snapshot count
    pub async fn count(&self) -> usize {
        self.snapshots.read().await.len()
    }

    /// Clear all snapshots
    pub async fn clear(&self) {
        self.snapshots.write().await.clear();
    }

    /// Get time series data for a specific queue
    pub async fn queue_time_series(
        &self,
        queue_name: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Vec<QueueDataPoint> {
        let snapshots = self.range(from, to).await;

        snapshots
            .iter()
            .filter_map(|snapshot| {
                snapshot.get_queue(queue_name).map(|queue| QueueDataPoint {
                    timestamp: snapshot.timestamp,
                    pending: queue.pending,
                    processing: queue.processing,
                    completed: queue.completed,
                    failed: queue.failed,
                    throughput: queue.throughput,
                    avg_wait_time: queue.avg_wait_time,
                    avg_runtime: queue.avg_runtime,
                    success_rate: queue.success_rate,
                })
            })
            .collect()
    }

    /// Get aggregated statistics for a queue over a time range
    pub async fn queue_stats(
        &self,
        queue_name: &str,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> Option<QueueStats> {
        let data_points = self.queue_time_series(queue_name, from, to).await;

        if data_points.is_empty() {
            return None;
        }

        let count = data_points.len();

        let total_completed: u64 = data_points.iter().map(|p| p.completed).sum();
        let total_failed: u64 = data_points.iter().map(|p| p.failed).sum();
        let avg_throughput: f64 = data_points.iter().map(|p| p.throughput).sum::<f64>() / count as f64;
        let avg_wait_time: f64 = data_points.iter().map(|p| p.avg_wait_time).sum::<f64>() / count as f64;
        let avg_runtime: f64 = data_points.iter().map(|p| p.avg_runtime).sum::<f64>() / count as f64;
        let avg_success_rate: f64 = data_points.iter().map(|p| p.success_rate).sum::<f64>() / count as f64;

        let max_pending = data_points.iter().map(|p| p.pending).max().unwrap_or(0);
        let max_throughput = data_points
            .iter()
            .map(|p| p.throughput)
            .fold(0.0, f64::max);

        Some(QueueStats {
            queue_name: queue_name.to_string(),
            from,
            to,
            total_completed,
            total_failed,
            avg_throughput,
            avg_wait_time,
            avg_runtime,
            avg_success_rate,
            max_pending,
            max_throughput,
            data_points: count,
        })
    }
}

impl Clone for SnapshotStore {
    fn clone(&self) -> Self {
        Self {
            snapshots: Arc::clone(&self.snapshots),
            max_snapshots: self.max_snapshots,
        }
    }
}

impl Default for SnapshotStore {
    fn default() -> Self {
        Self::with_default_capacity()
    }
}

/// A single data point in a time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueDataPoint {
    pub timestamp: DateTime<Utc>,
    pub pending: u64,
    pub processing: u64,
    pub completed: u64,
    pub failed: u64,
    pub throughput: f64,
    pub avg_wait_time: f64,
    pub avg_runtime: f64,
    pub success_rate: f64,
}

/// Aggregated statistics for a queue over a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    pub queue_name: String,
    pub from: DateTime<Utc>,
    pub to: DateTime<Utc>,
    pub total_completed: u64,
    pub total_failed: u64,
    pub avg_throughput: f64,
    pub avg_wait_time: f64,
    pub avg_runtime: f64,
    pub avg_success_rate: f64,
    pub max_pending: u64,
    pub max_throughput: f64,
    pub data_points: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_queue_snapshot_new() {
        let snapshot = QueueSnapshot::new();
        assert_eq!(snapshot.pending, 0);
        assert_eq!(snapshot.completed, 0);
        assert_eq!(snapshot.success_rate, 1.0);
    }

    #[test]
    fn test_queue_snapshot_from_metrics() {
        let mut metrics = QueueMetrics::new("test");
        metrics.jobs_processed = 100;
        metrics.jobs_failed = 10;
        metrics.jobs_pending = 5;
        metrics.throughput_per_minute = 10.5;

        let snapshot = QueueSnapshot::from(&metrics);
        assert_eq!(snapshot.completed, 100);
        assert_eq!(snapshot.failed, 10);
        assert_eq!(snapshot.pending, 5);
        assert_eq!(snapshot.throughput, 10.5);
    }

    #[test]
    fn test_metric_snapshot_new() {
        let snapshot = MetricSnapshot::new();
        assert_eq!(snapshot.queues.len(), 0);
    }

    #[tokio::test]
    async fn test_snapshot_store_take_snapshot() {
        let store = SnapshotStore::new(10);
        let mut metrics = HashMap::new();

        let mut queue_metrics = QueueMetrics::new("test");
        queue_metrics.jobs_processed = 50;
        metrics.insert("test".to_string(), queue_metrics);

        store.take_snapshot(&metrics).await;

        assert_eq!(store.count().await, 1);
    }

    #[tokio::test]
    async fn test_snapshot_store_max_capacity() {
        let store = SnapshotStore::new(3);
        let metrics = HashMap::new();

        for _ in 0..5 {
            store.take_snapshot(&metrics).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        // Should only keep last 3 snapshots
        assert_eq!(store.count().await, 3);
    }

    #[tokio::test]
    async fn test_snapshot_store_latest() {
        let store = SnapshotStore::new(10);
        let mut metrics = HashMap::new();

        let mut queue_metrics = QueueMetrics::new("test");
        queue_metrics.jobs_processed = 100;
        metrics.insert("test".to_string(), queue_metrics);

        store.take_snapshot(&metrics).await;

        let latest = store.latest().await;
        assert!(latest.is_some());

        let snapshot = latest.unwrap();
        assert_eq!(snapshot.queues.len(), 1);
    }

    #[tokio::test]
    async fn test_snapshot_store_range() {
        let store = SnapshotStore::new(10);
        let metrics = HashMap::new();

        let start = Utc::now();

        for _ in 0..5 {
            store.take_snapshot(&metrics).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let end = Utc::now();

        let snapshots = store.range(start, end).await;
        assert_eq!(snapshots.len(), 5);
    }

    #[tokio::test]
    async fn test_snapshot_store_queue_time_series() {
        let store = SnapshotStore::new(10);
        let mut metrics = HashMap::new();

        let mut queue_metrics = QueueMetrics::new("test");
        queue_metrics.jobs_processed = 50;
        queue_metrics.throughput_per_minute = 5.0;
        metrics.insert("test".to_string(), queue_metrics);

        let start = Utc::now();

        for _ in 0..3 {
            store.take_snapshot(&metrics).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let end = Utc::now();

        let time_series = store.queue_time_series("test", start, end).await;
        assert_eq!(time_series.len(), 3);
        assert_eq!(time_series[0].completed, 50);
    }

    #[tokio::test]
    async fn test_snapshot_store_queue_stats() {
        let store = SnapshotStore::new(10);
        let mut metrics = HashMap::new();

        let mut queue_metrics = QueueMetrics::new("test");
        queue_metrics.jobs_processed = 100;
        queue_metrics.jobs_failed = 10;
        queue_metrics.throughput_per_minute = 10.5;
        metrics.insert("test".to_string(), queue_metrics);

        let start = Utc::now();

        for _ in 0..5 {
            store.take_snapshot(&metrics).await;
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        let end = Utc::now();

        let stats = store.queue_stats("test", start, end).await;
        assert!(stats.is_some());

        let stats = stats.unwrap();
        assert_eq!(stats.queue_name, "test");
        assert_eq!(stats.data_points, 5);
        // The total_completed is summed across all snapshots
        // 5 snapshots * 100 completed each = 500 total
        assert_eq!(stats.total_completed, 500);
    }

    #[tokio::test]
    async fn test_snapshot_store_clear() {
        let store = SnapshotStore::new(10);
        let metrics = HashMap::new();

        store.take_snapshot(&metrics).await;
        assert_eq!(store.count().await, 1);

        store.clear().await;
        assert_eq!(store.count().await, 0);
    }
}
