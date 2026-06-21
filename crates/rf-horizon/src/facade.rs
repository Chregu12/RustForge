//! Horizon facade for easy access to core functionality
//!
//! This facade provides a synchronous public API that wraps async functionality internally.

use anyhow::Result;
use rf_core::runtime::block_on;
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
    pub fn init() {
        block_on(async {
            let mut instance = HORIZON_INSTANCE.write().await;
            if instance.is_none() {
                *instance = Some(HorizonFacadeInstance::new());
            }
        })
    }

    /// Ensure instance is initialized (internal async helper)
    async fn ensure_init_async() {
        let instance = HORIZON_INSTANCE.read().await;
        if instance.is_none() {
            drop(instance);
            let mut instance = HORIZON_INSTANCE.write().await;
            if instance.is_none() {
                *instance = Some(HorizonFacadeInstance::new());
            }
        }
    }

    /// Pause all queues
    pub fn pause() -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let mut instance = HORIZON_INSTANCE.write().await;

            if let Some(ref mut horizon) = *instance {
                horizon.paused = true;
                horizon.supervisor_manager.pause_all().await?;
                horizon.worker_registry.pause_all().await;
            }

            Ok(())
        })
    }

    /// Resume all queues
    pub fn continue_() -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let mut instance = HORIZON_INSTANCE.write().await;

            if let Some(ref mut horizon) = *instance {
                horizon.paused = false;
                horizon.supervisor_manager.continue_all().await?;
                horizon.worker_registry.resume_all().await;
            }

            Ok(())
        })
    }

    /// Get current status
    pub fn status() -> HorizonStatus {
        block_on(async {
            Self::ensure_init_async().await;
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
        })
    }

    /// Check if paused
    pub fn is_paused() -> bool {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.paused
            } else {
                false
            }
        })
    }

    /// Get metrics for a queue (placeholder - would integrate with actual metrics)
    pub fn queue_metrics(queue: &str) -> Option<QueueMetrics> {
        block_on(async {
            Self::ensure_init_async().await;
            // In a real implementation, this would fetch from the metrics system
            // For now, return a placeholder
            Some(QueueMetrics::new(queue))
        })
    }

    /// Get all supervisors
    pub fn supervisors() -> Vec<Supervisor> {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.supervisor_manager.all().await
            } else {
                Vec::new()
            }
        })
    }

    /// Add a supervisor
    pub fn add_supervisor(supervisor: Supervisor) -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.supervisor_manager.add(supervisor).await;
            }

            Ok(())
        })
    }

    /// Start a supervisor
    pub fn start_supervisor(name: &str) -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.supervisor_manager.start(name).await?;
            }

            Ok(())
        })
    }

    /// Pause a supervisor
    pub fn pause_supervisor(name: &str) -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.supervisor_manager.pause(name).await?;
            }

            Ok(())
        })
    }

    /// Scale a supervisor
    pub fn scale_supervisor(name: &str, count: u32) -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.supervisor_manager.scale(name, count).await?;
            }

            Ok(())
        })
    }

    /// Get worker registry
    pub fn worker_registry() -> WorkerRegistry {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.worker_registry.clone()
            } else {
                WorkerRegistry::new()
            }
        })
    }

    /// Get tag registry
    pub fn tag_registry() -> TagRegistry {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.tag_registry.clone()
            } else {
                TagRegistry::new()
            }
        })
    }

    /// Get snapshot store
    pub fn snapshot_store() -> SnapshotStore {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.snapshot_store.clone()
            } else {
                SnapshotStore::with_default_capacity()
            }
        })
    }

    /// Get recent jobs store
    pub fn recent_jobs_store() -> RecentJobsStore {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.recent_jobs_store.clone()
            } else {
                RecentJobsStore::with_default_capacity()
            }
        })
    }

    /// Get recent jobs statistics
    pub fn recent_jobs_stats() -> RecentJobsStats {
        block_on(async {
            Self::ensure_init_async().await;
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
        })
    }

    /// Clear all metrics (for testing)
    pub fn clear_metrics() -> Result<()> {
        block_on(async {
            Self::ensure_init_async().await;
            let instance = HORIZON_INSTANCE.read().await;

            if let Some(ref horizon) = *instance {
                horizon.snapshot_store.clear().await;
                horizon.recent_jobs_store.clear().await;
            }

            Ok(())
        })
    }

    /// Reset the facade (for testing)
    pub fn reset() {
        block_on(async {
            let mut instance = HORIZON_INSTANCE.write().await;
            *instance = None;
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Serializes every test in this module: they all `reset()`/`init()` the
    /// process-global `HORIZON_INSTANCE`, so running them in parallel races
    /// (one test's `reset()` wipes another's supervisors mid-assertion).
    /// `into_inner` ignores poisoning so one failure doesn't cascade.
    static HORIZON_TEST_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_horizon_facade_init() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        // Reset to clear any previous test state
        HorizonFacade::reset();

        // Now init fresh
        HorizonFacade::init();

        let status = HorizonFacade::status();
        // Note: In a parallel test environment, other tests might have added supervisors
        // so we just check it's initialized
        assert!(!status.is_paused);
    }

    #[test]
    fn test_horizon_facade_pause_continue() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        HorizonFacade::reset();
        HorizonFacade::init();

        HorizonFacade::pause().unwrap();
        assert!(HorizonFacade::is_paused());

        HorizonFacade::continue_().unwrap();
        assert!(!HorizonFacade::is_paused());
    }

    #[test]
    fn test_horizon_facade_add_supervisor() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        HorizonFacade::reset();
        HorizonFacade::init();

        let supervisor = Supervisor::new("test-supervisor", "default");
        HorizonFacade::add_supervisor(supervisor).unwrap();

        let status = HorizonFacade::status();
        assert_eq!(status.total_supervisors, 1);
    }

    #[test]
    fn test_horizon_facade_supervisors() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        HorizonFacade::reset();
        HorizonFacade::init();

        let supervisor = Supervisor::new("test-supervisor", "default");
        HorizonFacade::add_supervisor(supervisor).unwrap();

        let supervisors = HorizonFacade::supervisors();
        assert_eq!(supervisors.len(), 1);
        assert_eq!(supervisors[0].name, "test-supervisor");
    }

    #[test]
    fn test_horizon_facade_worker_registry() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        HorizonFacade::reset();
        HorizonFacade::init();

        let registry = HorizonFacade::worker_registry();
        block_on(async {
            assert_eq!(registry.count().await, 0);
        });
    }

    #[test]
    fn test_horizon_facade_recent_jobs_stats() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        HorizonFacade::reset();
        HorizonFacade::init();

        let stats = HorizonFacade::recent_jobs_stats();
        assert_eq!(stats.total, 0);
        assert_eq!(stats.success_rate, 1.0);
    }

    #[test]
    fn test_horizon_facade_clear_metrics() {
        let _guard = HORIZON_TEST_GUARD.lock().unwrap_or_else(|e| e.into_inner());
        HorizonFacade::reset();
        HorizonFacade::init();

        HorizonFacade::clear_metrics().unwrap();

        let stats = HorizonFacade::recent_jobs_stats();
        assert_eq!(stats.total, 0);
    }
}
