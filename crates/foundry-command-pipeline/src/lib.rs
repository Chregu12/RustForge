//! # Foundry Command Pipeline
//!
//! Command chaining and pipeline execution for batch operations.
//!
//! ## Features
//!
//! - **Sequential Execution**: Run commands one after another
//! - **Conditional Chaining**: Execute based on success/failure
//! - **Parallel Execution**: Run multiple commands concurrently
//! - **Error Handling**: Comprehensive error tracking and rollback support
//! - **Progress Tracking**: Monitor execution progress
//!
//! ## Example
//!
//! ```rust
//! use foundry_command_pipeline::{Pipeline, CommandExecutor};
//!
//! #[derive(Clone)]
//! struct MyExecutor;
//!
//! #[async_trait::async_trait]
//! impl CommandExecutor for MyExecutor {
//!     async fn execute(&self, command: &str, args: Vec<String>) -> Result<String, String> {
//!         Ok(format!("Executed: {}", command))
//!     }
//! }
//!
//! # tokio_test::block_on(async {
//! let executor = MyExecutor;
//! let mut pipeline = Pipeline::new(executor);
//!
//! pipeline
//!     .then("migrate", vec![])
//!     .then("seed", vec![])
//!     .then("cache:clear", vec![]);
//!
//! let result = pipeline.execute().await;
//! # });
//! ```

mod error;
mod executor;
mod pipeline;
mod result;

pub use error::{PipelineError, PipelineResult};
pub use executor::CommandExecutor;
pub use pipeline::{Pipeline, PipelineBuilder};
pub use result::{CommandResult, ExecutionSummary};
