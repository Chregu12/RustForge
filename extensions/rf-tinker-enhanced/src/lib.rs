//! # Foundry Tinker Enhanced
//!
//! Advanced REPL for Foundry with command history, autocomplete, syntax highlighting,
//! and built-in helpers for rapid development.
//!
//! ## Features
//!
//! - **Command History**: Persistent history stored in ~/.rustforge/tinker_history
//! - **Tab Completion**: Auto-complete for commands, models, and methods
//! - **Syntax Highlighting**: Color-coded output for better readability
//! - **Built-in Helpers**: now(), env(), config(), cache_get/put(), db_query(), dd()
//! - **Session Management**: Save sessions as executable scripts
//! - **Better Error Messages**: Clear, actionable error reporting

pub mod command;
pub mod completer;
pub mod helpers;
pub mod highlighter;
pub mod history;
pub mod repl;
pub mod session;

pub use command::TinkerCommand;
pub use completer::TinkerCompleter;
pub use helpers::TinkerHelpers;
pub use highlighter::TinkerHighlighter;
pub use history::TinkerHistory;
pub use repl::{TinkerRepl, TinkerReplConfig};
pub use session::{SessionManager, SessionScript};

use anyhow::Result;
use std::path::PathBuf;

/// Get the default Tinker history file path
pub fn default_history_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let rustforge_dir = home.join(".rustforge");
    std::fs::create_dir_all(&rustforge_dir)?;
    Ok(rustforge_dir.join("tinker_history"))
}

/// Get the default Tinker sessions directory
pub fn default_sessions_path() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let sessions_dir = home.join(".rustforge").join("sessions");
    std::fs::create_dir_all(&sessions_dir)?;
    Ok(sessions_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_paths() {
        let history = default_history_path();
        assert!(history.is_ok());

        let sessions = default_sessions_path();
        assert!(sessions.is_ok());
    }
}
