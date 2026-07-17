use std::time::Duration;

/// Result of a single command execution
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// The command that was executed
    pub command: String,
    /// Arguments passed to the command
    pub args: Vec<String>,
    /// Whether the command succeeded
    pub success: bool,
    /// Output from the command
    pub output: String,
    /// Error message if failed
    pub error: Option<String>,
    /// Duration of execution
    pub duration: Duration,
}

impl CommandResult {
    /// Create a successful result
    pub fn success(command: impl Into<String>, args: Vec<String>, output: String, duration: Duration) -> Self {
        Self {
            command: command.into(),
            args,
            success: true,
            output,
            error: None,
            duration,
        }
    }

    /// Create a failed result
    pub fn failure(
        command: impl Into<String>,
        args: Vec<String>,
        error: String,
        duration: Duration,
    ) -> Self {
        Self {
            command: command.into(),
            args,
            success: false,
            output: String::new(),
            error: Some(error),
            duration,
        }
    }
}

/// Summary of pipeline execution
#[derive(Debug, Clone)]
pub struct ExecutionSummary {
    /// Total number of commands
    pub total: usize,
    /// Number of successful commands
    pub succeeded: usize,
    /// Number of failed commands
    pub failed: usize,
    /// Number of skipped commands
    pub skipped: usize,
    /// Total duration
    pub total_duration: Duration,
    /// Individual command results
    pub results: Vec<CommandResult>,
}

impl ExecutionSummary {
    /// Create a new execution summary
    pub fn new() -> Self {
        Self {
            total: 0,
            succeeded: 0,
            failed: 0,
            skipped: 0,
            total_duration: Duration::ZERO,
            results: Vec::new(),
        }
    }

    /// Add a result to the summary
    pub fn add_result(&mut self, result: CommandResult) {
        self.total += 1;
        if result.success {
            self.succeeded += 1;
        } else {
            self.failed += 1;
        }
        self.total_duration += result.duration;
        self.results.push(result);
    }

    /// Add a skipped command
    pub fn add_skipped(&mut self) {
        self.total += 1;
        self.skipped += 1;
    }

    /// Check if all commands succeeded
    pub fn all_succeeded(&self) -> bool {
        self.failed == 0 && self.succeeded == self.total - self.skipped
    }

    /// Get all failed commands
    pub fn failed_commands(&self) -> Vec<&CommandResult> {
        self.results.iter().filter(|r| !r.success).collect()
    }
}

impl Default for ExecutionSummary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_result_success() {
        let result = CommandResult::success("test", vec![], "output".to_string(), Duration::from_secs(1));
        assert!(result.success);
        assert_eq!(result.output, "output");
        assert!(result.error.is_none());
    }

    #[test]
    fn test_command_result_failure() {
        let result = CommandResult::failure("test", vec![], "error".to_string(), Duration::from_secs(1));
        assert!(!result.success);
        assert_eq!(result.error, Some("error".to_string()));
    }

    #[test]
    fn test_execution_summary() {
        let mut summary = ExecutionSummary::new();
        assert_eq!(summary.total, 0);

        summary.add_result(CommandResult::success("cmd1", vec![], "ok".to_string(), Duration::from_secs(1)));
        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);

        summary.add_result(CommandResult::failure("cmd2", vec![], "err".to_string(), Duration::from_secs(1)));
        assert_eq!(summary.total, 2);
        assert_eq!(summary.failed, 1);

        summary.add_skipped();
        assert_eq!(summary.total, 3);
        assert_eq!(summary.skipped, 1);

        assert!(!summary.all_succeeded());
    }

    #[test]
    fn test_failed_commands() {
        let mut summary = ExecutionSummary::new();
        summary.add_result(CommandResult::success("cmd1", vec![], "ok".to_string(), Duration::from_secs(1)));
        summary.add_result(CommandResult::failure("cmd2", vec![], "err".to_string(), Duration::from_secs(1)));

        let failed = summary.failed_commands();
        assert_eq!(failed.len(), 1);
        assert_eq!(failed[0].command, "cmd2");
    }
}
