pub mod parser;
pub mod schedule;

pub use parser::CronParser;
pub use schedule::CronSchedule;

use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CronError {
    #[error("Invalid cron expression: {0}")]
    InvalidExpression(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Common cron expression patterns
pub struct CronPatterns;

impl CronPatterns {
    /// Every minute
    pub const EVERY_MINUTE: &'static str = "* * * * *";

    /// Every 5 minutes
    pub const EVERY_FIVE_MINUTES: &'static str = "*/5 * * * *";

    /// Every 15 minutes
    pub const EVERY_FIFTEEN_MINUTES: &'static str = "*/15 * * * *";

    /// Every 30 minutes
    pub const EVERY_THIRTY_MINUTES: &'static str = "*/30 * * * *";

    /// Hourly
    pub const HOURLY: &'static str = "0 * * * *";

    /// Daily at midnight
    pub const DAILY: &'static str = "0 0 * * *";

    /// Weekly (Sunday at midnight)
    pub const WEEKLY: &'static str = "0 0 * * 0";

    /// Monthly (1st at midnight)
    pub const MONTHLY: &'static str = "0 0 1 * *";

    /// Yearly (Jan 1st at midnight)
    pub const YEARLY: &'static str = "0 0 1 1 *";
}
