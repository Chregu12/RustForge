//! Horizon facade for easy access to core functionality

use anyhow::Result;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{
    metrics::QueueMetrics,
    recent_jobs::{RecentJobsStats, RecentJobsStore},
    snapshots::SnapshotStore,
    supervisor::{Supervisor, SupervisorManager},
    tags::TagRegistry,
    workers::WorkerRegistry,
};

/// Horizon status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HorizonStatus {
    pub is_paused: bool,
    pub total_supervisors: usize,
    pub running_supervisors: usize,
    pub total_workers: usize,
    pub active_workers: usize,
    pub recent_jobs: usize,
    pub total_processed: u64,
    pub total_failed: u64,
}

/// Global Horizon facade instance
static HORIZON_INSTANCE: once_cell::sync::Lazy<Arc<RwLock<Option<HorizonFacadeInstance>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

/// Internal Horizon facade instance
struct HorizonFacadeInstance {
    supervisor_manager: SupervisorManager,
    worker_registry: WorkerRegistry,
    tag_registry: TagRegistry,
    snapshot_store: SnapshotStore,
    recent_jobs_store: RecentJobsStore,
    paused: bool,
}

impl HorizonFacadeInstance {
    fn new() -> Self {
        let worker_registry = WorkerRegistry::new();
        let supervisor_manager = SupervisorManager::new(worker_registry.clone());

        Self {
            supervisor_manager,
            worker_registry,
            tag_registry: TagRegistry::new(),
            snapshot_store: SnapshotStore::with_default_capacity(),
            recent_jobs_store: RecentJobsStore::with_default_capacity(),
            paused: false,
        }
    }
}

/// Horizon facade for static access
pub struct HorizonFacade;

impl HorizonFacade {
    /// Initialize the Horizon facade
    pub async fn init() {
        let mut instance = HORIZON_INSTANCE.write().await;
        if instance.is_none() {
            *instance = Some(HorizonFacadeInstance::new());
        }
    }

    /// Ensure instance is initialized
    async fn ensure_init() {
        let instance = HORIZON_INSTANCE.read().await;
        if instance.is_none() {
            drop(instance);
            Self::init().await;
        }
    }

    /// Pause all queues
    pub async fn pause() -> Result<()> {
        Self::ensure_init().await;
        let mut instance = HORIZON_INSTANCE.write().await;

        if let Some(ref mut horizon) = *instance {
            horizon.paused = true;
            horizon.supervisor_manager.pause_all().await?;
            horizon.worker_registry.pause_all().await;
        }

        Ok(())
    }

    /// Resume all queues
    pub async fn continue_() -> Result<()> {
        Self::ensure_init().await;
        let mut instance = HORIZON_INSTANCE.write().await;

        if let Some(ref mut horizon) = *instance {
            horizon.paused = false;
            horizon.supervisor_manager.continue_all().await?;
            horizon.worker_registry.resume_all().await;
        }

        Ok(())
    }

