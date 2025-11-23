use thiserror::Error;

/// Result type for pipeline operations
pub type PipelineResult<T> = Result<T, PipelineError>;

/// Error types for pipeline execution
#[derive(Debug, Clone, Error)]
pub enum PipelineError {
    /// Command execution failed
    #[error("Command '{command}' failed: {reason}")]
    CommandFailed { command: String, reason: String },

    /// Multiple commands failed
    #[error("Pipeline failed with {0} error(s)")]
    MultipleFailed(Vec<String>),

    /// Pipeline was stopped due to error
    #[error("Pipeline stopped after command '{0}' failed")]
    Stopped(String),

    /// Invalid pipeline configuration
    #[error("Invalid pipeline: {0}")]
    InvalidPipeline(String),

    /// Custom error
    #[error("{0}")]
    Custom(String),
}

impl PipelineError {
    /// Create a command failed error
    pub fn command_failed(command: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::CommandFailed {
            command: command.into(),
            reason: reason.into(),
        }
    }

    /// Create a multiple failed error
    pub fn multiple_failed(errors: Vec<String>) -> Self {
        Self::MultipleFailed(errors)
    }

    /// Create a stopped error
    pub fn stopped(command: impl Into<String>) -> Self {
        Self::Stopped(command.into())
    }

    /// Create an invalid pipeline error
    pub fn invalid(message: impl Into<String>) -> Self {
        Self::InvalidPipeline(message.into())
    }

    /// Create a custom error
    pub fn custom(message: impl Into<String>) -> Self {
        Self::Custom(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_failed_error() {
        let error = PipelineError::command_failed("migrate", "Connection refused");
        assert!(matches!(error, PipelineError::CommandFailed { .. }));
    }

    #[test]
    fn test_multiple_failed_error() {
        let errors = vec!["Error 1".to_string(), "Error 2".to_string()];
        let error = PipelineError::multiple_failed(errors);
        assert!(matches!(error, PipelineError::MultipleFailed(_)));
    }

    #[test]
    fn test_stopped_error() {
        let error = PipelineError::stopped("seed");
        assert!(matches!(error, PipelineError::Stopped(_)));
    }
}
