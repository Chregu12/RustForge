//! Supervisor management for controlling worker processes

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::balancer::BalanceStrategy;
use crate::workers::{WorkerProcess, WorkerRegistry};

/// Supervisor configuration and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supervisor {
    pub name: String,
    pub queue: String,
    pub balance: BalanceStrategy,
    pub min_processes: u32,
    pub max_processes: u32,
    pub balance_cooldown: u32,  // Seconds between balance checks
    pub balance_max_shift: u32, // Max workers to add/remove per balance
    pub timeout: u32,           // Job timeout in seconds
    pub memory: u32,            // Memory limit per worker in MB
    pub tries: u32,             // Max retry attempts
    pub nice: i32,              // Process priority (-20 to 19)
    pub status: SupervisorStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub last_balanced_at: Option<DateTime<Utc>>,
}

/// Supervisor status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SupervisorStatus {
    Running,
    Paused,
    Stopped,
}

impl Supervisor {
    /// Create a new supervisor
    pub fn new(name: impl Into<String>, queue: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            queue: queue.into(),
            balance: BalanceStrategy::Simple,
            min_processes: 1,
            max_processes: 10,
            balance_cooldown: 30,
            balance_max_shift: 2,
            timeout: 60,
            memory: 128,
            tries: 3,
            nice: 0,
            status: SupervisorStatus::Stopped,
            started_at: None,
            last_balanced_at: None,
        }
    }

    /// Set balance strategy
    pub fn balance_strategy(mut self, strategy: BalanceStrategy) -> Self {
        self.balance = strategy;
        self
    }

    /// Set min processes
    pub fn min_processes(mut self, min: u32) -> Self {
        self.min_processes = min;
        self
    }

    /// Set max processes
    pub fn max_processes(mut self, max: u32) -> Self {
        self.max_processes = max;
        self
    }

    /// Set balance cooldown
    pub fn balance_cooldown(mut self, seconds: u32) -> Self {
        self.balance_cooldown = seconds;
        self
    }

    /// Set timeout
    pub fn timeout(mut self, seconds: u32) -> Self {
        self.timeout = seconds;
        self
    }

    /// Set memory limit
    pub fn memory(mut self, mb: u32) -> Self {
        self.memory = mb;
        self
    }

    /// Set max tries
    pub fn tries(mut self, tries: u32) -> Self {
        self.tries = tries;
        self
    }

    /// Set nice value
    pub fn nice(mut self, nice: i32) -> Self {
        self.nice = nice.clamp(-20, 19);
        self
    }

    /// Start the supervisor
    pub fn start(&mut self) {
        self.status = SupervisorStatus::Running;
        self.started_at = Some(Utc::now());
    }

    /// Pause the supervisor
    pub fn pause(&mut self) {
        self.status = SupervisorStatus::Paused;
    }

    /// Continue (resume) the supervisor
    pub fn continue_(&mut self) {
        self.status = SupervisorStatus::Running;
    }

    /// Terminate the supervisor
    pub fn terminate(&mut self) {
        self.status = SupervisorStatus::Stopped;
    }

    /// Check if supervisor is running
    pub fn is_running(&self) -> bool {
        self.status == SupervisorStatus::Running
    }

    /// Check if supervisor is paused
    pub fn is_paused(&self) -> bool {
        self.status == SupervisorStatus::Paused
    }

    /// Check if supervisor is stopped
    pub fn is_stopped(&self) -> bool {
        self.status == SupervisorStatus::Stopped
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> Option<i64> {
        self.started_at
            .map(|start| (Utc::now() - start).num_seconds())
    }

    /// Check if cooldown period has elapsed
    pub fn can_balance(&self) -> bool {
        if let Some(last_balanced) = self.last_balanced_at {
            let elapsed = (Utc::now() - last_balanced).num_seconds();
            elapsed >= self.balance_cooldown as i64
        } else {
            true
        }
    }

    /// Mark that balancing occurred
    pub fn mark_balanced(&mut self) {
        self.last_balanced_at = Some(Utc::now());
    }
}

