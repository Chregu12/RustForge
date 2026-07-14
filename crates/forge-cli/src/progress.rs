#![allow(dead_code)] // utility helpers reserved for CLI commands, not all consumed yet
//! Progress indicators for long-running operations
//!
//! This module provides progress bars, spinners, and multi-progress for
//! operations like migrations, seeding, and code generation.

use console::style;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::time::Duration;

/// Create a progress bar for a determinate operation
pub fn create_progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .expect("Invalid progress bar template")
            .progress_chars("█▓▒░ "),
    );
    pb.set_message(message.to_string());
    pb
}

/// Create a spinner for indeterminate operations
pub fn create_spinner(message: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .expect("Invalid spinner template")
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(message.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Create a multi-progress container for parallel operations
pub fn create_multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Progress tracker for migrations
pub struct MigrationProgress {
    multi: MultiProgress,
    current: Option<ProgressBar>,
}

impl MigrationProgress {
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            current: None,
        }
    }

    /// Start a new migration
    pub fn start_migration(&mut self, name: &str) {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("├─ {spinner:.cyan} {msg}")
                .expect("Invalid template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(format!("{}", style(name).dim()));
        pb.enable_steady_tick(Duration::from_millis(80));
        self.current = Some(pb);
    }

    /// Complete the current migration
    pub fn complete_migration(&mut self, name: &str) {
        if let Some(pb) = self.current.take() {
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("├─ {msg}")
                    .expect("Invalid template"),
            );
            pb.finish_with_message(format!(
                "{} {} {}",
                style(name).dim(),
                style("█".repeat(12)).cyan(),
                style("100%").green().bold()
            ));
        }
    }

    /// Fail the current migration
    pub fn fail_migration(&mut self, name: &str, error: &str) {
        if let Some(pb) = self.current.take() {
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("├─ {msg}")
                    .expect("Invalid template"),
            );
            pb.finish_with_message(format!(
                "{} {} {}",
                style(name).dim(),
                style("✗").red().bold(),
                style(error).red()
            ));
        }
    }

    /// Finish all migrations
    pub fn finish(&self, count: usize, duration: Duration) {
        println!(
            "{} {} migrations completed in {:.1}s",
            style("✓").green().bold(),
            count,
            duration.as_secs_f64()
        );
    }
}

impl Default for MigrationProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress tracker for seeding operations
pub struct SeedProgress {
    spinner: ProgressBar,
}

impl SeedProgress {
    pub fn new(seeder_name: &str, count: usize) -> Self {
        let spinner = create_spinner(&format!("{} ({} records)...", seeder_name, count));
        Self { spinner }
    }

    /// Update the progress message
    pub fn update(&self, current: usize, total: usize) {
        self.spinner.set_message(format!(
            "Seeding... {}/{} records",
            style(current).cyan(),
            style(total).cyan()
        ));
    }

    /// Complete the seeding operation
    pub fn finish(&self, count: usize, duration: Duration) {
        self.spinner.finish_with_message(format!(
            "{} Seeded {} records in {:.1}s",
            style("✓").green().bold(),
            count,
            duration.as_secs_f64()
        ));
    }
}

/// Progress tracker for file generation operations
pub struct GenerationProgress {
    multi: MultiProgress,
    items: Vec<ProgressBar>,
}

impl GenerationProgress {
    pub fn new() -> Self {
        Self {
            multi: MultiProgress::new(),
            items: Vec::new(),
        }
    }

    /// Add a file generation task
    pub fn add_task(&mut self, filename: &str) -> usize {
        let pb = self.multi.add(ProgressBar::new_spinner());
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("  {spinner:.blue} {msg}")
                .expect("Invalid template")
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message(format!("Generating {}", style(filename).dim()));
        pb.enable_steady_tick(Duration::from_millis(80));

        self.items.push(pb);
        self.items.len() - 1
    }

    /// Complete a task
    pub fn complete_task(&mut self, index: usize, filename: &str) {
        if let Some(pb) = self.items.get(index) {
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("  {msg}")
                    .expect("Invalid template"),
            );
            pb.finish_with_message(format!(
                "{} Created: {}",
                style("✓").green().bold(),
                style(filename).cyan()
            ));
        }
    }

    /// Fail a task
    pub fn fail_task(&mut self, index: usize, filename: &str, error: &str) {
        if let Some(pb) = self.items.get(index) {
            pb.set_style(
                ProgressStyle::default_spinner()
                    .template("  {msg}")
                    .expect("Invalid template"),
            );
            pb.finish_with_message(format!(
                "{} Failed: {} - {}",
                style("✗").red().bold(),
                style(filename).cyan(),
                style(error).red()
            ));
        }
    }

    /// Clear all progress bars
    pub fn clear(&self) {
        for pb in &self.items {
            pb.finish_and_clear();
        }
    }
}

impl Default for GenerationProgress {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple progress bar wrapper for easy use
pub struct SimpleProgress {
    bar: ProgressBar,
}

impl SimpleProgress {
    pub fn new(total: u64, message: &str) -> Self {
        Self {
            bar: create_progress_bar(total, message),
        }
    }

    pub fn inc(&self, delta: u64) {
        self.bar.inc(delta);
    }

    pub fn set_message(&self, message: &str) {
        self.bar.set_message(message.to_string());
    }

    pub fn finish(&self) {
        self.bar
            .finish_with_message(format!("{} Complete", style("✓").green().bold()));
    }

    pub fn finish_with_message(&self, message: &str) {
        self.bar.finish_with_message(message.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_progress_bar() {
        let pb = create_progress_bar(100, "Testing");
        assert_eq!(pb.length().unwrap(), 100);
    }

    #[test]
    fn test_create_spinner() {
        let spinner = create_spinner("Loading...");
        assert!(spinner.is_hidden());
    }

    #[test]
    fn test_migration_progress_new() {
        let progress = MigrationProgress::new();
        assert!(progress.current.is_none());
    }

    #[test]
    fn test_migration_progress_default() {
        let progress = MigrationProgress::default();
        assert!(progress.current.is_none());
    }

    #[test]
    fn test_generation_progress_new() {
        let progress = GenerationProgress::new();
        assert_eq!(progress.items.len(), 0);
    }

    #[test]
    fn test_generation_progress_default() {
        let progress = GenerationProgress::default();
        assert_eq!(progress.items.len(), 0);
    }

    #[test]
    fn test_simple_progress_new() {
        let progress = SimpleProgress::new(50, "Testing");
        assert_eq!(progress.bar.length().unwrap(), 50);
    }

    #[test]
    fn test_seed_progress_creation() {
        let progress = SeedProgress::new("UserSeeder", 1000);
        // The spinner is active (not yet finished) right after construction.
        assert!(!progress.spinner.is_finished());
    }
}
