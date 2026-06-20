//! Command handler for meta commands

use std::collections::HashMap;

/// Handler for meta commands (starting with .)
pub struct CommandHandler {
    commands: HashMap<String, CommandInfo>,
}

/// Information about a command
#[derive(Clone)]
pub struct CommandInfo {
    pub name: String,
    pub aliases: Vec<String>,
    pub description: String,
    pub usage: String,
}

impl CommandHandler {
    pub fn new() -> Self {
        let mut commands = HashMap::new();

        commands.insert(
            ".help".to_string(),
            CommandInfo {
                name: ".help".to_string(),
                aliases: vec![".h".to_string(), ".?".to_string()],
                description: "Show available commands".to_string(),
                usage: ".help".to_string(),
            },
        );

        commands.insert(
            ".exit".to_string(),
            CommandInfo {
                name: ".exit".to_string(),
                aliases: vec![".quit".to_string(), ".q".to_string()],
                description: "Exit Tinker".to_string(),
                usage: ".exit".to_string(),
            },
        );

        commands.insert(
            ".tables".to_string(),
            CommandInfo {
                name: ".tables".to_string(),
                aliases: vec![],
                description: "List all database tables".to_string(),
                usage: ".tables".to_string(),
            },
        );

        commands.insert(
            ".schema".to_string(),
            CommandInfo {
                name: ".schema".to_string(),
                aliases: vec![],
                description: "Show table schema".to_string(),
                usage: ".schema <table_name>".to_string(),
            },
        );

        commands.insert(
            ".databases".to_string(),
            CommandInfo {
                name: ".databases".to_string(),
                aliases: vec![],
                description: "List available databases".to_string(),
                usage: ".databases".to_string(),
            },
        );

        commands.insert(
            ".clear".to_string(),
            CommandInfo {
                name: ".clear".to_string(),
                aliases: vec![],
                description: "Clear the screen".to_string(),
                usage: ".clear".to_string(),
            },
        );

        commands.insert(
            ".reconnect".to_string(),
            CommandInfo {
                name: ".reconnect".to_string(),
                aliases: vec![],
                description: "Reconnect to database".to_string(),
                usage: ".reconnect".to_string(),
            },
        );

        commands.insert(
            ".env".to_string(),
            CommandInfo {
                name: ".env".to_string(),
                aliases: vec![],
                description: "Show environment info".to_string(),
                usage: ".env".to_string(),
            },
        );

        Self { commands }
    }

    /// Get command info by name or alias
    pub fn get_command(&self, name: &str) -> Option<&CommandInfo> {
        // Direct lookup
        if let Some(cmd) = self.commands.get(name) {
            return Some(cmd);
        }

        // Search by alias
        self.commands.values().find(|&cmd| cmd.aliases.contains(&name.to_string())).map(|v| v as _)
    }

    /// Get all commands
    pub fn all_commands(&self) -> impl Iterator<Item = &CommandInfo> {
        self.commands.values()
    }
}

impl Default for CommandHandler {
    fn default() -> Self {
        Self::new()
    }
}
