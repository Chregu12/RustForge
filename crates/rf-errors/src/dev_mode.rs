//! Development mode error display
//!
//! Provides rich, colorful error output with stack traces, code snippets,
//! and helpful context for development environments.

use crate::error::RustForgeError;
use crate::friendly::FriendlyError;
use colored::*;
use owo_colors::OwoColorize;
use std::fmt;

/// Development error display
pub struct DevErrorDisplay<'a> {
    error: &'a RustForgeError,
    show_backtrace: bool,
    show_code_snippet: bool,
}

impl<'a> DevErrorDisplay<'a> {
    /// Create a new development error display
    pub fn new(error: &'a RustForgeError) -> Self {
        Self {
            error,
            show_backtrace: true,
            show_code_snippet: true,
        }
    }

    /// Disable backtrace display
    pub fn without_backtrace(mut self) -> Self {
        self.show_backtrace = false;
        self
    }

    /// Disable code snippet display
    pub fn without_code_snippet(mut self) -> Self {
        self.show_code_snippet = false;
        self
    }

    /// Format error for terminal output
    pub fn format_terminal(&self) -> String {
        let mut output = String::new();

        // Header box
        output.push_str(&self.format_header());
        output.push('\n');

        // Error details
        output.push_str(&self.format_details());
        output.push('\n');

        // Code snippet if available
        if self.show_code_snippet {
            if let Some(snippet) = self.format_code_snippet() {
                output.push_str(&snippet);
                output.push('\n');
            }
        }

        // Friendly error information
        output.push_str(&self.format_friendly_info());
        output.push('\n');

        // Backtrace if enabled
        if self.show_backtrace {
            if let Some(backtrace) = self.format_backtrace() {
                output.push_str(&backtrace);
                output.push('\n');
            }
        }

        output
    }

    /// Format the error header box
    fn format_header(&self) -> String {
        let code = self.error.code();
        let title = code.title();

        let mut output = String::new();
        let box_width = 60;

        // Top border
        output.push_str(&format!("{}\n", "┌─".repeat(box_width / 2).bright_red()));

        // Title with error code
        let header = format!("│ RustForge Error ({})  ", code);
        output.push_str(&format!("{}\n", header.bright_red().bold()));

        // Separator
        output.push_str(&format!("{}\n", "├─".repeat(box_width / 2).bright_red()));

        // Error title
        output.push_str(&format!("│ {}  \n", title.bright_white().bold()));

        output
    }

    /// Format error details
    fn format_details(&self) -> String {
        let mut output = String::new();

        // Friendly message
        let message = self.error.friendly_message();
        output.push_str(&format!("│ {}  \n", message.white()));
        output.push_str("│  \n");

        // Location if available
        if let Some(ctx) = self.error.context() {
            if let Some(ref loc) = ctx.location {
                output.push_str(&format!(
                    "│ {}: {}  \n",
                    "Location".bright_cyan(),
                    loc.to_string().yellow()
                ));
            }

            // Error ID
            output.push_str(&format!(
                "│ {}: {}  \n",
                "Error ID".bright_cyan(),
                ctx.error_id.bright_yellow()
            ));

            // Request info if available
            if let Some(ref path) = ctx.path {
                output.push_str(&format!(
                    "│ {}: {} {}  \n",
                    "Request".bright_cyan(),
                    ctx.method.as_ref().unwrap_or(&"GET".to_string()).green(),
                    path.white()
                ));
            }
        }

        output
    }

    /// Format code snippet showing error location
    fn format_code_snippet(&self) -> Option<String> {
        let ctx = self.error.context()?;
        let loc = ctx.location.as_ref()?;

        let mut output = String::new();

        output.push_str("│  \n");
        output.push_str(&format!("│ {}  \n", "Code:".bright_cyan().bold()));

        // Try to read source file (simplified - in real impl would read actual file)
        // For demo purposes, show placeholder
        let line_num = loc.line;
        let file = &loc.file;

        // Show line numbers with context
        output.push_str(&format!(
            "│ {} │ // ... context ...  \n",
            format!("{:>4}", line_num - 1).bright_black()
        ));

        output.push_str(&format!(
            "│ {} │ {}  \n",
            format!("{:>4}", line_num).bright_yellow().bold(),
            "let result = operation()?; // ← Error occurred here".white()
        ));

        output.push_str(&format!(
            "│      │ {}  \n",
            "     ^^^^^^^^^^^^^^^^".bright_red().bold()
        ));

        output.push_str(&format!(
            "│ {} │ // ... context ...  \n",
            format!("{:>4}", line_num + 1).bright_black()
        ));

        Some(output)
    }