/// Supervisor manager for controlling multiple supervisors
pub struct SupervisorManager {
    supervisors: Arc<RwLock<HashMap<String, Supervisor>>>,
    worker_registry: WorkerRegistry,
}

impl SupervisorManager {
    /// Create a new supervisor manager
    pub fn new(worker_registry: WorkerRegistry) -> Self {
        Self {
            supervisors: Arc::new(RwLock::new(HashMap::new())),
            worker_registry,
        }
    }

    /// Add a supervisor
    pub async fn add(&self, supervisor: Supervisor) {
        self.supervisors
            .write()
            .await
            .insert(supervisor.name.clone(), supervisor);
    }

    /// Remove a supervisor
    pub async fn remove(&self, name: &str) -> Option<Supervisor> {
        self.supervisors.write().await.remove(name)
    }

    /// Get a supervisor
    pub async fn get(&self, name: &str) -> Option<Supervisor> {
        self.supervisors.read().await.get(name).cloned()
    }

    /// Get all supervisors
    pub async fn all(&self) -> Vec<Supervisor> {
        self.supervisors.read().await.values().cloned().collect()
    }

    /// Get supervisors by queue
    pub async fn by_queue(&self, queue: &str) -> Vec<Supervisor> {
        self.supervisors
            .read()
            .await
            .values()
            .filter(|s| s.queue == queue)
            .cloned()
            .collect()
    }

    /// Get running supervisors
    pub async fn running(&self) -> Vec<Supervisor> {
        self.supervisors
            .read()
            .await
            .values()
            .filter(|s| s.is_running())
            .cloned()
            .collect()
    }

    /// Scale workers for a supervisor
    pub async fn scale(&self, name: &str, count: u32) -> Result<()> {
        let mut supervisors = self.supervisors.write().await;
        let supervisor = supervisors
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Supervisor {} not found", name))?;

        // Clamp to min/max
        let target_count = count.max(supervisor.min_processes).min(supervisor.max_processes);

        // Get current workers for this supervisor
        let current_workers = self.worker_registry.by_supervisor(name).await;
        let current_count = current_workers.len() as u32;

        if target_count > current_count {
            // Add workers
            let to_add = target_count - current_count;
            for i in 0..to_add {
                let worker_id = format!("{}-worker-{}", name, current_count + i);
                let worker = WorkerProcess::new(
                    worker_id,
                    format!("Worker {}", current_count + i),
                    name,
                    &supervisor.queue,
                );
                self.worker_registry.register(worker).await;
            }
        } else if target_count < current_count {
            // Remove workers (remove idle ones first)
            let to_remove = current_count - target_count;
            let mut removed = 0;

            // First try to remove idle workers
            for worker in current_workers.iter() {
                if removed >= to_remove {
                    break;
                }
                if worker.status == crate::workers::WorkerProcessStatus::Idle {
                    self.worker_registry.unregister(&worker.id).await;
                    removed += 1;
                }
            }

            // If we still need to remove more, remove any worker
            if removed < to_remove {
                for worker in current_workers.iter() {
                    if removed >= to_remove {
                        break;
                    }
                    self.worker_registry.unregister(&worker.id).await;
                    removed += 1;
                }
            }
        }

        Ok(())
    }

    /// Start a supervisor
    pub async fn start(&self, name: &str) -> Result<()> {
        let mut supervisors = self.supervisors.write().await;
        let supervisor = supervisors
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Supervisor {} not found", name))?;

        supervisor.start();

        // Resume all workers for this supervisor
        let workers = self.worker_registry.by_supervisor(name).await;
        for worker in workers {
            self.worker_registry.resume_worker(&worker.id).await;
        }

        Ok(())
    }

