//! # rf-tinker: Laravel Tinker-style Interactive REPL
//!
//! An interactive shell for RustForge applications, inspired by Laravel's Tinker.
//!
//! ## Features
//!
//! - **Database Queries**: Execute SQL and use the query builder
//! - **Model Inspection**: Explore your models and relationships
//! - **Cache Operations**: Get, set, and clear cache entries
//! - **Helper Functions**: Access all RustForge helpers
//! - **History**: Command history with arrow key navigation
//! - **Syntax Highlighting**: Colored output for better readability
//!
//! ## Quick Start
//!
//! ```bash
//! # Start tinker
//! forge tinker
//!
//! # Or with a specific database
//! forge tinker --database=sqlite://app.db
//! ```
//!
//! ## Commands
//!
//! ```text
//! Tinker> DB::table("users").get()
//! Tinker> User::find(1)
//! Tinker> Cache::get("key")
//! Tinker> .tables          # List all tables
//! Tinker> .schema users    # Show table schema
//! Tinker> .models          # List available models
//! Tinker> .help            # Show help
//! Tinker> .exit            # Exit tinker
//! ```

pub mod commands;
pub mod completer;
pub mod executor;
pub mod formatter;
pub mod highlighter;
pub mod repl;

pub use executor::{ExecutionContext, ExecutionResult, QueryExecutor};
pub use formatter::OutputFormatter;
pub use repl::{Tinker, TinkerConfig};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{ExecutionContext, ExecutionResult, OutputFormatter, QueryExecutor, Tinker, TinkerConfig};
}
