//! Enhanced REPL implementation for Tinker

use crate::command::TinkerCommand;
use crate::completer::TinkerCompleter;
use crate::helpers::TinkerHelpers;
use crate::history::TinkerHistory;
use crate::session::{SessionManager, SessionScript};
use anyhow::Result;
use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::history::DefaultHistory;
use rustyline::Editor;
use std::path::PathBuf;

/// Configuration for Tinker REPL
#[derive(Debug, Clone)]
pub struct TinkerReplConfig {
    /// History file path
    pub history_path: PathBuf,
    /// Sessions directory
    pub sessions_dir: PathBuf,
    /// Enable syntax highlighting
    pub highlight: bool,
    /// Enable autocomplete
    pub autocomplete: bool,
    /// Prompt string
    pub prompt: String,
}

impl Default for TinkerReplConfig {
    fn default() -> Self {
        Self {
            history_path: crate::default_history_path().unwrap_or_else(|_| PathBuf::from(".tinker_history")),
            sessions_dir: crate::default_sessions_path().unwrap_or_else(|_| PathBuf::from(".tinker_sessions")),
            highlight: true,
            autocomplete: true,
            prompt: "tinker> ".to_string(),
        }
    }
}

/// Enhanced Tinker REPL
pub struct TinkerRepl {
    config: TinkerReplConfig,
    history_manager: TinkerHistory,
    helpers: TinkerHelpers,
    session_manager: SessionManager,
    session_commands: Vec<String>,
    editor: Editor<TinkerCompleter, DefaultHistory>,
}

impl TinkerRepl {
    /// Create a new Tinker REPL
    pub fn new(config: TinkerReplConfig) -> Result<Self> {
        let mut history_manager = TinkerHistory::new(config.history_path.clone());
        history_manager.load()?;

        let helpers = TinkerHelpers::new();
        let session_manager = SessionManager::new(config.sessions_dir.clone());
        let session_commands = Vec::new();

        let completer = TinkerCompleter::new();
        let mut editor = Editor::new()?;
        editor.set_helper(Some(completer));

        Ok(Self {
            config,
            history_manager,
            helpers,
            session_manager,
            session_commands,
            editor,
        })
    }

    /// Run the REPL
    pub fn run(&mut self) -> Result<()> {
        self.print_welcome();

        loop {
            match self.editor.readline(&self.config.prompt) {
                Ok(line) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let _ = self.editor.add_history_entry(trimmed);
                    self.history_manager.add(trimmed)?;
                    self.session_commands.push(trimmed.to_string());

                    let command = TinkerCommand::parse(trimmed);
                    match self.execute_command(command) {
                        Ok(should_continue) => {
                            if !should_continue {
                                break;
                            }
                        }
                        Err(e) => {
                            eprintln!("{} {}", "Error:".red().bold(), e);
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    println!("exit");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
            }
        }

        self.history_manager.save()?;
        Ok(())
    }

    /// Execute a Tinker command
    fn execute_command(&mut self, command: TinkerCommand) -> Result<bool> {
        match command {
            TinkerCommand::Helpers => {
                println!("{}", TinkerHelpers::format_helpers());
                Ok(true)
            }
            TinkerCommand::Models => {
                println!("\n Available Models:\n");
                println!("  (No models loaded - this would list ORM models)");
                Ok(true)
            }
            TinkerCommand::Routes => {
                println!("\n Available Routes:\n");
                println!("  (No routes loaded - this would list API routes)");
                Ok(true)
            }
            TinkerCommand::Config(key) => {
                let value = self.helpers.config(&key, None);
                println!("{}", serde_json::to_string_pretty(&value)?);
                Ok(true)
            }
            TinkerCommand::Env(key) => {
                match self.helpers.env(&key, None) {
                    Ok(value) => println!("{}", value),
                    Err(e) => println!("{} {}", "Not set:".yellow(), e),
                }
                Ok(true)
            }
            TinkerCommand::Clear => {
                print!("\x1B[2J\x1B[1;1H");
                Ok(true)
            }
            TinkerCommand::History => {
                self.show_history();
                Ok(true)
            }
            TinkerCommand::Save(name) => {
                self.save_session(&name)?;
                println!("{} Session saved as '{}'", "✓".green(), name);
                Ok(true)
            }
            TinkerCommand::Help => {
                self.print_help();
                Ok(true)
            }
            TinkerCommand::Exit => {
                println!("Goodbye!");
                Ok(false)
            }
            TinkerCommand::Execute(code) => {
                println!("{} Executing: {}", ">>".cyan(), code);
                println!("  (Code execution not implemented - this is a demo)");
                Ok(true)
            }
        }
    }

    /// Print welcome message
    fn print_welcome(&self) {
        println!("\n{}", "╔═══════════════════════════════════════════╗".cyan());
        println!("{}", "║   Foundry Tinker Enhanced REPL v0.1.0   ║".cyan());
        println!("{}", "╚═══════════════════════════════════════════╝".cyan());
        println!("\nType {} for available commands, {} to exit\n", "help".green(), "exit".red());
    }

    /// Print help message
    fn print_help(&self) {
        println!("\n{}\n", "Available Commands:".bold());
        println!("  {:15} - {}", "helpers", "Show all available helper functions");
        println!("  {:15} - {}", "models", "List all available models");
        println!("  {:15} - {}", "routes", "Show all registered routes");
        println!("  {:15} - {}", "config <key>", "Show configuration value");
        println!("  {:15} - {}", "env <key>", "Show environment variable");
        println!("  {:15} - {}", "clear", "Clear the screen");
        println!("  {:15} - {}", "history", "Show command history");
        println!("  {:15} - {}", "save <name>", "Save current session as script");
        println!("  {:15} - {}", "help", "Show this help message");
        println!("  {:15} - {}", "exit / quit", "Exit Tinker REPL");
        println!();
    }

    /// Show command history
    fn show_history(&self) {
        let entries = self.history_manager.last_n(20);
        println!("\n{}\n", "Recent History:".bold());
        for (i, entry) in entries.iter().enumerate() {
            println!("  {:3}. {}", i + 1, entry);
        }
        println!();
    }

    /// Save current session
    fn save_session(&self, name: &str) -> Result<()> {
        let session = SessionScript::new(name.to_string(), self.session_commands.clone());
        self.session_manager.save(&session)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_repl_config_default() {
        let config = TinkerReplConfig::default();
        assert!(config.highlight);
        assert!(config.autocomplete);
        assert!(!config.prompt.is_empty());
    }

    #[test]
    fn test_repl_new() {
        let temp_dir = TempDir::new().unwrap();
        let config = TinkerReplConfig {
            history_path: temp_dir.path().join("history"),
            sessions_dir: temp_dir.path().join("sessions"),
            highlight: true,
            autocomplete: true,
            prompt: "test> ".to_string(),
        };

        let repl = TinkerRepl::new(config);
        assert!(repl.is_ok());
    }
}
