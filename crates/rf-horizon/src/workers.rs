//! Worker process monitoring and registry

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{Pid, System};
use tokio::sync::RwLock;

/// Worker process status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum WorkerProcessStatus {
    Idle,
    Running,
    Paused,
    Stopped,
}

/// Current job being processed
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentJob {
    pub id: String,
    pub name: String,
    pub started_at: DateTime<Utc>,
    pub progress: Option<u8>, // 0-100
}

impl CurrentJob {
    /// Create a new current job
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            started_at: Utc::now(),
            progress: None,
        }
    }

    /// Set progress
    pub fn with_progress(mut self, progress: u8) -> Self {
        self.progress = Some(progress.min(100));
        self
    }

    /// Update progress
    pub fn set_progress(&mut self, progress: u8) {
        self.progress = Some(progress.min(100));
    }

    /// Get elapsed time in seconds
    pub fn elapsed_seconds(&self) -> i64 {
        (Utc::now() - self.started_at).num_seconds()
    }
}

/// Worker process information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerProcess {
    pub id: String,
    pub name: String,
    pub supervisor: String,
    pub queue: String,
    pub status: WorkerProcessStatus,
    pub current_job: Option<CurrentJob>,
    pub started_at: DateTime<Utc>,
    pub jobs_processed: u64,
    pub jobs_failed: u64,
    pub memory_usage: u64,  // MB
    pub cpu_usage: f32,     // Percentage
    pub pid: Option<u32>,   // Process ID
}

impl WorkerProcess {
    /// Create a new worker process
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        supervisor: impl Into<String>,
        queue: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            supervisor: supervisor.into(),
            queue: queue.into(),
            status: WorkerProcessStatus::Idle,
            current_job: None,
            started_at: Utc::now(),
            jobs_processed: 0,
            jobs_failed: 0,
            memory_usage: 0,
            cpu_usage: 0.0,
            pid: None,
        }
    }

    /// Set process ID
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }

    /// Start processing a job
    pub fn start_job(&mut self, job: CurrentJob) {
        self.current_job = Some(job);
        self.status = WorkerProcessStatus::Running;
    }

    /// Complete current job
    pub fn complete_job(&mut self, success: bool) {
        self.current_job = None;
        self.status = WorkerProcessStatus::Idle;

        if success {
            self.jobs_processed += 1;
        } else {
            self.jobs_failed += 1;
        }
    }

    /// Pause the worker
    pub fn pause(&mut self) {
        self.status = WorkerProcessStatus::Paused;
    }

    /// Resume the worker
    pub fn resume(&mut self) {
        self.status = if self.current_job.is_some() {
            WorkerProcessStatus::Running
        } else {
            WorkerProcessStatus::Idle
        };
    }

    /// Stop the worker
    pub fn stop(&mut self) {
        self.status = WorkerProcessStatus::Stopped;
        self.current_job = None;
    }

    /// Check if worker is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.status,
            WorkerProcessStatus::Idle | WorkerProcessStatus::Running
        )
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        let total = self.jobs_processed + self.jobs_failed;
        if total == 0 {
            return 1.0;
        }
        self.jobs_processed as f64 / total as f64
    }

    /// Update system metrics (memory, CPU)
    pub fn update_system_metrics(&mut self, system: &System) {
        if let Some(pid) = self.pid {
            if let Some(process) = system.process(Pid::from_u32(pid)) {
                // Memory in MB
                self.memory_usage = process.memory() / 1024 / 1024;
                // CPU usage percentage
                self.cpu_usage = process.cpu_usage();
            }
        }
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> i64 {
        (Utc::now() - self.started_at).num_seconds()
    }
}

/// Worker registry for tracking all workers
pub struct WorkerRegistry {
    workers: Arc<RwLock<HashMap<String, WorkerProcess>>>,
    system: Arc<RwLock<System>>,
}

impl WorkerRegistry {
    /// Create a new worker registry
    pub fn new() -> Self {
        Self {
            workers: Arc::new(RwLock::new(HashMap::new())),
            system: Arc::new(RwLock::new(System::new_all())),
        }
    }

    /// Register a worker
    pub async fn register(&self, worker: WorkerProcess) {
        self.workers
            .write()
            .await
            .insert(worker.id.clone(), worker);
    }

    /// Unregister a worker
    pub async fn unregister(&self, worker_id: &str) -> Option<WorkerProcess> {
        self.workers.write().await.remove(worker_id)
    }

    /// Get a worker by ID
    pub async fn get(&self, worker_id: &str) -> Option<WorkerProcess> {
        self.workers.read().await.get(worker_id).cloned()
    }