    /// Get current status
    pub async fn status() -> HorizonStatus {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            let total_supervisors = horizon.supervisor_manager.count().await;
            let running_supervisors = horizon.supervisor_manager.running().await.len();
            let total_workers = horizon.worker_registry.count().await;
            let active_workers = horizon.worker_registry.active().await.len();
            let recent_jobs = horizon.recent_jobs_store.count().await;
            let total_processed = horizon.worker_registry.total_jobs_processed().await;
            let total_failed = horizon.worker_registry.total_jobs_failed().await;

            HorizonStatus {
                is_paused: horizon.paused,
                total_supervisors,
                running_supervisors,
                total_workers,
                active_workers,
                recent_jobs,
                total_processed,
                total_failed,
            }
        } else {
            HorizonStatus {
                is_paused: false,
                total_supervisors: 0,
                running_supervisors: 0,
                total_workers: 0,
                active_workers: 0,
                recent_jobs: 0,
                total_processed: 0,
                total_failed: 0,
            }
        }
    }

    /// Check if paused
    pub async fn is_paused() -> bool {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.paused
        } else {
            false
        }
    }

    /// Get metrics for a queue (placeholder - would integrate with actual metrics)
    pub async fn queue_metrics(queue: &str) -> Option<QueueMetrics> {
        Self::ensure_init().await;
        // In a real implementation, this would fetch from the metrics system
        // For now, return a placeholder
        Some(QueueMetrics::new(queue))
    }

    /// Get all supervisors
    pub async fn supervisors() -> Vec<Supervisor> {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.supervisor_manager.all().await
        } else {
            Vec::new()
        }
    }

    /// Add a supervisor
    pub async fn add_supervisor(supervisor: Supervisor) -> Result<()> {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.supervisor_manager.add(supervisor).await;
        }

        Ok(())
    }

    /// Start a supervisor
    pub async fn start_supervisor(name: &str) -> Result<()> {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.supervisor_manager.start(name).await?;
        }

        Ok(())
    }

    /// Pause a supervisor
    pub async fn pause_supervisor(name: &str) -> Result<()> {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.supervisor_manager.pause(name).await?;
        }

        Ok(())
    }

    /// Scale a supervisor
    pub async fn scale_supervisor(name: &str, count: u32) -> Result<()> {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.supervisor_manager.scale(name, count).await?;
        }

        Ok(())
    }

    /// Get worker registry
    pub async fn worker_registry() -> WorkerRegistry {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.worker_registry.clone()
        } else {
            WorkerRegistry::new()
        }
    }

    /// Get tag registry
    pub async fn tag_registry() -> TagRegistry {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.tag_registry.clone()
        } else {
            TagRegistry::new()
        }
    }

    /// Get snapshot store
    pub async fn snapshot_store() -> SnapshotStore {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.snapshot_store.clone()
        } else {
            SnapshotStore::with_default_capacity()
        }
    }

    /// Get recent jobs store
    pub async fn recent_jobs_store() -> RecentJobsStore {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.recent_jobs_store.clone()
        } else {
            RecentJobsStore::with_default_capacity()
        }
    }

    /// Get recent jobs statistics
    pub async fn recent_jobs_stats() -> RecentJobsStats {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.recent_jobs_store.stats().await
        } else {
            RecentJobsStats {
                total: 0,
                pending: 0,
                completed: 0,
                failed: 0,
                avg_runtime: 0.0,
                success_rate: 1.0,
            }
        }
    }

    /// Clear all metrics (for testing)
    pub async fn clear_metrics() -> Result<()> {
        Self::ensure_init().await;
        let instance = HORIZON_INSTANCE.read().await;

        if let Some(ref horizon) = *instance {
            horizon.snapshot_store.clear().await;
            horizon.recent_jobs_store.clear().await;
        }

        Ok(())
    }

    /// Reset the facade (for testing)
    pub async fn reset() {
        let mut instance = HORIZON_INSTANCE.write().await;
        *instance = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_horizon_facade_init() {
        // Reset to clear any previous test state
        HorizonFacade::reset().await;

        // Now init fresh
        HorizonFacade::init().await;

        let status = HorizonFacade::status().await;
        // Note: In a parallel test environment, other tests might have added supervisors
        // so we just check it's initialized
        assert!(!status.is_paused);
    }

    #[tokio::test]
    async fn test_horizon_facade_pause_continue() {
        HorizonFacade::reset().await;
        HorizonFacade::init().await;

        HorizonFacade::pause().await.unwrap();
        assert!(HorizonFacade::is_paused().await);

        HorizonFacade::continue_().await.unwrap();
        assert!(!HorizonFacade::is_paused().await);
    }

    #[tokio::test]
    async fn test_horizon_facade_add_supervisor() {
        HorizonFacade::reset().await;
        HorizonFacade::init().await;

        let supervisor = Supervisor::new("test-supervisor", "default");
        HorizonFacade::add_supervisor(supervisor).await.unwrap();

        let status = HorizonFacade::status().await;
        assert_eq!(status.total_supervisors, 1);
    }

    #[tokio::test]
    async fn test_horizon_facade_supervisors() {
        HorizonFacade::reset().await;
        HorizonFacade::init().await;

        let supervisor = Supervisor::new("test-supervisor", "default");
        HorizonFacade::add_supervisor(supervisor).await.unwrap();

        let supervisors = HorizonFacade::supervisors().await;
        assert_eq!(supervisors.len(), 1);
        assert_eq!(supervisors[0].name, "test-supervisor");
    }

    #[tokio::test]
    async fn test_horizon_facade_worker_registry() {
        HorizonFacade::reset().await;
        HorizonFacade::init().await;

        let registry = HorizonFacade::worker_registry().await;
        assert_eq!(registry.count().await, 0);
    }

    #[tokio::test]
    async fn test_horizon_facade_recent_jobs_stats() {
        HorizonFacade::reset().await;
        HorizonFacade::init().await;

        let stats = HorizonFacade::recent_jobs_stats().await;
        assert_eq!(stats.total, 0);
        assert_eq!(stats.success_rate, 1.0);
    }

    #[tokio::test]
    async fn test_horizon_facade_clear_metrics() {
        HorizonFacade::reset().await;
        HorizonFacade::init().await;

        HorizonFacade::clear_metrics().await.unwrap();

        let stats = HorizonFacade::recent_jobs_stats().await;
        assert_eq!(stats.total, 0);
    }
}
