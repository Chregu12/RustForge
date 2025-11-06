//! Command Event System for RustForge
//!
//! This crate provides a comprehensive event system for command lifecycle management,
//! enabling hooks into command execution with async event listeners.
//!
//! # Features
//!
//! - Event-based command lifecycle hooks
//! - Async event listeners
//! - Priority-based execution
//! - Error handling without breaking execution
//! - Broadcasting to multiple listeners
//!
//! # Example
//!
//! ```rust
//! use foundry_command_events::{EventDispatcher, CommandFinished, Event};
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut dispatcher = EventDispatcher::new();
//!
//!     dispatcher.listen(|event: &CommandFinished| async move {
//!         println!("Command {} finished in {}ms", event.command, event.duration);
//!         Ok(())
//!     }).await;
//!
//!     // Fire event
//!     dispatcher.dispatch(CommandFinished {
//!         command: "make:model".to_string(),
//!         duration: 150,
//!         exit_code: 0,
//!         output: "Model created successfully".to_string(),
//!     }).await;
//! }
//! ```

mod events;
mod dispatcher;
mod listener;
mod context;
mod error;

pub use events::*;
pub use dispatcher::EventDispatcher;
pub use listener::{EventListener, ListenerPriority};
pub use context::EventContext;
pub use error::{EventError, Result};
