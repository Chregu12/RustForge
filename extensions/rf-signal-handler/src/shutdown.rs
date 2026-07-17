//! Graceful shutdown management

use crate::callback::CallbackCollection;
use crate::error::{SignalError, SignalResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};

/// Shutdown phases for orderly cleanup
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ShutdownPhase {
    /// Pre-shutdown preparation
    PreShutdown,
    /// Main shutdown phase
    Shutdown,
    /// Post-shutdown cleanup
    PostShutdown,
}

impl ShutdownPhase {
    /// Get all phases in order
    pub fn all() -> Vec<ShutdownPhase> {
        vec![
            ShutdownPhase::PreShutdown,
            ShutdownPhase::Shutdown,
            ShutdownPhase::PostShutdown,
        ]
    }

    /// Get phase name
    pub fn name(&self) -> &'static str {
        match self {
            ShutdownPhase::PreShutdown => "pre-shutdown",
            ShutdownPhase::Shutdown => "shutdown",
            ShutdownPhase::PostShutdown => "post-shutdown",
        }
    }
}

/// Manages graceful shutdown process
pub struct ShutdownManager {
    shutdown_flag: Arc<AtomicBool>,
    phases: Arc<RwLock<std::collections::HashMap<ShutdownPhase, CallbackCollection>>>,
    exit_code: Arc<RwLock<i32>>,
}

impl ShutdownManager {
    /// Create new shutdown manager
    pub fn new() -> Self {
        let mut phases = std::collections::HashMap::new();
        for phase in ShutdownPhase::all() {
            phases.insert(phase, CallbackCollection::new());
        }

        Self {
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            phases: Arc::new(RwLock::new(phases)),
            exit_code: Arc::new(RwLock::new(0)),
        }
    }

    /// Register cleanup callback for a specific phase
    pub async fn register_cleanup(
        &self,
        phase: ShutdownPhase,
        callback: crate::callback::SignalCallback,
    ) -> SignalResult<()> {
        let mut phases = self.phases.write().await;
        if let Some(collection) = phases.get_mut(&phase) {
            collection.add(callback);
            Ok(())
        } else {
            Err(SignalError::Other(anyhow::anyhow!(
                "Invalid shutdown phase"
            )))
        }
    }

    /// Check if shutdown has been initiated
    pub fn is_shutting_down(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Initiate shutdown process
    pub async fn shutdown(&self) -> SignalResult<i32> {
        if self
            .shutdown_flag
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("Shutdown already in progress");
            return Ok(*self.exit_code.read().await);
        }

        info!("Initiating graceful shutdown");

        for phase in ShutdownPhase::all() {
            self.execute_phase(phase).await?;
        }

        let exit_code = *self.exit_code.read().await;
        info!("Shutdown completed with exit code: {}", exit_code);
        Ok(exit_code)
    }

    /// Execute shutdown phase
    async fn execute_phase(&self, phase: ShutdownPhase) -> SignalResult<()> {
        info!("Executing shutdown phase: {}", phase.name());

        let phases = self.phases.read().await;
        if let Some(collection) = phases.get(&phase) {
            if let Err(e) = collection.execute_all().await {
                error!("Error during {} phase: {}", phase.name(), e);
                let mut exit_code = self.exit_code.write().await;
                *exit_code = 1;
                return Err(SignalError::ShutdownFailed(format!(
                    "Phase {} failed: {}",
                    phase.name(),
                    e
                )));
            }
        }

        Ok(())
    }

    /// Set exit code
    pub async fn set_exit_code(&self, code: i32) {
        let mut exit_code = self.exit_code.write().await;
        *exit_code = code;
    }

    /// Get exit code
    pub async fn get_exit_code(&self) -> i32 {
        *self.exit_code.read().await
    }

    /// Get shutdown flag for external monitoring
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }
}

impl Default for ShutdownManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::callback::SignalCallback;

    #[tokio::test]
    async fn test_shutdown_manager_creation() {
        let manager = ShutdownManager::new();
        assert!(!manager.is_shutting_down());
        assert_eq!(manager.get_exit_code().await, 0);
    }

    #[tokio::test]
    async fn test_shutdown_phases() {
        let manager = ShutdownManager::new();
        let executed = Arc::new(std::sync::Mutex::new(Vec::new()));

        for phase in ShutdownPhase::all() {
            let executed_clone = executed.clone();
            let phase_name = phase.name().to_string();
            manager
                .register_cleanup(
                    phase,
                    SignalCallback::sync(format!("{}_callback", phase.name()), move || {
                        executed_clone.lock().unwrap().push(phase_name.clone());
                    }),
                )
                .await
                .unwrap();
        }

        manager.shutdown().await.unwrap();

        let execution_order = executed.lock().unwrap();
        assert_eq!(execution_order.len(), 3);
        assert_eq!(execution_order[0], "pre-shutdown");
        assert_eq!(execution_order[1], "shutdown");
        assert_eq!(execution_order[2], "post-shutdown");
    }

    #[tokio::test]
    async fn test_shutdown_flag() {
        let manager = ShutdownManager::new();
        assert!(!manager.is_shutting_down());

        manager.shutdown().await.unwrap();
        assert!(manager.is_shutting_down());
    }

    #[tokio::test]
    async fn test_exit_code() {
        let manager = ShutdownManager::new();
        assert_eq!(manager.get_exit_code().await, 0);

        manager.set_exit_code(42).await;
        assert_eq!(manager.get_exit_code().await, 42);
    }

    #[tokio::test]
    async fn test_multiple_shutdown_calls() {
        let manager = ShutdownManager::new();
        let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let counter_clone = counter.clone();
        manager
            .register_cleanup(
                ShutdownPhase::Shutdown,
                SignalCallback::sync("test", move || {
                    counter_clone.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }),
            )
            .await
            .unwrap();

        // First shutdown
        manager.shutdown().await.unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second shutdown should not execute callbacks again
        manager.shutdown().await.unwrap();
        assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 1);
    }
}
