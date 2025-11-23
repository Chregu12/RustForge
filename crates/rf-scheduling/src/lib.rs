//! Foundry Scheduling - Task scheduling system with cron expressions
//!
//! This crate provides a comprehensive task scheduling system for the Foundry framework.
//!
//! # Features
//!
//! - **Cron Expressions**: Standard cron syntax with timezone support
//! - **Job Management**: Register and execute scheduled jobs
//! - **Timeout Support**: Configure maximum execution time
//! - **Overlap Control**: Prevent or allow concurrent job execution
//! - **Error Handling**: Callbacks for success and failure
//!
//! # Example
//!
//! ```no_run
//! use rf_scheduling::prelude::*;
//!
//! # async fn example() {
//! // Create a scheduler
//! let scheduler = TaskScheduler::new();
//!
//! // Schedule a simple job
//! scheduler.schedule_fn("cleanup", "0 2 * * *", |ctx| async {
//!     println!("Running cleanup at {:?}", ctx.started_at);
//!     Ok(())
//! }).await;
//!
//! // Run the scheduler (blocking)
//! scheduler.run().await;
//! # }
//! ```

pub mod cron;
pub mod jobs;
pub mod scheduler;

pub use cron::{CronError, CronParser, CronPatterns, CronSchedule};
pub use jobs::{JobContext, JobResult, JobRunner, RunnerConfig, ScheduledJob};
pub use scheduler::TaskScheduler;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::cron::{CronParser, CronPatterns, CronSchedule};
    pub use crate::jobs::{JobContext, JobError, JobResult, ScheduledJob};
    pub use crate::scheduler::TaskScheduler;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cron_patterns() {
        assert_eq!(CronPatterns::HOURLY, "0 * * * *");
        assert_eq!(CronPatterns::DAILY, "0 0 * * *");
    }
}
