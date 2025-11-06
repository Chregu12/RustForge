//! Output capturing for command execution

use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// Output mode for command execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutputMode {
    /// Capture all output
    Capture,
    /// Pass through to stdout/stderr
    PassThrough,
    /// Suppress all output
    Silent,
}

/// Captured output from command execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CapturedOutput {
    /// Captured stdout lines
    pub stdout: Vec<String>,
    /// Captured stderr lines
    pub stderr: Vec<String>,
}

impl CapturedOutput {
    /// Create new empty captured output
    pub fn new() -> Self {
        Self {
            stdout: Vec::new(),
            stderr: Vec::new(),
        }
    }

    /// Add stdout line
    pub fn add_stdout(&mut self, line: String) {
        self.stdout.push(line);
    }

    /// Add stderr line
    pub fn add_stderr(&mut self, line: String) {
        self.stderr.push(line);
    }

    /// Get all stdout as string
    pub fn stdout_string(&self) -> String {
        self.stdout.join("\n")
    }

    /// Get all stderr as string
    pub fn stderr_string(&self) -> String {
        self.stderr.join("\n")
    }

    /// Check if any errors were captured
    pub fn has_errors(&self) -> bool {
        !self.stderr.is_empty()
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.stdout.len() + self.stderr.len()
    }
}

/// Output capture handler
#[derive(Clone)]
pub struct OutputCapture {
    inner: Arc<Mutex<CapturedOutput>>,
    mode: OutputMode,
}

impl OutputCapture {
    /// Create new output capture
    pub fn new(mode: OutputMode) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CapturedOutput::new())),
            mode,
        }
    }

    /// Create capture mode
    pub fn capture() -> Self {
        Self::new(OutputMode::Capture)
    }

    /// Create pass-through mode
    pub fn pass_through() -> Self {
        Self::new(OutputMode::PassThrough)
    }

    /// Create silent mode
    pub fn silent() -> Self {
        Self::new(OutputMode::Silent)
    }

    /// Capture stdout line
    pub fn capture_stdout(&self, line: String) {
        match self.mode {
            OutputMode::Capture => {
                if let Ok(mut output) = self.inner.lock() {
                    output.add_stdout(line);
                }
            }
            OutputMode::PassThrough => {
                println!("{}", line);
            }
            OutputMode::Silent => {
                // Do nothing
            }
        }
    }

    /// Capture stderr line
    pub fn capture_stderr(&self, line: String) {
        match self.mode {
            OutputMode::Capture => {
                if let Ok(mut output) = self.inner.lock() {
                    output.add_stderr(line);
                }
            }
            OutputMode::PassThrough => {
                eprintln!("{}", line);
            }
            OutputMode::Silent => {
                // Do nothing
            }
        }
    }

    /// Get captured output
    pub fn get_output(&self) -> CapturedOutput {
        self.inner
            .lock()
            .map(|output| output.clone())
            .unwrap_or_default()
    }

    /// Get output mode
    pub fn mode(&self) -> OutputMode {
        self.mode
    }
}

impl Default for OutputCapture {
    fn default() -> Self {
        Self::capture()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_captured_output() {
        let mut output = CapturedOutput::new();
        output.add_stdout("line1".to_string());
        output.add_stdout("line2".to_string());
        output.add_stderr("error1".to_string());

        assert_eq!(output.stdout.len(), 2);
        assert_eq!(output.stderr.len(), 1);
        assert_eq!(output.stdout_string(), "line1\nline2");
        assert_eq!(output.stderr_string(), "error1");
        assert!(output.has_errors());
        assert_eq!(output.line_count(), 3);
    }

    #[test]
    fn test_output_capture_capture_mode() {
        let capture = OutputCapture::capture();
        capture.capture_stdout("test1".to_string());
        capture.capture_stderr("error1".to_string());

        let output = capture.get_output();
        assert_eq!(output.stdout.len(), 1);
        assert_eq!(output.stderr.len(), 1);
    }

    #[test]
    fn test_output_capture_modes() {
        let capture = OutputCapture::capture();
        assert_eq!(capture.mode(), OutputMode::Capture);

        let pass = OutputCapture::pass_through();
        assert_eq!(pass.mode(), OutputMode::PassThrough);

        let silent = OutputCapture::silent();
        assert_eq!(silent.mode(), OutputMode::Silent);
    }
}
