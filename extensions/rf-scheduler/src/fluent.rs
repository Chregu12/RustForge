//! Fluent API for building task schedules

use crate::{Scheduler, SchedulerResult, Task};

/// Fluent task builder for chainable schedule configuration
pub struct TaskBuilder {
    name: Option<String>,
    cron_expression: Option<String>,
}

impl TaskBuilder {
    /// Create a new task builder
    pub fn new() -> Self {
        Self {
            name: None,
            cron_expression: None,
        }
    }

    /// Set the task name
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Schedule to run daily at midnight (00:00)
    pub fn daily(mut self) -> Self {
        self.cron_expression = Some("0 0 * * *".to_string());
        self
    }

    /// Schedule to run hourly (at minute 0)
    pub fn hourly(mut self) -> Self {
        self.cron_expression = Some("0 * * * *".to_string());
        self
    }

    /// Schedule to run weekly on Sunday at midnight
    pub fn weekly(mut self) -> Self {
        self.cron_expression = Some("0 0 * * SUN".to_string());
        self
    }

    /// Schedule to run monthly on the 1st at midnight
    pub fn monthly(mut self) -> Self {
        self.cron_expression = Some("0 0 1 * *".to_string());
        self
    }

    /// Schedule to run every 5 minutes
    pub fn every_five_minutes(mut self) -> Self {
        self.cron_expression = Some("*/5 * * * *".to_string());
        self
    }

    /// Schedule to run every 10 minutes
    pub fn every_ten_minutes(mut self) -> Self {
        self.cron_expression = Some("*/10 * * * *".to_string());
        self
    }

    /// Schedule to run every 15 minutes
    pub fn every_fifteen_minutes(mut self) -> Self {
        self.cron_expression = Some("*/15 * * * *".to_string());
        self
    }

    /// Schedule to run every 30 minutes
    pub fn every_thirty_minutes(mut self) -> Self {
        self.cron_expression = Some("*/30 * * * *".to_string());
        self
    }

    /// Schedule to run at a specific time (HH:MM format)
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.at("14:30") // Run daily at 2:30 PM
    /// ```
    pub fn at(mut self, time: &str) -> Self {
        let parts: Vec<&str> = time.split(':').collect();
        if parts.len() == 2 {
            let hour = parts[0];
            let minute = parts[1];
            self.cron_expression = Some(format!("{} {} * * *", minute, hour));
        }
        self
    }

    /// Schedule to run on specific day(s) of the week
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.daily().at("09:00").on("monday") // Run Mondays at 9 AM
    /// ```
    pub fn on(mut self, day: &str) -> Self {
        let day_num = match day.to_lowercase().as_str() {
            "sunday" | "sun" => "0",
            "monday" | "mon" => "1",
            "tuesday" | "tue" => "2",
            "wednesday" | "wed" => "3",
            "thursday" | "thu" => "4",
            "friday" | "fri" => "5",
            "saturday" | "sat" => "6",
            _ => return self,
        };

        if let Some(cron) = &self.cron_expression {
            let parts: Vec<&str> = cron.split_whitespace().collect();
            if parts.len() >= 5 {
                self.cron_expression = Some(format!(
                    "{} {} {} {} {}",
                    parts[0], parts[1], parts[2], parts[3], day_num
                ));
            }
        } else {
            // Default to midnight on that day
            self.cron_expression = Some(format!("0 0 * * {}", day_num));
        }

        self
    }

    /// Schedule to run on multiple days of the week
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.at("09:00").on_days(&["monday", "wednesday", "friday"])
    /// ```
    pub fn on_days(mut self, days: &[&str]) -> Self {
        let day_nums: Vec<String> = days
            .iter()
            .filter_map(|day| match day.to_lowercase().as_str() {
                "sunday" | "sun" => Some("0".to_string()),
                "monday" | "mon" => Some("1".to_string()),
                "tuesday" | "tue" => Some("2".to_string()),
                "wednesday" | "wed" => Some("3".to_string()),
                "thursday" | "thu" => Some("4".to_string()),
                "friday" | "fri" => Some("5".to_string()),
                "saturday" | "sat" => Some("6".to_string()),
                _ => None,
            })
            .collect();

        if !day_nums.is_empty() {
            let days_str = day_nums.join(",");

            if let Some(cron) = &self.cron_expression {
                let parts: Vec<&str> = cron.split_whitespace().collect();
                if parts.len() >= 5 {
                    self.cron_expression = Some(format!(
                        "{} {} {} {} {}",
                        parts[0], parts[1], parts[2], parts[3], days_str
                    ));
                }
            } else {
                // Default to midnight on those days
                self.cron_expression = Some(format!("0 0 * * {}", days_str));
            }
        }

        self
    }

