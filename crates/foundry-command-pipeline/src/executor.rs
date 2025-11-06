use async_trait::async_trait;

/// Trait for executing commands in a pipeline
#[async_trait]
pub trait CommandExecutor: Send + Sync + Clone {
    /// Execute a command with arguments
    ///
    /// Returns Ok(output) on success, Err(reason) on failure
    async fn execute(&self, command: &str, args: Vec<String>) -> Result<String, String>;
}

/// A simple executor for testing
#[derive(Debug, Clone)]
pub struct DummyExecutor;

#[async_trait]
impl CommandExecutor for DummyExecutor {
    async fn execute(&self, command: &str, _args: Vec<String>) -> Result<String, String> {
        Ok(format!("Executed: {}", command))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dummy_executor() {
        let executor = DummyExecutor;
        let result = executor.execute("test", vec![]).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Executed: test");
    }
}