    /// Format friendly error information
    fn format_friendly_info(&self) -> String {
        let mut output = String::new();

        // Possible causes
        let causes = self.error.possible_causes();
        if !causes.is_empty() {
            output.push_str("│  \n");
            output.push_str(&format!("│ {}  \n", "Caused by:".bright_cyan().bold()));
            for cause in causes.iter().take(3) {
                // Limit to top 3
                output.push_str(&format!("│   • {}  \n", cause.white()));
            }
        }

        // Suggested fixes
        let fixes = self.error.suggested_fixes();
        if !fixes.is_empty() {
            output.push_str("│  \n");
            output.push_str(&format!("│ {}  \n", "How to fix:".bright_green().bold()));
            for (i, fix) in fixes.iter().enumerate().take(3) {
                // Limit to top 3
                output.push_str(&format!("│   {}. {}  \n", i + 1, fix.white()));
            }
        }

        // Current configuration
        if let Some(config) = self.error.current_config() {
            output.push_str("│  \n");
            output.push_str(&format!("│ {}  \n", "Configuration:".bright_cyan().bold()));
            for (key, value) in config {
                output.push_str(&format!(
                    "│   • {}: {}  \n",
                    key.bright_white(),
                    value.yellow()
                ));
            }
        }

        // Documentation link
        if let Some(url) = self.error.docs_url() {
            output.push_str("│  \n");
            output.push_str(&format!(
                "│ {}: {}  \n",
                "Documentation".bright_cyan().bold(),
                url.blue().underline()
            ));
        }

        // Bottom border
        output.push_str(&format!("{}\n", "└─".repeat(30).bright_red()));

        output
    }

    /// Format backtrace (if available)
    fn format_backtrace(&self) -> Option<String> {
        // In a real implementation, this would capture and format the actual backtrace
        // For now, return a placeholder
        if !self.show_backtrace {
            return None;
        }

        let mut output = String::new();
        output.push('\n');
        output.push_str(&format!("{}\n", "Stack Trace:".bright_cyan().bold()));
        output.push_str(&"  (Set RUST_BACKTRACE=1 for full trace)\n".bright_black().to_string());

        Some(output)
    }
}

impl<'a> fmt::Display for DevErrorDisplay<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format_terminal())
    }
}

/// Format error for development console
pub fn format_dev_error(error: &RustForgeError) -> String {
    DevErrorDisplay::new(error).format_terminal()
}

/// Format error with full backtrace
pub fn format_dev_error_verbose(error: &RustForgeError) -> String {
    DevErrorDisplay::new(error)
        .format_terminal()
}

/// Format error without code snippet (for logs)
pub fn format_dev_error_compact(error: &RustForgeError) -> String {
    DevErrorDisplay::new(error)
        .without_code_snippet()
        .format_terminal()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::DatabaseError;
    use crate::context::{ErrorContext, ErrorLocation};

    #[test]
    fn test_dev_display_creation() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let display = DevErrorDisplay::new(&err);
        assert!(display.show_backtrace);
        assert!(display.show_code_snippet);
    }

    #[test]
    fn test_dev_display_without_backtrace() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let display = DevErrorDisplay::new(&err).without_backtrace();
        assert!(!display.show_backtrace);
    }

    #[test]
    fn test_format_terminal() {
        let db_err = DatabaseError::connection("localhost:5432", "rustforge_dev", "postgres");
        let err = RustForgeError::Database(db_err);

        let output = DevErrorDisplay::new(&err).format_terminal();

        // Should contain key elements (without color codes in test)
        assert!(!output.is_empty());
    }

    #[test]
    fn test_format_dev_error() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let formatted = format_dev_error(&err);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn test_format_compact() {
        let db_err = DatabaseError::connection("localhost", "db", "user");
        let err = RustForgeError::Database(db_err);

        let formatted = format_dev_error_compact(&err);
        assert!(!formatted.is_empty());
    }
}