    /// Pause a supervisor
    pub async fn pause(&self, name: &str) -> Result<()> {
        let mut supervisors = self.supervisors.write().await;
        let supervisor = supervisors
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Supervisor {} not found", name))?;

        supervisor.pause();

        // Pause all workers for this supervisor
        let workers = self.worker_registry.by_supervisor(name).await;
        for worker in workers {
            self.worker_registry.pause_worker(&worker.id).await;
        }

        Ok(())
    }

    /// Continue (resume) a supervisor
    pub async fn continue_(&self, name: &str) -> Result<()> {
        let mut supervisors = self.supervisors.write().await;
        let supervisor = supervisors
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Supervisor {} not found", name))?;

        supervisor.continue_();

        // Resume all workers for this supervisor
        let workers = self.worker_registry.by_supervisor(name).await;
        for worker in workers {
            self.worker_registry.resume_worker(&worker.id).await;
        }

        Ok(())
    }

    /// Terminate a supervisor
    pub async fn terminate(&self, name: &str) -> Result<()> {
        let mut supervisors = self.supervisors.write().await;
        let supervisor = supervisors
            .get_mut(name)
            .ok_or_else(|| anyhow::anyhow!("Supervisor {} not found", name))?;

        supervisor.terminate();

        // Stop all workers for this supervisor
        let workers = self.worker_registry.by_supervisor(name).await;
        for worker in workers {
            self.worker_registry.stop_worker(&worker.id).await;
        }

        Ok(())
    }

    /// Pause all supervisors
    pub async fn pause_all(&self) -> Result<()> {
        let names: Vec<String> = self
            .supervisors
            .read()
            .await
            .keys()
            .cloned()
            .collect();

        for name in names {
            self.pause(&name).await?;
        }

        Ok(())
    }

    /// Continue all supervisors
    pub async fn continue_all(&self) -> Result<()> {
        let names: Vec<String> = self
            .supervisors
            .read()
            .await
            .keys()
            .cloned()
            .collect();

        for name in names {
            self.continue_(&name).await?;
        }

        Ok(())
    }

    /// Terminate all supervisors
    pub async fn terminate_all(&self) -> Result<()> {
        let names: Vec<String> = self
            .supervisors
            .read()
            .await
            .keys()
            .cloned()
            .collect();

        for name in names {
            self.terminate(&name).await?;
        }

        Ok(())
    }

    /// Get supervisor count
    pub async fn count(&self) -> usize {
        self.supervisors.read().await.len()
    }

    /// Clear all supervisors
    pub async fn clear(&self) {
        self.supervisors.write().await.clear();
    }
}

impl Clone for SupervisorManager {
    fn clone(&self) -> Self {
        Self {
            supervisors: Arc::clone(&self.supervisors),
            worker_registry: self.worker_registry.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supervisor_new() {
        let supervisor = Supervisor::new("supervisor-1", "default");
        assert_eq!(supervisor.name, "supervisor-1");
        assert_eq!(supervisor.queue, "default");
        assert_eq!(supervisor.status, SupervisorStatus::Stopped);
    }

    #[test]
    fn test_supervisor_builder() {
        let supervisor = Supervisor::new("supervisor-1", "default")
            .min_processes(2)
            .max_processes(20)
            .timeout(120)
            .memory(256)
            .tries(5)
            .nice(10);

        assert_eq!(supervisor.min_processes, 2);
        assert_eq!(supervisor.max_processes, 20);
        assert_eq!(supervisor.timeout, 120);
        assert_eq!(supervisor.memory, 256);
        assert_eq!(supervisor.tries, 5);
        assert_eq!(supervisor.nice, 10);
    }

    #[test]
    fn test_supervisor_start() {
        let mut supervisor = Supervisor::new("supervisor-1", "default");
        assert!(supervisor.is_stopped());

        supervisor.start();
        assert!(supervisor.is_running());
        assert!(supervisor.started_at.is_some());
    }

    #[test]
    fn test_supervisor_pause() {
        let mut supervisor = Supervisor::new("supervisor-1", "default");
        supervisor.start();
        supervisor.pause();

        assert!(supervisor.is_paused());
    }

    #[test]
    fn test_supervisor_continue() {
        let mut supervisor = Supervisor::new("supervisor-1", "default");
        supervisor.start();
        supervisor.pause();
        supervisor.continue_();

        assert!(supervisor.is_running());
    }

    #[test]
    fn test_supervisor_nice_clamping() {
        let supervisor = Supervisor::new("supervisor-1", "default").nice(100);
        assert_eq!(supervisor.nice, 19);

        let supervisor = Supervisor::new("supervisor-1", "default").nice(-100);
        assert_eq!(supervisor.nice, -20);
    }

    #[test]
    fn test_supervisor_can_balance() {
        let mut supervisor = Supervisor::new("supervisor-1", "default");
        supervisor.balance_cooldown = 30;

        // Should be able to balance initially
        assert!(supervisor.can_balance());

        // After marking balanced, should not be able to balance immediately
        supervisor.mark_balanced();
        assert!(!supervisor.can_balance());
    }

    #[tokio::test]
    async fn test_supervisor_manager_add() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry);

        let supervisor = Supervisor::new("supervisor-1", "default");
        manager.add(supervisor).await;

        assert_eq!(manager.count().await, 1);
    }

