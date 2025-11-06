//! Tab completion for Tinker REPL

use crate::command::TinkerCommand;
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Helper, Context};
use rustyline::Result as RustylineResult;

/// Tab completer for Tinker
pub struct TinkerCompleter {
    commands: Vec<String>,
    models: Vec<String>,
    helpers: Vec<String>,
}

impl TinkerCompleter {
    /// Create a new completer with default commands
    pub fn new() -> Self {
        Self {
            commands: TinkerCommand::all_commands()
                .iter()
                .map(|s| s.to_string())
                .collect(),
            models: Vec::new(),
            helpers: vec![
                "now()".to_string(),
                "env()".to_string(),
                "config()".to_string(),
                "cache_get()".to_string(),
                "cache_put()".to_string(),
                "db_query()".to_string(),
                "dd()".to_string(),
            ],
        }
    }

    /// Add model names for completion
    pub fn add_models(&mut self, models: Vec<String>) {
        self.models.extend(models);
    }

    /// Add helper functions for completion
    pub fn add_helpers(&mut self, helpers: Vec<String>) {
        self.helpers.extend(helpers);
    }

    /// Get all completions
    fn get_completions(&self, line: &str, pos: usize) -> Vec<Pair> {
        let mut completions = Vec::new();
        let start = &line[..pos];

        // Complete commands
        for cmd in &self.commands {
            if cmd.starts_with(start) {
                completions.push(Pair {
                    display: cmd.clone(),
                    replacement: cmd.clone(),
                });
            }
        }

        // Complete helpers
        for helper in &self.helpers {
            if helper.starts_with(start) {
                completions.push(Pair {
                    display: helper.clone(),
                    replacement: helper.clone(),
                });
            }
        }

        // Complete models
        for model in &self.models {
            if model.starts_with(start) {
                completions.push(Pair {
                    display: model.clone(),
                    replacement: model.clone(),
                });
            }
        }

        completions
    }
}

impl Default for TinkerCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for TinkerCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> RustylineResult<(usize, Vec<Pair>)> {
        let completions = self.get_completions(line, pos);
        Ok((0, completions))
    }
}

// Implement Helper trait (required by rustyline 14.0)
impl Helper for TinkerCompleter {}

// Implement required traits for Helper
impl Hinter for TinkerCompleter {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Highlighter for TinkerCompleter {}

impl Validator for TinkerCompleter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completer_new() {
        let completer = TinkerCompleter::new();
        assert!(!completer.commands.is_empty());
        assert!(!completer.helpers.is_empty());
    }

    #[test]
    fn test_add_models() {
        let mut completer = TinkerCompleter::new();
        completer.add_models(vec!["User".to_string(), "Post".to_string()]);
        assert_eq!(completer.models.len(), 2);
    }

    #[test]
    fn test_get_completions() {
        let completer = TinkerCompleter::new();
        let completions = completer.get_completions("help", 4);
        assert!(!completions.is_empty());
    }

    #[test]
    fn test_complete_helpers() {
        let completer = TinkerCompleter::new();
        let completions = completer.get_completions("now", 3);
        assert!(completions.iter().any(|p| p.display == "now()"));
    }
}