    /// Schedule to run on weekdays (Monday-Friday)
    pub fn weekdays(self) -> Self {
        self.on_days(&["monday", "tuesday", "wednesday", "thursday", "friday"])
    }

    /// Schedule to run on weekends (Saturday-Sunday)
    pub fn weekends(self) -> Self {
        self.on_days(&["saturday", "sunday"])
    }

    /// Schedule to run between specific hours
    ///
    /// # Example
    ///
    /// ```ignore
    /// builder.hourly().between("9", "17") // Run hourly from 9 AM to 5 PM
    /// ```
    pub fn between(mut self, start_hour: &str, end_hour: &str) -> Self {
        let hour_range = format!("{}-{}", start_hour, end_hour);

        if let Some(cron) = &self.cron_expression {
            let parts: Vec<&str> = cron.split_whitespace().collect();
            if parts.len() >= 5 {
                self.cron_expression = Some(format!(
                    "{} {} {} {} {}",
                    parts[0], hour_range, parts[2], parts[3], parts[4]
                ));
            }
        }

        self
    }

    /// Get the built cron expression
    pub fn cron(&self) -> Option<&str> {
        self.cron_expression.as_deref()
    }

    /// Schedule the task with the built configuration
    pub async fn schedule<T: Task + 'static>(
        self,
        scheduler: &Scheduler,
        task: T,
    ) -> SchedulerResult<()> {
        if let Some(cron) = self.cron_expression {
            scheduler.schedule(&cron, task).await
        } else {
            // Default to daily if no schedule specified
            scheduler.schedule("0 0 * * *", task).await
        }
    }
}

impl Default for TaskBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_daily_schedule() {
        let builder = TaskBuilder::new().daily();
        assert_eq!(builder.cron(), Some("0 0 * * *"));
    }

    #[test]
    fn test_hourly_schedule() {
        let builder = TaskBuilder::new().hourly();
        assert_eq!(builder.cron(), Some("0 * * * *"));
    }

    #[test]
    fn test_weekly_schedule() {
        let builder = TaskBuilder::new().weekly();
        assert_eq!(builder.cron(), Some("0 0 * * SUN"));
    }

    #[test]
    fn test_monthly_schedule() {
        let builder = TaskBuilder::new().monthly();
        assert_eq!(builder.cron(), Some("0 0 1 * *"));
    }

    #[test]
    fn test_every_five_minutes() {
        let builder = TaskBuilder::new().every_five_minutes();
        assert_eq!(builder.cron(), Some("*/5 * * * *"));
    }

    #[test]
    fn test_at_specific_time() {
        let builder = TaskBuilder::new().at("14:30");
        assert_eq!(builder.cron(), Some("30 14 * * *"));
    }

    #[test]
    fn test_daily_at_time() {
        let builder = TaskBuilder::new().daily().at("09:00");
        assert_eq!(builder.cron(), Some("00 09 * * *"));
    }

    #[test]
    fn test_on_specific_day() {
        let builder = TaskBuilder::new().at("09:00").on("monday");
        assert_eq!(builder.cron(), Some("00 09 * * 1"));
    }

    #[test]
    fn test_on_multiple_days() {
        let builder = TaskBuilder::new()
            .at("09:00")
            .on_days(&["monday", "wednesday", "friday"]);
        assert_eq!(builder.cron(), Some("00 09 * * 1,3,5"));
    }

    #[test]
    fn test_weekdays() {
        let builder = TaskBuilder::new().at("09:00").weekdays();
        assert_eq!(builder.cron(), Some("00 09 * * 1,2,3,4,5"));
    }

    #[test]
    fn test_weekends() {
        let builder = TaskBuilder::new().at("10:00").weekends();
        assert_eq!(builder.cron(), Some("00 10 * * 6,0"));
    }

    #[test]
    fn test_between_hours() {
        let builder = TaskBuilder::new().hourly().between("9", "17");
        assert_eq!(builder.cron(), Some("0 9-17 * * *"));
    }

    #[test]
    fn test_builder_chaining() {
        let builder = TaskBuilder::new()
            .name("backup")
            .daily()
            .at("02:00")
            .weekdays();

        assert_eq!(builder.name, Some("backup".to_string()));
        assert_eq!(builder.cron(), Some("00 02 * * 1,2,3,4,5"));
    }
}