    #[tokio::test]
    async fn test_supervisor_manager_get() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry);

        let supervisor = Supervisor::new("supervisor-1", "default");
        manager.add(supervisor).await;

        let retrieved = manager.get("supervisor-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "supervisor-1");
    }

    #[tokio::test]
    async fn test_supervisor_manager_by_queue() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry);

        manager.add(Supervisor::new("s1", "emails")).await;
        manager.add(Supervisor::new("s2", "default")).await;
        manager.add(Supervisor::new("s3", "emails")).await;

        let email_supervisors = manager.by_queue("emails").await;
        assert_eq!(email_supervisors.len(), 2);
    }

    #[tokio::test]
    async fn test_supervisor_manager_start() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry);

        let supervisor = Supervisor::new("supervisor-1", "default");
        manager.add(supervisor).await;

        manager.start("supervisor-1").await.unwrap();

        let supervisor = manager.get("supervisor-1").await.unwrap();
        assert!(supervisor.is_running());
    }

    #[tokio::test]
    async fn test_supervisor_manager_pause() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry);

        let mut supervisor = Supervisor::new("supervisor-1", "default");
        supervisor.start();
        manager.add(supervisor).await;

        manager.pause("supervisor-1").await.unwrap();

        let supervisor = manager.get("supervisor-1").await.unwrap();
        assert!(supervisor.is_paused());
    }

    #[tokio::test]
    async fn test_supervisor_manager_scale() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry.clone());

        let supervisor = Supervisor::new("supervisor-1", "default")
            .min_processes(1)
            .max_processes(10);
        manager.add(supervisor).await;

        // Scale to 5 workers
        manager.scale("supervisor-1", 5).await.unwrap();

        let workers = worker_registry.by_supervisor("supervisor-1").await;
        assert_eq!(workers.len(), 5);
    }

    #[tokio::test]
    async fn test_supervisor_manager_scale_down() {
        let worker_registry = WorkerRegistry::new();
        let manager = SupervisorManager::new(worker_registry.clone());

        let supervisor = Supervisor::new("supervisor-1", "default")
            .min_processes(1)
            .max_processes(10);
        manager.add(supervisor).await;

        // Scale up to 5
        manager.scale("supervisor-1", 5).await.unwrap();
        assert_eq!(worker_registry.by_supervisor("supervisor-1").await.len(), 5);

        // Scale down to 2
        manager.scale("supervisor-1", 2).await.unwrap();
        assert_eq!(worker_registry.by_supervisor("supervisor-1").await.len(), 2);
    }
}
