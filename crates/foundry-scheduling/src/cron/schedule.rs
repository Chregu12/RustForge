use super::CronError;
use chrono::{DateTime, Duration, Utc};
use chrono_tz::Tz;
use cron::Schedule as CronScheduleInner;
use std::str::FromStr;

/// Cron schedule wrapper
#[derive(Debug, Clone)]
pub struct CronSchedule {
    schedule: CronScheduleInner,
    timezone: Tz,
}

impl CronSchedule {
    pub fn new(expression: &str) -> Result<Self, CronError> {
        Self::with_timezone(expression, Tz::UTC)
    }

    pub fn with_timezone(expression: &str, timezone: Tz) -> Result<Self, CronError> {
        let schedule = CronScheduleInner::from_str(expression)
            .map_err(|e| CronError::InvalidExpression(e.to_string()))?;

        Ok(Self { schedule, timezone })
    }

    /// Get the next execution time from now
    pub fn next(&self) -> Option<DateTime<Utc>> {
        self.next_after(&Utc::now())
    }

    /// Get the next execution time after the given time
    pub fn next_after(&self, after: &DateTime<Utc>) -> Option<DateTime<Utc>> {
        let after_tz = after.with_timezone(&self.timezone);
        self.schedule
            .after(&after_tz)
            .next()
            .map(|dt| dt.with_timezone(&Utc))
    }

    /// Get upcoming execution times (limited to count)
    pub fn upcoming(&self, count: usize) -> Vec<DateTime<Utc>> {
        let now_tz = Utc::now().with_timezone(&self.timezone);
        self.schedule
            .after(&now_tz)
            .take(count)
            .map(|dt| dt.with_timezone(&Utc))
            .collect()
    }

    /// Check if the schedule should run at the given time
    pub fn is_due(&self, at: &DateTime<Utc>) -> bool {
        if let Some(next) = self.next_after(&(*at - Duration::minutes(1))) {
            next <= *at
        } else {
            false
        }
    }

    /// Get the timezone
    pub fn timezone(&self) -> Tz {
        self.timezone
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_cron_schedule_creation() {
        let schedule = CronSchedule::new("0 * * * *").unwrap();
        assert!(schedule.next().is_some());
    }

    #[test]
    fn test_cron_schedule_invalid() {
        let result = CronSchedule::new("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_cron_upcoming() {
        let schedule = CronSchedule::new("0 * * * *").unwrap();
        let upcoming = schedule.upcoming(5);
        assert_eq!(upcoming.len(), 5);
    }

    #[test]
    fn test_cron_timezone() {
        let schedule = CronSchedule::with_timezone("0 12 * * *", Tz::America__New_York).unwrap();
        assert_eq!(schedule.timezone(), Tz::America__New_York);
    }
}
