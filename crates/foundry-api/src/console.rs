/// Console output utility for commands with verbosity support
///
/// Provides a structured way to output messages with automatic handling of verbosity levels.
///
/// # Example
///
/// ```rust
/// use foundry_api::console::Console;
/// use foundry_api::VerbosityLevel;
///
/// let console = Console::new(VerbosityLevel::Verbose);
///
/// console.line("This is always shown");
/// console.info("This is shown unless quiet");
/// console.verbose("This is shown with -v");
/// console.debug("This is shown with -vvv");
/// ```

use crate::VerbosityLevel;
use colored::Colorize;
use std::fmt;

/// Console for structured command output with verbosity support
#[derive(Clone)]
pub struct Console {
    verbosity: VerbosityLevel,
}

impl Console {
    /// Create a new console with the given verbosity level
    pub fn new(verbosity: VerbosityLevel) -> Self {
        Self { verbosity }
    }

    /// Create a console with normal verbosity
    pub fn normal() -> Self {
        Self::new(VerbosityLevel::Normal)
    }

    /// Create a console with verbose output
    pub fn verbose() -> Self {
        Self::new(VerbosityLevel::Verbose)
    }

    /// Create a console with debug output
    pub fn debug() -> Self {
        Self::new(VerbosityLevel::Debug)
    }

    /// Get the current verbosity level
    pub fn verbosity(&self) -> VerbosityLevel {
        self.verbosity
    }

    /// Set the verbosity level
    pub fn set_verbosity(&mut self, verbosity: VerbosityLevel) {
        self.verbosity = verbosity;
    }

    /// Output a line unconditionally
    pub fn line<S: AsRef<str>>(&self, message: S) {
        println!("{}", message.as_ref());
    }

    /// Output normal information (shown unless quiet)
    pub fn info<S: AsRef<str>>(&self, message: S) {
        if !self.verbosity.is_quiet() {
            println!("{}", message.as_ref());
        }
    }

    /// Output success message
    pub fn success<S: AsRef<str>>(&self, message: S) {
        if !self.verbosity.is_quiet() {
            println!("{}", message.as_ref().green());
        }
    }

    /// Output warning message
    pub fn warn<S: AsRef<str>>(&self, message: S) {
        if !self.verbosity.is_quiet() {
            eprintln!("{}", message.as_ref().yellow());
        }
    }

    /// Output error message
    pub fn error<S: AsRef<str>>(&self, message: S) {
        eprintln!("{}", message.as_ref().red());
    }

    /// Output verbose message (shown with -v or higher)
    pub fn verbose<S: AsRef<str>>(&self, message: S) {
        if self.verbosity.is_verbose() {
            println!("{}", message.as_ref().cyan());
        }
    }

    /// Output very verbose message (shown with -vv or higher)
    pub fn very_verbose<S: AsRef<str>>(&self, message: S) {
        if self.verbosity.is_very_verbose() {
            println!("{}", message.as_ref().bright_cyan());
        }
    }

    /// Output debug message (shown with -vvv)
    pub fn debug<S: AsRef<str>>(&self, message: S) {
        if self.verbosity.is_debug() {
            println!("{}", format!("[DEBUG] {}", message.as_ref()).magenta());
        }
    }

    /// Output a formatted section header
    pub fn section<S: AsRef<str>>(&self, title: S) {
        if !self.verbosity.is_quiet() {
            let title = title.as_ref();
            println!("\n{}", title.bold().underline());
        }
    }

    /// Output a formatted key-value pair
    pub fn item<S1: AsRef<str>, S2: AsRef<str>>(&self, key: S1, value: S2) {
        if !self.verbosity.is_quiet() {
            println!(
                "{}: {}",
                key.as_ref().bold(),
                value.as_ref()
            );
        }
    }

    /// Output a list item
    pub fn list_item<S: AsRef<str>>(&self, item: S) {
        if !self.verbosity.is_quiet() {
            println!("  • {}", item.as_ref());
        }
    }

    /// Output a table row
    pub fn table_row(&self, columns: &[&str]) {
        if !self.verbosity.is_quiet() {
            println!("{}", columns.join(" | "));
        }
    }

    /// Output an empty line
    pub fn blank(&self) {
        if !self.verbosity.is_quiet() {
            println!();
        }
    }

    /// Output a line only if condition is true
    pub fn line_if<S: AsRef<str>>(&self, condition: bool, message: S) {
        if condition {
            self.line(message);
        }
    }

    /// Output info only if condition is true
    pub fn info_if<S: AsRef<str>>(&self, condition: bool, message: S) {
        if condition {
            self.info(message);
        }
    }

    /// Output verbose only if condition is true
    pub fn verbose_if<S: AsRef<str>>(&self, condition: bool, message: S) {
        if condition {
            self.verbose(message);
        }
    }

    /// Format a progress message
    pub fn progress<S: AsRef<str>>(&self, current: usize, total: usize, message: S) -> String {
        format!(
            "[{}/{}] {}",
            current,
            total,
            message.as_ref()
        )
    }

    /// Check if output is quiet
    pub fn is_quiet(&self) -> bool {
        self.verbosity.is_quiet()
    }

    /// Check if output should be shown
    pub fn is_visible(&self) -> bool {
        !self.is_quiet()
    }

    /// Check if verbose output should be shown
    pub fn is_verbose(&self) -> bool {
        self.verbosity.is_verbose()
    }

    /// Check if debug output should be shown
    pub fn is_debug(&self) -> bool {
        self.verbosity.is_debug()
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::normal()
    }
}

impl fmt::Debug for Console {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Console")
            .field("verbosity", &self.verbosity)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_creation() {
        let console = Console::new(VerbosityLevel::Verbose);
        assert_eq!(console.verbosity(), VerbosityLevel::Verbose);
    }

    #[test]
    fn test_console_visibility() {
        let quiet = Console::new(VerbosityLevel::Quiet);
        assert!(quiet.is_quiet());
        assert!(!quiet.is_visible());

        let normal = Console::new(VerbosityLevel::Normal);
        assert!(!normal.is_quiet());
        assert!(normal.is_visible());
        assert!(!normal.is_verbose());

        let verbose = Console::new(VerbosityLevel::Verbose);
        assert!(verbose.is_verbose());
        assert!(!verbose.is_debug());

        let debug = Console::new(VerbosityLevel::Debug);
        assert!(debug.is_debug());
    }

    #[test]
    fn test_console_setverbosity() {
        let mut console = Console::normal();
        assert_eq!(console.verbosity(), VerbosityLevel::Normal);

        console.set_verbosity(VerbosityLevel::Debug);
        assert_eq!(console.verbosity(), VerbosityLevel::Debug);
    }

    #[test]
    fn test_console_progress_format() {
        let console = Console::normal();
        let progress = console.progress(3, 10, "Processing");
        assert_eq!(progress, "[3/10] Processing");
    }

    #[test]
    fn test_console_clone() {
        let console = Console::new(VerbosityLevel::VeryVerbose);
        let cloned = console.clone();
        assert_eq!(cloned.verbosity(), console.verbosity());
    }
}
