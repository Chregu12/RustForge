//! Job scheduling with cron-like patterns

use crate::error::SchedulerError;
use crate::job::Job;
use crate::queue::QueueManager;
use cron::Schedule;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::task::JoinHandle;

/// Scheduled job entry
struct ScheduledJob {
    /// Cron schedule
    schedule: Schedule,

    /// Job factory function
    job_factory: Box<dyn Fn() -> Box<dyn std::any::Any + Send> + Send + Sync>,

    /// Job name for logging
    name: String,
}

/// Job scheduler for cron-like scheduled tasks
///
/// # Example
///
/// ```ignore
/// use rf_jobs::{Scheduler, QueueManager};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let manager = QueueManager::new("redis://localhost:6379").await?;
/// let mut scheduler = Scheduler::new(manager);
///
/// // Schedule job every day at midnight
/// scheduler.schedule("0 0 * * *", "daily-report", || {
///     DailyReportJob
/// })?;
///
/// // Start scheduler
/// scheduler.start().await?;
/// # Ok(())
/// # }
/// ```
pub struct Scheduler {
    queue_manager: Arc<QueueManager>,
    schedules: Vec<ScheduledJob>,
    handle: Option<JoinHandle<()>>,
    shutdown_tx: broadcast::Sender<()>,
}

impl Scheduler {
    /// Create new scheduler
    pub fn new(queue_manager: QueueManager) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            queue_manager: Arc::new(queue_manager),
            schedules: Vec::new(),
            handle: None,
            shutdown_tx,
        }
    }

    /// Schedule a job with cron expression
    ///
    /// # Arguments
    ///
    /// * `cron_expr` - Cron expression (e.g., "0 0 * * *" for daily at midnight)
    /// * `name` - Job name for logging
    /// * `job_factory` - Function that creates the job
    ///
    /// # Example
    ///
    /// ```ignore
    /// scheduler.schedule("*/15 * * * *", "cache-cleanup", || {
    ///     CacheCleanupJob
    /// })?;
    /// ```
    pub fn schedule<F, J>(&mut self, cron_expr: &str, name: &str, job_factory: F) -> Result<(), SchedulerError>
    where
        F: Fn() -> J + Send + Sync + 'static,
        J: Job + 'static,
    {
        let schedule = Schedule::from_str(cron_expr)
            .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;

        let scheduled = ScheduledJob {
            schedule,
            job_factory: Box::new(move || Box::new(job_factory())),
            name: name.to_string(),
        };

        self.schedules.push(scheduled);

        tracing::info!(
            name = %name,
            cron = %cron_expr,
            "Scheduled job"
        );

        Ok(())
    }

    /// Start the scheduler
    ///
    /// The scheduler runs in the background and dispatches jobs according to their schedules.
    pub async fn start(&mut self) -> Result<(), SchedulerError> {
        if self.handle.is_some() {
            return Err(SchedulerError::InvalidCron("Scheduler already running".into()));
        }

        tracing::info!(
            schedules = self.schedules.len(),
            "Starting job scheduler"
        );

        let schedules = self.schedules.clone_schedules();
        let queue_manager = Arc::clone(&self.queue_manager);
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        let handle = tokio::spawn(async move {
            Self::run_scheduler(schedules, queue_manager, &mut shutdown_rx).await;
        });

        self.handle = Some(handle);

        Ok(())
    }

    /// Run the scheduler loop
    async fn run_scheduler(
        schedules: Vec<(Schedule, String)>,
        queue_manager: Arc<QueueManager>,
        shutdown_rx: &mut broadcast::Receiver<()>,
    ) {
        let mut last_minute = chrono::Utc::now().format("%Y-%m-%d %H:%M").to_string();

        loop {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                tracing::info!("Scheduler received shutdown signal");
                break;
            }

            let now = chrono::Utc::now();
            let current_minute = now.format("%Y-%m-%d %H:%M").to_string();

            // Only check once per minute
            if current_minute != last_minute {
                last_minute = current_minute;

                for (schedule, name) in &schedules {
                    if let Some(next) = schedule.upcoming(chrono::Utc).next() {
                        // Check if job should run now (within current minute)
                        if next <= now {
                            tracing::info!(
                                name = %name,
                                next = %next,
                                "Dispatching scheduled job"
                            );

                            // Note: In a real implementation, we would need a job registry
                            // to deserialize and dispatch the actual job
                            // For now, we just log
                            tracing::warn!(
                                name = %name,
                                "Job dispatching not yet implemented (needs job registry)"
                            );
                        }
                    }
                }
            }

            // Sleep for a short time
            tokio::time::sleep(Duration::from_secs(30)).await;
        }

        tracing::info!("Scheduler stopped");
    }

    /// Graceful shutdown
    pub async fn shutdown(self) -> Result<(), SchedulerError> {
        tracing::info!("Shutting down scheduler");

        // Signal scheduler to stop
        let _ = self.shutdown_tx.send(());

        // Wait for scheduler to finish
        if let Some(handle) = self.handle {
            handle
                .await
                .map_err(|e| SchedulerError::InvalidCron(format!("Shutdown error: {}", e)))?;
        }

        tracing::info!("Scheduler shutdown complete");
        Ok(())
    }
}

impl ScheduledJob {
    fn clone_schedules(schedules: &[ScheduledJob]) -> Vec<(Schedule, String)> {
        schedules
            .iter()
            .map(|s| (s.schedule.clone(), s.name.clone()))
            .collect()
    }
}

trait CloneSchedules {
    fn clone_schedules(&self) -> Vec<(Schedule, String)>;
}

impl CloneSchedules for Vec<ScheduledJob> {
    fn clone_schedules(&self) -> Vec<(Schedule, String)> {
        self.iter()
            .map(|s| (s.schedule.clone(), s.name.clone()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JobContext;
    use async_trait::async_trait;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, Serialize, Deserialize)]
    struct TestJob;

    #[async_trait]
    impl Job for TestJob {
        async fn handle(&self, _ctx: JobContext) -> crate::JobResult {
            Ok(())
        }
    }

    #[test]
    fn test_schedule_parsing() {
        // Valid cron expressions (cron crate uses 6 fields: sec min hour day month dayofweek)
        assert!(Schedule::from_str("0 0 0 * * *").is_ok()); // Daily at midnight
        assert!(Schedule::from_str("0 */15 * * * *").is_ok()); // Every 15 minutes
        assert!(Schedule::from_str("0 0 9 * * Mon").is_ok()); // Every Monday at 9am

        // Invalid cron expression
        assert!(Schedule::from_str("invalid").is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn test_scheduler_creation() {
        let manager = QueueManager::new("redis://localhost:6379")
            .await
            .unwrap();

        let mut scheduler = Scheduler::new(manager);

        scheduler
            .schedule("0 0 * * *", "test-job", || TestJob)
            .unwrap();

        assert_eq!(scheduler.schedules.len(), 1);
    }
}