    /// Get all workers
    pub async fn all(&self) -> Vec<WorkerProcess> {
        self.workers.read().await.values().cloned().collect()
    }

    /// Get workers by queue
    pub async fn by_queue(&self, queue: &str) -> Vec<WorkerProcess> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| w.queue == queue)
            .cloned()
            .collect()
    }

    /// Get workers by supervisor
    pub async fn by_supervisor(&self, supervisor: &str) -> Vec<WorkerProcess> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| w.supervisor == supervisor)
            .cloned()
            .collect()
    }

    /// Get active workers
    pub async fn active(&self) -> Vec<WorkerProcess> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| w.is_active())
            .cloned()
            .collect()
    }

    /// Get idle workers
    pub async fn idle(&self) -> Vec<WorkerProcess> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| w.status == WorkerProcessStatus::Idle)
            .cloned()
            .collect()
    }

    /// Get running workers (currently processing)
    pub async fn running(&self) -> Vec<WorkerProcess> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| w.status == WorkerProcessStatus::Running)
            .cloned()
            .collect()
    }

    /// Get paused workers
    pub async fn paused(&self) -> Vec<WorkerProcess> {
        self.workers
            .read()
            .await
            .values()
            .filter(|w| w.status == WorkerProcessStatus::Paused)
            .cloned()
            .collect()
    }

    /// Update worker status
    pub async fn update_status(&self, worker_id: &str, status: WorkerProcessStatus) {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.status = status;
        }
    }

    /// Start a job on a worker
    pub async fn start_job(&self, worker_id: &str, job: CurrentJob) {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.start_job(job);
        }
    }

    /// Complete a job on a worker
    pub async fn complete_job(&self, worker_id: &str, success: bool) {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.complete_job(success);
        }
    }

    /// Pause a worker
    pub async fn pause_worker(&self, worker_id: &str) {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.pause();
        }
    }

    /// Resume a worker
    pub async fn resume_worker(&self, worker_id: &str) {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.resume();
        }
    }

    /// Stop a worker
    pub async fn stop_worker(&self, worker_id: &str) {
        if let Some(worker) = self.workers.write().await.get_mut(worker_id) {
            worker.stop();
        }
    }

    /// Pause all workers
    pub async fn pause_all(&self) {
        for worker in self.workers.write().await.values_mut() {
            worker.pause();
        }
    }

    /// Resume all workers
    pub async fn resume_all(&self) {
        for worker in self.workers.write().await.values_mut() {
            worker.resume();
        }
    }

    /// Stop all workers
    pub async fn stop_all(&self) {
        for worker in self.workers.write().await.values_mut() {
            worker.stop();
        }
    }

    /// Update system metrics for all workers
    pub async fn update_system_metrics(&self) {
        let mut system = self.system.write().await;
        system.refresh_all();

        let mut workers = self.workers.write().await;
        for worker in workers.values_mut() {
            worker.update_system_metrics(&system);
        }
    }

    /// Get worker count
    pub async fn count(&self) -> usize {
        self.workers.read().await.len()
    }

    /// Get total jobs processed
    pub async fn total_jobs_processed(&self) -> u64 {
        self.workers
            .read()
            .await
            .values()
            .map(|w| w.jobs_processed)
            .sum()
    }

    /// Get total jobs failed
    pub async fn total_jobs_failed(&self) -> u64 {
        self.workers
            .read()
            .await
            .values()
            .map(|w| w.jobs_failed)
            .sum()
    }

    /// Get overall success rate
    pub async fn success_rate(&self) -> f64 {
        let total_processed = self.total_jobs_processed().await;
        let total_failed = self.total_jobs_failed().await;
        let total = total_processed + total_failed;

        if total == 0 {
            return 1.0;
        }

        total_processed as f64 / total as f64
    }

    /// Clear all workers
    pub async fn clear(&self) {
        self.workers.write().await.clear();
    }
}

impl Clone for WorkerRegistry {
    fn clone(&self) -> Self {
        Self {
            workers: Arc::clone(&self.workers),
            system: Arc::clone(&self.system),
        }
    }
}

