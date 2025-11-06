//! Programmatic Command Execution for Foundry Core
//!
//! This crate provides the ability to execute commands programmatically,
//! similar to Laravel's `Artisan::call()` functionality.
//!
//! # Examples
//!
//! ```no_run
//! use foundry_command_executor::CommandExecutor;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let executor = CommandExecutor::new(registry);
//! let result = executor.execute("migrate:run", vec!["--force".to_string()]).await?;
//! println!("Exit code: {}", result.exit_code);
//! # Ok(())
//! # }
//! ```

pub mod error;
pub mod executor;
pub mod options;
pub mod output;
pub mod registry;
pub mod result;

pub use error::{ExecutionError, ExecutionResult};
pub use executor::CommandExecutor;
pub use options::{CommandOptions, ExecutionMode};
pub use output::{OutputCapture, OutputMode};
pub use registry::CommandRegistry;
pub use result::ExecutionResult as CommandExecutionResult;
