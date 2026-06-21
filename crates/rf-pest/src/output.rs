//! Test output formatting

use colored::*;
use std::time::Duration;

/// Test result enum
#[derive(Debug, Clone)]
pub enum TestResult {
    Passed(String, Duration),
    Failed(String, String),
    Skipped(String),
}

/// Test output formatter
pub struct TestOutput {
    #[allow(dead_code)] // reserved: indentation level for future nested output
    indent: usize,
}

impl TestOutput {
    /// Create a new output formatter
    pub fn new() -> Self {
        Self { indent: 2 }
    }

    /// Print test suite header
    pub fn print_header(&self, count: usize) {
        println!();
        println!(
            "{}",
            "  ╔══════════════════════════════════════════════╗".cyan()
        );
        println!(
            "{}",
            "  ║          RustForge Pest Test Suite           ║".cyan()
        );
        println!(
            "{}",
            "  ╚══════════════════════════════════════════════╝".cyan()
        );
        println!();
        println!(
            "  {} {} tests to run",
            "▶".cyan(),
            count.to_string().yellow()
        );
        println!();
    }

    /// Print a passed test
    pub fn print_passed(&self, name: &str, duration: Duration) {
        let duration_str = format!("({:.2}ms)", duration.as_secs_f64() * 1000.0);
        println!(
            "  {} {} {}",
            "✓".green(),
            name,
            duration_str.dimmed()
        );
    }

    /// Print a failed test
    pub fn print_failed(&self, name: &str, error: &str) {
        println!("  {} {}", "✗".red(), name.red());
        println!("    {}", error.red().dimmed());
    }

    /// Print a skipped test
    pub fn print_skipped(&self, name: &str) {
        println!("  {} {}", "○".yellow(), name.dimmed());
    }

    /// Print test summary
    pub fn print_summary(&self, passed: usize, failed: usize, skipped: usize, duration: Duration) {
        println!();
        println!("  {}", "─".repeat(50).dimmed());
        println!();

        let status = if failed == 0 {
            "PASSED".green().bold()
        } else {
            "FAILED".red().bold()
        };

        println!("  Tests:  {}", status);
        println!();

        if passed > 0 {
            println!(
                "    {} {} passed",
                "✓".green(),
                passed.to_string().green()
            );
        }

        if failed > 0 {
            println!(
                "    {} {} failed",
                "✗".red(),
                failed.to_string().red()
            );
        }

        if skipped > 0 {
            println!(
                "    {} {} skipped",
                "○".yellow(),
                skipped.to_string().yellow()
            );
        }

        println!();
        println!(
            "  Duration: {}",
            format!("{:.2}s", duration.as_secs_f64()).cyan()
        );
        println!();
    }

    /// Print a group header
    pub fn print_group(&self, name: &str) {
        println!();
        println!("  {} {}", "▸".cyan(), name.bold());
    }

    /// Print progress bar (for long-running tests)
    pub fn print_progress(&self, current: usize, total: usize) {
        let percentage = if total > 0 { (current as f64 / total as f64 * 100.0) as usize } else { 0 };
        let filled = percentage / 5;
        let empty = 20 - filled;

        print!(
            "\r  [{}{}] {}%",
            "█".repeat(filled).green(),
            "░".repeat(empty).dimmed(),
            percentage
        );

        if current == total {
            println!();
        }
    }
}

impl Default for TestOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_creation() {
        let output = TestOutput::new();
        assert_eq!(output.indent, 2);
    }
}