impl Default for WorkerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_job_new() {
        let job = CurrentJob::new("job-123", "SendEmail");
        assert_eq!(job.id, "job-123");
        assert_eq!(job.name, "SendEmail");
        assert_eq!(job.progress, None);
    }

    #[test]
    fn test_current_job_with_progress() {
        let job = CurrentJob::new("job-123", "SendEmail").with_progress(50);
        assert_eq!(job.progress, Some(50));
    }

    #[test]
    fn test_current_job_set_progress() {
        let mut job = CurrentJob::new("job-123", "SendEmail");
        job.set_progress(75);
        assert_eq!(job.progress, Some(75));

        // Test clamping at 100
        job.set_progress(150);
        assert_eq!(job.progress, Some(100));
    }

    #[test]
    fn test_worker_process_new() {
        let worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");
        assert_eq!(worker.id, "worker-1");
        assert_eq!(worker.queue, "default");
        assert_eq!(worker.status, WorkerProcessStatus::Idle);
        assert_eq!(worker.jobs_processed, 0);
    }

    #[test]
    fn test_worker_process_start_job() {
        let mut worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");
        let job = CurrentJob::new("job-123", "SendEmail");

        worker.start_job(job);

        assert_eq!(worker.status, WorkerProcessStatus::Running);
        assert!(worker.current_job.is_some());
        assert_eq!(worker.current_job.as_ref().unwrap().id, "job-123");
    }

    #[test]
    fn test_worker_process_complete_job() {
        let mut worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");
        let job = CurrentJob::new("job-123", "SendEmail");

        worker.start_job(job);
        worker.complete_job(true);

        assert_eq!(worker.status, WorkerProcessStatus::Idle);
        assert!(worker.current_job.is_none());
        assert_eq!(worker.jobs_processed, 1);
        assert_eq!(worker.jobs_failed, 0);
    }

    #[test]
    fn test_worker_process_complete_job_failure() {
        let mut worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");
        let job = CurrentJob::new("job-123", "SendEmail");

        worker.start_job(job);
        worker.complete_job(false);

        assert_eq!(worker.jobs_processed, 0);
        assert_eq!(worker.jobs_failed, 1);
    }

    #[test]
    fn test_worker_process_pause_resume() {
        let mut worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");

        worker.pause();
        assert_eq!(worker.status, WorkerProcessStatus::Paused);

        worker.resume();
        assert_eq!(worker.status, WorkerProcessStatus::Idle);
    }

    #[test]
    fn test_worker_process_success_rate() {
        let mut worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");
        worker.jobs_processed = 80;
        worker.jobs_failed = 20;

        assert_eq!(worker.success_rate(), 0.8);
    }

    #[tokio::test]
    async fn test_worker_registry_register() {
        let registry = WorkerRegistry::new();
        let worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");

        registry.register(worker).await;

        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_worker_registry_get() {
        let registry = WorkerRegistry::new();
        let worker = WorkerProcess::new("worker-1", "Worker 1", "supervisor-1", "default");

        registry.register(worker).await;

        let retrieved = registry.get("worker-1").await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "worker-1");
    }

    #[tokio::test]
    async fn test_worker_registry_by_queue() {
        let registry = WorkerRegistry::new();

        registry
            .register(WorkerProcess::new("w1", "W1", "s1", "emails"))
            .await;
        registry
            .register(WorkerProcess::new("w2", "W2", "s1", "default"))
            .await;
        registry
            .register(WorkerProcess::new("w3", "W3", "s1", "emails"))
            .await;

        let email_workers = registry.by_queue("emails").await;
        assert_eq!(email_workers.len(), 2);
    }

    #[tokio::test]
    async fn test_worker_registry_active() {
        let registry = WorkerRegistry::new();

        let mut w1 = WorkerProcess::new("w1", "W1", "s1", "default");
        w1.status = WorkerProcessStatus::Running;

        let mut w2 = WorkerProcess::new("w2", "W2", "s1", "default");
        w2.status = WorkerProcessStatus::Stopped;

        registry.register(w1).await;
        registry.register(w2).await;

        let active = registry.active().await;
        assert_eq!(active.len(), 1);
    }

    #[tokio::test]
    async fn test_worker_registry_pause_all() {
        let registry = WorkerRegistry::new();

        registry
            .register(WorkerProcess::new("w1", "W1", "s1", "default"))
            .await;
        registry
            .register(WorkerProcess::new("w2", "W2", "s1", "default"))
            .await;

        registry.pause_all().await;

        let paused = registry.paused().await;
        assert_eq!(paused.len(), 2);
    }

    #[tokio::test]
    async fn test_worker_registry_success_rate() {
        let registry = WorkerRegistry::new();

        let mut w1 = WorkerProcess::new("w1", "W1", "s1", "default");
        w1.jobs_processed = 100;
        w1.jobs_failed = 10;

        let mut w2 = WorkerProcess::new("w2", "W2", "s1", "default");
        w2.jobs_processed = 50;
        w2.jobs_failed = 5;

        registry.register(w1).await;
        registry.register(w2).await;

        let rate = registry.success_rate().await;
        // (100 + 50) / (100 + 10 + 50 + 5) = 150 / 165 ≈ 0.909
        assert!((rate - 0.909).abs() < 0.01);
    }
}
