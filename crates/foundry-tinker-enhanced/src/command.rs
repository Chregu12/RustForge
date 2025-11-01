//! Tinker command definitions and parsing

use serde::{Deserialize, Serialize};

/// Tinker command types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TinkerCommand {
    /// Show all available helpers
    Helpers,
    /// List all available models
    Models,
    /// Show all routes
    Routes,
    /// Show configuration value
    Config(String),
    /// Show environment variable
    Env(String),
    /// Clear the screen
    Clear,
    /// Exit tinker
    Exit,
    /// Show command history
    History,
    /// Save session as script
    Save(String),
    /// Execute raw code
    Execute(String),
    /// Show help
    Help,
}

impl TinkerCommand {
    /// Parse a command from user input
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();

        if trimmed.is_empty() {
            return TinkerCommand::Execute(String::new());
        }

        let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
        let cmd = parts[0];
        let args = parts.get(1).map(|s| s.trim().to_string());

        match cmd {
            "helpers" => TinkerCommand::Helpers,
            "models" => TinkerCommand::Models,
            "routes" => TinkerCommand::Routes,
            "config" => {
                if let Some(key) = args {
                    TinkerCommand::Config(key)
                } else {
                    TinkerCommand::Help
                }
            }
            "env" => {
                if let Some(key) = args {
                    TinkerCommand::Env(key)
                } else {
                    TinkerCommand::Help
                }
            }
            "clear" | "cls" => TinkerCommand::Clear,
            "exit" | "quit" | "q" => TinkerCommand::Exit,
            "history" | "hist" => TinkerCommand::History,
            "save" => {
                if let Some(name) = args {
                    TinkerCommand::Save(name)
                } else {
                    TinkerCommand::Help
                }
            }
            "help" | "?" => TinkerCommand::Help,
            _ => TinkerCommand::Execute(trimmed.to_string()),
        }
    }

    /// Get command name for history
    pub fn name(&self) -> &str {
        match self {
            TinkerCommand::Helpers => "helpers",
            TinkerCommand::Models => "models",
            TinkerCommand::Routes => "routes",
            TinkerCommand::Config(_) => "config",
            TinkerCommand::Env(_) => "env",
            TinkerCommand::Clear => "clear",
            TinkerCommand::Exit => "exit",
            TinkerCommand::History => "history",
            TinkerCommand::Save(_) => "save",
            TinkerCommand::Execute(_) => "execute",
            TinkerCommand::Help => "help",
        }
    }

    /// Get all available command names
    pub fn all_commands() -> Vec<&'static str> {
        vec![
            "helpers",
            "models",
            "routes",
            "config",
            "env",
            "clear",
            "exit",
            "quit",
            "history",
            "save",
            "help",
        ]
    }

    /// Get command description
    pub fn description(&self) -> &str {
        match self {
            TinkerCommand::Helpers => "Show all available helper functions",
            TinkerCommand::Models => "List all available models",
            TinkerCommand::Routes => "Show all registered routes",
            TinkerCommand::Config(_) => "Show configuration value",
            TinkerCommand::Env(_) => "Show environment variable",
            TinkerCommand::Clear => "Clear the screen",
            TinkerCommand::Exit => "Exit tinker REPL",
            TinkerCommand::History => "Show command history",
            TinkerCommand::Save(_) => "Save session as executable script",
            TinkerCommand::Execute(_) => "Execute code",
            TinkerCommand::Help => "Show help information",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_helpers() {
        let cmd = TinkerCommand::parse("helpers");
        assert_eq!(cmd, TinkerCommand::Helpers);
    }

    #[test]
    fn test_parse_config() {
        let cmd = TinkerCommand::parse("config database.host");
        assert_eq!(cmd, TinkerCommand::Config("database.host".to_string()));
    }

    #[test]
    fn test_parse_env() {
        let cmd = TinkerCommand::parse("env DATABASE_URL");
        assert_eq!(cmd, TinkerCommand::Env("DATABASE_URL".to_string()));
    }

    #[test]
    fn test_parse_save() {
        let cmd = TinkerCommand::parse("save my_session");
        assert_eq!(cmd, TinkerCommand::Save("my_session".to_string()));
    }

    #[test]
    fn test_parse_exit_variants() {
        assert_eq!(TinkerCommand::parse("exit"), TinkerCommand::Exit);
        assert_eq!(TinkerCommand::parse("quit"), TinkerCommand::Exit);
        assert_eq!(TinkerCommand::parse("q"), TinkerCommand::Exit);
    }

    #[test]
    fn test_parse_execute() {
        let cmd = TinkerCommand::parse("let x = 42;");
        match cmd {
            TinkerCommand::Execute(code) => assert_eq!(code, "let x = 42;"),
            _ => panic!("Expected Execute command"),
        }
    }

    #[test]
    fn test_all_commands() {
        let commands = TinkerCommand::all_commands();
        assert!(commands.contains(&"helpers"));
        assert!(commands.contains(&"models"));
        assert!(commands.contains(&"config"));
    }
}
