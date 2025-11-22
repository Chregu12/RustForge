//! # rf-horizon
//!
//! Laravel Horizon equivalent for Rust - Queue dashboard and advanced queue management.
//!
//! ## Features
//!
//! - Real-time queue monitoring
//! - Job batching with progress tracking
//! - Job chaining
//! - Advanced failed job handling
//! - Queue metrics and statistics
//! - Web-based dashboard with filtering, search, and pagination
//! - Job retry and deletion capabilities
//! - Batch operations for failed jobs
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_horizon::Horizon;
//! use rf_jobs::QueueManager;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create queue manager
//!     let queue_manager = QueueManager::new("redis://localhost:6379").await?;
//!
//!     // Create and configure Horizon
//!     let horizon = Horizon::builder()
//!         .queue_manager(queue_manager)
//!         .monitor_queue("default")
//!         .monitor_queue("emails")
//!         .failed_job_retention_days(7)
//!         .build();
//!
//!     // Start dashboard server
//!     horizon.serve("0.0.0.0:8080").await?;
//!     Ok(())
//! }
//! ```

pub mod batching;
pub mod chaining;
pub mod collector;
pub mod dashboard;
pub mod failed_jobs;
pub mod metrics;
pub mod routes;

pub use batching::{Batch, BatchProgress, BatchStatus};
pub use chaining::Chain;
pub use collector::{AggregateStats, MetricsCollector};
pub use dashboard::Dashboard;
pub use failed_jobs::{FailedJob, FailedJobHandler};
pub use metrics::{JobHistoryEntry, JobHistoryStatus, QueueMetrics, WorkerInfo, WorkerStatus};
pub use routes::{routes, AppState};

use anyhow::Result;
use rf_jobs::QueueManager;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Main Horizon instance for queue monitoring and management
#[derive(Clone)]
pub struct Horizon {
    pub config: Arc<HorizonConfig>,
    state: Arc<RwLock<HorizonState>>,
    collector: Option<MetricsCollector>,
}

/// Horizon configuration
#[derive(Debug, Clone)]
pub struct HorizonConfig {
    pub monitored_queues: Vec<String>,
    pub failed_job_retention_days: u32,
    pub metrics_retention_hours: u32,
    pub enable_dashboard: bool,
}

impl Default for HorizonConfig {
    fn default() -> Self {
        Self {
            monitored_queues: vec!["default".to_string()],
            failed_job_retention_days: 7,
            metrics_retention_hours: 48,
            enable_dashboard: true,
        }
    }
}

/// Internal Horizon state
#[derive(Debug, Default, Clone)]
pub struct HorizonState {
    pub batches: HashMap<String, BatchProgress>,
    pub failed_jobs: Vec<FailedJob>,
    pub metrics: HashMap<String, QueueMetrics>,
}

/// Builder for Horizon
pub struct HorizonBuilder {
    config: HorizonConfig,
    queue_manager: Option<QueueManager>,
}

impl HorizonBuilder {
    /// Create a new Horizon builder
    pub fn new() -> Self {
        Self {
            config: HorizonConfig::default(),
            queue_manager: None,
        }
    }

    /// Set the queue manager
    pub fn queue_manager(mut self, manager: QueueManager) -> Self {
        self.queue_manager = Some(manager);
        self
    }

    /// Monitor a specific queue
    pub fn monitor_queue(mut self, queue_name: impl Into<String>) -> Self {
        self.config.monitored_queues.push(queue_name.into());
        self
    }

    /// Set failed job retention period
    pub fn failed_job_retention_days(mut self, days: u32) -> Self {
        self.config.failed_job_retention_days = days;
        self
    }

    /// Set metrics retention period
    pub fn metrics_retention_hours(mut self, hours: u32) -> Self {
        self.config.metrics_retention_hours = hours;
        self
    }

    /// Enable or disable dashboard
    pub fn enable_dashboard(mut self, enable: bool) -> Self {
        self.config.enable_dashboard = enable;
        self
    }

    /// Build the Horizon instance
    pub fn build(self) -> Horizon {
        let collector = self
            .queue_manager
            .as_ref()
            .map(|qm| MetricsCollector::new(qm.clone(), self.config.monitored_queues.clone()));

        Horizon {
            config: Arc::new(self.config),
            state: Arc::new(RwLock::new(HorizonState::default())),
            collector,
        }
    }
}

impl Default for HorizonBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Horizon {
    /// Create a new Horizon builder
    pub fn builder() -> HorizonBuilder {
        HorizonBuilder::new()
    }

    /// Create a new Horizon instance with default config
    pub fn new() -> Self {
        Self {
            config: Arc::new(HorizonConfig::default()),
            state: Arc::new(RwLock::new(HorizonState::default())),
            collector: None,
        }
    }

    /// Start the Horizon dashboard server
    pub async fn serve(self, addr: &str) -> Result<()> {
        dashboard::serve(self, addr).await
    }

    /// Start the Horizon dashboard with a queue manager
    pub async fn serve_with_queue_manager(
        self,
        queue_manager: QueueManager,
        addr: &str,
    ) -> Result<()> {
        // Create metrics collector if not already created
        let collector = self.collector.clone().unwrap_or_else(|| {
            MetricsCollector::new(queue_manager.clone(), self.config.monitored_queues.clone())
        });

        // Start metrics collection in background
        collector.clone().start().await;

        // Create app state
        let app_state = AppState::new(self, queue_manager);

        // Build router with routes
        let app = routes(app_state);

        // Start server
        let listener = tokio::net::TcpListener::bind(addr).await?;
        println!("Horizon dashboard running on http://{}", addr);

        axum::serve(listener, app).await?;
        Ok(())
    }

    /// Get current state snapshot
    pub async fn state(&self) -> HorizonState {
        self.state.read().await.clone()
    }

    /// Record a batch
    pub async fn record_batch(&self, batch_id: String, progress: BatchProgress) {
        let mut state = self.state.write().await;
        state.batches.insert(batch_id, progress);
    }

    /// Record a failed job
    pub async fn record_failed_job(&self, failed_job: FailedJob) {
        let mut state = self.state.write().await;
        state.failed_jobs.push(failed_job);
    }

    /// Update queue metrics
    pub async fn update_metrics(&self, queue_name: String, metrics: QueueMetrics) {
        let mut state = self.state.write().await;
        state.metrics.insert(queue_name, metrics);
    }

    /// Get metrics collector
    pub fn collector(&self) -> Option<&MetricsCollector> {
        self.collector.as_ref()
    }
}

impl Default for Horizon {
    fn default() -> Self {
        Self::new()
    }
}
