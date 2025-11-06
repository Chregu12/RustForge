//! Execution result types

use crate::output::CapturedOutput;
use foundry_plugins::CommandResult;
use serde::{Deserialize, Serialize};

/// Result of command execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Exit code (0 = success)
    pub exit_code: i32,

    /// Command result
    pub result: CommandResult,

    /// Captured output
    pub output: CapturedOutput,

    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl ExecutionResult {
    /// Create new execution result
    pub fn new(
        exit_code: i32,
        result: CommandResult,
        output: CapturedOutput,
        execution_time_ms: u64,
    ) -> Self {
        Self {
            exit_code,
            result,
            output,
            execution_time_ms,
        }
    }

    /// Check if execution was successful
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }

    /// Check if execution failed
    pub fn is_failure(&self) -> bool {
        !self.is_success()
    }

    /// Get stdout as string
    pub fn stdout(&self) -> String {
        self.output.stdout_string()
    }

    /// Get stderr as string
    pub fn stderr(&self) -> String {
        self.output.stderr_string()
    }

    /// Check if there were any errors
    pub fn has_errors(&self) -> bool {
        self.is_failure() || self.output.has_errors()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use foundry_plugins::CommandStatus;

    #[test]
    fn test_execution_result_success() {
        let result = ExecutionResult::new(
            0,
            CommandResult::success("test"),
            CapturedOutput::new(),
            100,
        );

        assert!(result.is_success());
        assert!(!result.is_failure());
        assert_eq!(result.exit_code, 0);
        assert_eq!(result.execution_time_ms, 100);
    }

    #[test]
    fn test_execution_result_failure() {
        let mut output = CapturedOutput::new();
        output.add_stderr("error".to_string());

        let result = ExecutionResult::new(
            1,
            CommandResult {
                status: CommandStatus::Failure,
                message: Some("failed".to_string()),
                data: None,
                error: None,
            },
            output,
            50,
        );

        assert!(result.is_failure());
        assert!(!result.is_success());
        assert!(result.has_errors());
        assert_eq!(result.stderr(), "error");
    }

    #[test]
    fn test_execution_result_output() {
        let mut output = CapturedOutput::new();
        output.add_stdout("line1".to_string());
        output.add_stdout("line2".to_string());

        let result = ExecutionResult::new(
            0,
            CommandResult::success("test"),
            output,
            75,
        );

        assert_eq!(result.stdout(), "line1\nline2");
        assert_eq!(result.stderr(), "");
    }
}
