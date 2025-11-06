//! Signal Handling and Graceful Shutdown for Foundry Core
//!
//! This crate provides cross-platform signal handling for graceful application shutdown,
//! similar to Laravel's signal handling capabilities.
//!
//! # Examples
//!
//! ```no_run
//! use foundry_signal_handler::{SignalHandler, Signal};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let handler = SignalHandler::new();
//!
//! handler.on_signal(Signal::SIGTERM, || {
//!     println!("Cleaning up...");
//!     // Close connections, save state, etc.
//! }).await?;
//!
//! // Wait for signal
//! handler.wait().await?;
//! # Ok(())
//! # }
//! ```

pub mod callback;
pub mod error;
pub mod handler;
pub mod shutdown;
pub mod signal_types;

pub use callback::{SignalCallback, SignalCallbackFn};
pub use error::{SignalError, SignalResult};
pub use handler::SignalHandler;
pub use shutdown::{ShutdownManager, ShutdownPhase};
pub use signal_types::Signal;
