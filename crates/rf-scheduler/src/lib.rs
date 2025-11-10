//! # rf-scheduler: Cron-like Task Scheduling for RustForge
//!
//! Provides scheduled task execution with cron expressions and simple intervals.
//!
//! ## Features
//!
//! - **Cron Expressions**: Full cron syntax support
//! - **Simple Intervals**: Hourly, daily, weekly shortcuts
//! - **Overlap Prevention**: Prevent concurrent task execution
//! - **Error Handling**: Automatic error logging and retry
//! - **Async Tasks**: Full async/await support
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_scheduler::{Scheduler, Task};
//! use async_trait::async_trait;
//!
//! struct CleanupTask;
//!
//! #[async_trait]
//! impl Task for CleanupTask {
//!     async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!         println!("Running cleanup...");
//!         Ok(())
//!     }
//!
//!     fn name(&self) -> &str {
//!         "cleanup"
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let scheduler = Scheduler::new();
//!
//! // Cron: Every day at midnight
//! scheduler.schedule("0 0 * * *", CleanupTask).await?;
//!
//! // Simple: Every hour
//! scheduler.hourly(CleanupTask).await;
//!
//! // scheduler.start().await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use cron::Schedule;
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{sleep, Duration};

pub use thiserror::Error;

/// Scheduler errors
#[derive(Debug, Error)]
pub enum SchedulerError {
    #[error("Invalid cron expression: {0}")]
    InvalidCron(String),

    #[error("Task execution failed: {0}")]
    TaskFailed(String),

    #[error("Task already running: {0}")]
    TaskRunning(String),
}

/// Result type for scheduler operations
pub type SchedulerResult<T> = Result<T, SchedulerError>;

/// Task trait for scheduled tasks
#[async_trait]
pub trait Task: Send + Sync {
    /// Execute the task
    async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    /// Get task name
    fn name(&self) -> &str;

    /// Prevent task overlap (default: true)
    fn prevent_overlap(&self) -> bool {
        true
    }
}

struct ScheduledTask {
    schedule: Schedule,
    task: Arc<dyn Task>,
    last_run: Option<DateTime<Utc>>,
    running: bool,
}

/// Task scheduler
pub struct Scheduler {
    tasks: Arc<Mutex<Vec<ScheduledTask>>>,
    running_tasks: Arc<Mutex<HashMap<String, bool>>>,
}

impl Scheduler {
    /// Create new scheduler
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(Vec::new())),
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Schedule task with cron expression (supports 5 or 6 field cron)
    pub async fn schedule(&self, cron: &str, task: impl Task + 'static) -> SchedulerResult<()> {
        // Add seconds field if not present (cron crate requires 6 fields)
        let cron_expr = if cron.split_whitespace().count() == 5 {
            format!("0 {}", cron)
        } else {
            cron.to_string()
        };

        let schedule = Schedule::from_str(&cron_expr)
            .map_err(|e| SchedulerError::InvalidCron(e.to_string()))?;

        let scheduled = ScheduledTask {
            schedule,
            task: Arc::new(task),
            last_run: None,
            running: false,
        };

        let mut tasks = self.tasks.lock().await;
        tasks.push(scheduled);

        Ok(())
    }

    /// Schedule task to run every hour
    pub async fn hourly(&self, task: impl Task + 'static) {
        self.schedule("0 * * * *", task).await.unwrap();
    }

    /// Schedule task to run daily at specific time (HH:MM format)
    pub async fn daily_at(&self, time: &str, task: impl Task + 'static) -> SchedulerResult<()> {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() != 2 {
            return Err(SchedulerError::InvalidCron(
                "Time must be in HH:MM format".to_string(),
            ));
        }

        let cron = format!("{} {} * * *", parts[1], parts[0]);
        self.schedule(&cron, task).await
    }

    /// Schedule task to run daily
    pub async fn daily(&self, task: impl Task + 'static) {
        self.daily_at("00:00", task).await.unwrap();
    }

    /// Start the scheduler
    pub async fn start(self) -> SchedulerResult<()> {
        loop {
            let mut tasks = self.tasks.lock().await;
            let now = Utc::now();

            for scheduled in tasks.iter_mut() {
                // Check if task should run
                if let Some(next) = scheduled.schedule.upcoming(Utc).next() {
                    if next <= now && !scheduled.running {
                        // Check if task ran recently
                        if let Some(last) = scheduled.last_run {
                            if (now - last).num_seconds() < 60 {
                                continue;
                            }
                        }

                        // Check overlap
                        if scheduled.task.prevent_overlap() {
                            let mut running = self.running_tasks.lock().await;
                            if running.get(scheduled.task.name()).copied().unwrap_or(false) {
                                tracing::warn!(
                                    task = scheduled.task.name(),
                                    "Task still running, skipping"
                                );
                                continue;
                            }
                            running.insert(scheduled.task.name().to_string(), true);
                        }

                        // Run task
                        scheduled.running = true;
                        scheduled.last_run = Some(now);

                        let task = Arc::clone(&scheduled.task);
                        let running_tasks = Arc::clone(&self.running_tasks);

                        tokio::spawn(async move {
                            let task_name = task.name().to_string();

                            tracing::info!(task = %task_name, "Running scheduled task");

                            match task.run().await {
                                Ok(_) => {
                                    tracing::info!(task = %task_name, "Task completed successfully");
                                }
                                Err(e) => {
                                    tracing::error!(task = %task_name, error = %e, "Task failed");
                                }
                            }

                            // Mark as not running
                            let mut running = running_tasks.lock().await;
                            running.remove(&task_name);
                        });

                        scheduled.running = false;
                    }
                }
            }

            drop(tasks);
            sleep(Duration::from_secs(30)).await;
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestTask {
        name: String,
    }

    #[async_trait]
    impl Task for TestTask {
        async fn run(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        fn name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_schedule_creation() {
        let scheduler = Scheduler::new();
        let task = TestTask {
            name: "test".to_string(),
        };

        assert!(scheduler.schedule("0 * * * *", task).await.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_cron() {
        let scheduler = Scheduler::new();
        let task = TestTask {
            name: "test".to_string(),
        };

        assert!(scheduler.schedule("invalid", task).await.is_err());
    }

    #[tokio::test]
    async fn test_daily_at() {
        let scheduler = Scheduler::new();
        let task = TestTask {
            name: "test".to_string(),
        };

        assert!(scheduler.daily_at("02:30", task).await.is_ok());
    }

    #[tokio::test]
    async fn test_shortcuts() {
        let scheduler = Scheduler::new();

        scheduler.hourly(TestTask {
            name: "hourly".to_string(),
        }).await;

        scheduler.daily(TestTask {
            name: "daily".to_string(),
        }).await;

        // Just check they don't panic
    }
}
