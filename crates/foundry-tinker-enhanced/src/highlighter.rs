//! Syntax highlighting for Tinker REPL

use colored::Colorize;
use rustyline::highlight::Highlighter;

/// Syntax highlighter for Tinker REPL
pub struct TinkerHighlighter;

impl TinkerHighlighter {
    /// Create a new highlighter
    pub fn new() -> Self {
        Self
    }

    /// Highlight a line of code
    pub fn highlight_line(&self, line: &str) -> String {
        // Simple highlighting for now - can be enhanced with syntect
        let mut result = String::new();
        let words: Vec<&str> = line.split_whitespace().collect();

        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                result.push(' ');
            }

            // Highlight keywords
            if is_rust_keyword(word) {
                result.push_str(&word.blue().to_string());
            }
            // Highlight commands
            else if is_tinker_command(word) {
                result.push_str(&word.green().to_string());
            }
            // Highlight strings
            else if word.starts_with('"') || word.starts_with('\'') {
                result.push_str(&word.yellow().to_string());
            }
            // Highlight numbers
            else if word.parse::<f64>().is_ok() {
                result.push_str(&word.cyan().to_string());
            }
            // Regular text
            else {
                result.push_str(word);
            }
        }

        result
    }
}

impl Default for TinkerHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter for TinkerHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> std::borrow::Cow<'l, str> {
        // For now, return the line as-is
        // Full syntax highlighting can be added with syntect
        std::borrow::Cow::Borrowed(line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        false
    }
}

/// Check if word is a Rust keyword
fn is_rust_keyword(word: &str) -> bool {
    matches!(
        word,
        "let"
            | "mut"
            | "fn"
            | "if"
            | "else"
            | "for"
            | "while"
            | "loop"
            | "match"
            | "return"
            | "pub"
            | "use"
            | "mod"
            | "struct"
            | "enum"
            | "trait"
            | "impl"
            | "async"
            | "await"
    )
}

/// Check if word is a Tinker command
fn is_tinker_command(word: &str) -> bool {
    matches!(
        word,
        "helpers"
            | "models"
            | "routes"
            | "config"
            | "env"
            | "clear"
            | "exit"
            | "quit"
            | "history"
            | "save"
            | "help"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_new() {
        let highlighter = TinkerHighlighter::new();
        let result = highlighter.highlight_line("helpers");
        assert!(!result.is_empty());
    }

    #[test]
    fn test_is_rust_keyword() {
        assert!(is_rust_keyword("let"));
        assert!(is_rust_keyword("fn"));
        assert!(is_rust_keyword("async"));
        assert!(!is_rust_keyword("foo"));
    }

    #[test]
    fn test_is_tinker_command() {
        assert!(is_tinker_command("helpers"));
        assert!(is_tinker_command("models"));
        assert!(is_tinker_command("config"));
        assert!(!is_tinker_command("foo"));
    }

    #[test]
    fn test_highlight_line() {
        let highlighter = TinkerHighlighter::new();
        let line = "let x = 42";
        let result = highlighter.highlight_line(line);
        assert!(!result.is_empty());
    }
}
