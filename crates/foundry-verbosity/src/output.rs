use crate::level::VerbosityLevel;
use std::fmt;

/// Output manager for verbosity-aware printing
#[derive(Debug, Clone)]
pub struct Output {
    level: VerbosityLevel,
}

impl Output {
    /// Create a new output manager with the given verbosity level
    pub fn new(level: VerbosityLevel) -> Self {
        Self { level }
    }

    /// Get the current verbosity level
    pub fn level(&self) -> VerbosityLevel {
        self.level
    }

    /// Set the verbosity level
    pub fn set_level(&mut self, level: VerbosityLevel) {
        self.level = level;
    }

    /// Print an error message (always shown)
    pub fn error(&self, msg: impl fmt::Display) {
        if self.level.shows_error() {
            eprintln!("ERROR: {}", msg);
        }
    }

    /// Print a warning message (shown at normal and above)
    pub fn warning(&self, msg: impl fmt::Display) {
        if self.level.shows_normal() {
            println!("WARNING: {}", msg);
        }
    }

    /// Print a normal message (shown at normal and above)
    pub fn normal(&self, msg: impl fmt::Display) {
        if self.level.shows_normal() {
            println!("{}", msg);
        }
    }

    /// Print an info message (shown at -v and above)
    pub fn info(&self, msg: impl fmt::Display) {
        if self.level.shows_info() {
            println!("INFO: {}", msg);
        }
    }

    /// Print a debug message (shown at -vv and above)
    pub fn debug(&self, msg: impl fmt::Display) {
        if self.level.shows_debug() {
            println!("DEBUG: {}", msg);
        }
    }

    /// Print a trace message (shown at -vvv)
    pub fn trace(&self, msg: impl fmt::Display) {
        if self.level.shows_trace() {
            println!("TRACE: {}", msg);
        }
    }

    /// Print a success message (shown at normal and above)
    pub fn success(&self, msg: impl fmt::Display) {
        if self.level.shows_normal() {
            println!("✓ {}", msg);
        }
    }

    /// Execute a closure only if info messages are shown
    pub fn with_info<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.level.shows_info() {
            f();
        }
    }

    /// Execute a closure only if debug messages are shown
    pub fn with_debug<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.level.shows_debug() {
            f();
        }
    }

    /// Execute a closure only if trace messages are shown
    pub fn with_trace<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        if self.level.shows_trace() {
            f();
        }
    }
}

impl Default for Output {
    fn default() -> Self {
        Self::new(VerbosityLevel::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_creation() {
        let output = Output::new(VerbosityLevel::Verbose);
        assert_eq!(output.level(), VerbosityLevel::Verbose);
    }

    #[test]
    fn test_set_level() {
        let mut output = Output::new(VerbosityLevel::Normal);
        output.set_level(VerbosityLevel::Verbose);
        assert_eq!(output.level(), VerbosityLevel::Verbose);
    }

    #[test]
    fn test_default_output() {
        let output = Output::default();
        assert_eq!(output.level(), VerbosityLevel::Normal);
    }

    // Note: Testing actual output is difficult without capturing stdout/stderr
    // In a real scenario, you might want to use a testing framework that can
    // capture output or implement a custom writer for testing
}
