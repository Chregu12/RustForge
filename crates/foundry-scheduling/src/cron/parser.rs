use super::{CronError, CronSchedule};

/// Cron expression parser
pub struct CronParser;

impl CronParser {
    /// Parse a cron expression
    pub fn parse(expression: &str) -> Result<CronSchedule, CronError> {
        CronSchedule::new(expression)
    }

    /// Validate a cron expression
    pub fn validate(expression: &str) -> bool {
        Self::parse(expression).is_ok()
    }

    /// Parse with description
    pub fn parse_with_description(expression: &str) -> Result<(CronSchedule, String), CronError> {
        let schedule = Self::parse(expression)?;
        let description = Self::describe(expression);
        Ok((schedule, description))
    }

    /// Get human-readable description
    pub fn describe(expression: &str) -> String {
        // Simple implementation - could be enhanced
        match expression {
            "* * * * *" => "Every minute".to_string(),
            "*/5 * * * *" => "Every 5 minutes".to_string(),
            "*/15 * * * *" => "Every 15 minutes".to_string(),
            "*/30 * * * *" => "Every 30 minutes".to_string(),
            "0 * * * *" => "Every hour".to_string(),
            "0 0 * * *" => "Every day at midnight".to_string(),
            "0 0 * * 0" => "Every Sunday at midnight".to_string(),
            "0 0 1 * *" => "On the 1st of every month at midnight".to_string(),
            _ => format!("Cron: {}", expression),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_validate() {
        assert!(CronParser::validate("0 * * * *"));
        assert!(!CronParser::validate("invalid"));
    }

    #[test]
    fn test_parser_describe() {
        let desc = CronParser::describe("0 * * * *");
        assert_eq!(desc, "Every hour");
    }
}
