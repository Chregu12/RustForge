//! Health check reporting

use colored::Colorize;
use serde::{Deserialize, Serialize};

/// Status of a health check
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CheckStatus {
    /// Check passed
    Pass,
    /// Check failed
    Fail,
    /// Check was skipped
    Skip,
    /// Check returned a warning
    Warn,
}

impl CheckStatus {
    /// Get status symbol
    pub fn symbol(&self) -> &str {
        match self {
            CheckStatus::Pass => "✓",
            CheckStatus::Fail => "✗",
            CheckStatus::Skip => "⊘",
            CheckStatus::Warn => "⚠",
        }
    }

    /// Get colored symbol
    pub fn colored_symbol(&self) -> String {
        match self {
            CheckStatus::Pass => self.symbol().green().to_string(),
            CheckStatus::Fail => self.symbol().red().to_string(),
            CheckStatus::Skip => self.symbol().yellow().to_string(),
            CheckStatus::Warn => self.symbol().yellow().to_string(),
        }
    }
}

/// Result of a single health check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Name of the check
    pub name: String,
    /// Status of the check
    pub status: CheckStatus,
    /// Message describing the result
    pub message: String,
    /// Additional details
    pub details: Option<serde_json::Value>,
}

impl CheckResult {
    /// Create a passing check result
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            details: None,
        }
    }

    /// Create a failing check result
    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            details: None,
        }
    }

    /// Create a skipped check result
    pub fn skipped(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Skip,
            message: message.into(),
            details: None,
        }
    }

    /// Create a warning check result
    pub fn warn(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the result
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Complete health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// All check results
    pub checks: Vec<CheckResult>,
    /// Overall status
    pub overall_status: CheckStatus,
    /// Timestamp
    pub timestamp: String,
}

impl HealthReport {
    /// Create a new health report
    pub fn new(checks: Vec<CheckResult>) -> Self {
        let overall_status = if checks.iter().any(|c| c.status == CheckStatus::Fail) {
            CheckStatus::Fail
        } else if checks.iter().any(|c| c.status == CheckStatus::Warn) {
            CheckStatus::Warn
        } else {
            CheckStatus::Pass
        };

        Self {
            checks,
            overall_status,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }

    /// Format as human-readable table
    pub fn format_table(&self) -> String {
        let mut output = String::new();
        output.push_str("\n");
        output.push_str(&"╔═══════════════════════════════════════════╗\n".cyan().to_string());
        output.push_str(&"║         Health Check Report              ║\n".cyan().to_string());
        output.push_str(&"╚═══════════════════════════════════════════╝\n".cyan().to_string());
        output.push('\n');

        // Find max width
        let max_name_width = self
            .checks
            .iter()
            .map(|c| c.name.len())
            .max()
            .unwrap_or(20)
            .max(20);

        // Header
        output.push_str(&format!(
            "  {:<width$}  {}  {}\n",
            "Check".bold(),
            "Status".bold(),
            "Message".bold(),
            width = max_name_width
        ));
        output.push_str(&format!("  {}\n", "─".repeat(max_name_width + 50)));

        // Results
        for check in &self.checks {
            output.push_str(&format!(
                "  {:<width$}  {}      {}\n",
                check.name,
                check.status.colored_symbol(),
                check.message,
                width = max_name_width
            ));
        }

        output.push('\n');

        // Summary
        let summary = format!("Overall Status: {}", self.overall_status.colored_symbol());
        output.push_str(&format!("  {}\n", summary));
        output.push('\n');

        output
    }

    /// Count checks by status
    pub fn count_by_status(&self, status: CheckStatus) -> usize {
        self.checks.iter().filter(|c| c.status == status).count()
    }

    /// Check if all checks passed
    pub fn all_passed(&self) -> bool {
        self.overall_status == CheckStatus::Pass
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_status_symbol() {
        assert_eq!(CheckStatus::Pass.symbol(), "✓");
        assert_eq!(CheckStatus::Fail.symbol(), "✗");
        assert_eq!(CheckStatus::Skip.symbol(), "⊘");
        assert_eq!(CheckStatus::Warn.symbol(), "⚠");
    }

    #[test]
    fn test_check_result_pass() {
        let result = CheckResult::pass("test", "Success");
        assert_eq!(result.name, "test");
        assert_eq!(result.status, CheckStatus::Pass);
        assert_eq!(result.message, "Success");
    }

    #[test]
    fn test_check_result_with_details() {
        let result = CheckResult::pass("test", "Success")
            .with_details(serde_json::json!({"key": "value"}));

        assert!(result.details.is_some());
    }

    #[test]
    fn test_health_report_overall_status() {
        let checks = vec![
            CheckResult::pass("check1", "OK"),
            CheckResult::pass("check2", "OK"),
        ];
        let report = HealthReport::new(checks);
        assert_eq!(report.overall_status, CheckStatus::Pass);

        let checks_with_fail = vec![
            CheckResult::pass("check1", "OK"),
            CheckResult::fail("check2", "Failed"),
        ];
        let report_fail = HealthReport::new(checks_with_fail);
        assert_eq!(report_fail.overall_status, CheckStatus::Fail);
    }

    #[test]
    fn test_health_report_count_by_status() {
        let checks = vec![
            CheckResult::pass("check1", "OK"),
            CheckResult::fail("check2", "Failed"),
            CheckResult::pass("check3", "OK"),
        ];
        let report = HealthReport::new(checks);

        assert_eq!(report.count_by_status(CheckStatus::Pass), 2);
        assert_eq!(report.count_by_status(CheckStatus::Fail), 1);
    }

    #[test]
    fn test_format_table() {
        let checks = vec![
            CheckResult::pass("rust", "Version 1.70.0"),
            CheckResult::pass("disk", "10 GB available"),
        ];
        let report = HealthReport::new(checks);
        let table = report.format_table();

        assert!(table.contains("rust"));
        assert!(table.contains("disk"));
        assert!(table.contains("Overall Status"));
    }
}
