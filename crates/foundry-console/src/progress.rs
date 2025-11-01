use std::io::{self, Write};

#[derive(Debug, Clone, Copy)]
pub enum ProgressStyle {
    Bar,
    Dots,
    Line,
}

pub struct ProgressBar {
    total: usize,
    current: usize,
    style: ProgressStyle,
    message: String,
    width: usize,
}

impl ProgressBar {
    pub fn new(total: usize) -> Self {
        Self {
            total,
            current: 0,
            style: ProgressStyle::Bar,
            message: String::new(),
            width: 50,
        }
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn with_style(mut self, style: ProgressStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = width;
        self
    }

    pub fn set(&mut self, current: usize) {
        self.current = current.min(self.total);
        self.render();
    }

    pub fn inc(&mut self, delta: usize) {
        self.current = (self.current + delta).min(self.total);
        self.render();
    }

    pub fn finish(&mut self) {
        self.current = self.total;
        self.render();
        println!(); // New line after completion
    }

    pub fn finish_with_message(&mut self, message: &str) {
        self.current = self.total;
        self.message = message.to_string();
        self.render();
        println!(); // New line after completion
    }

    fn render(&self) {
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            100
        };

        let bar = match self.style {
            ProgressStyle::Bar => self.render_bar(),
            ProgressStyle::Dots => self.render_dots(),
            ProgressStyle::Line => self.render_line(),
        };

        let msg = if !self.message.is_empty() {
            format!(" {}", self.message)
        } else {
            String::new()
        };

        print!("\r{} {:>3}% ({}/{}){}",
            bar,
            percentage,
            self.current,
            self.total,
            msg
        );

        io::stdout().flush().unwrap_or(());
    }

    fn render_bar(&self) -> String {
        let filled = if self.total > 0 {
            (self.current as f64 / self.total as f64 * self.width as f64) as usize
        } else {
            self.width
        };

        let empty = self.width - filled;

        format!("[{}{}]",
            "█".repeat(filled),
            "░".repeat(empty)
        )
    }

    fn render_dots(&self) -> String {
        let filled = if self.total > 0 {
            (self.current as f64 / self.total as f64 * self.width as f64) as usize
        } else {
            self.width
        };

        let empty = self.width - filled;

        format!("[{}{}]",
            "●".repeat(filled),
            "○".repeat(empty)
        )
    }

    fn render_line(&self) -> String {
        let filled = if self.total > 0 {
            (self.current as f64 / self.total as f64 * self.width as f64) as usize
        } else {
            self.width
        };

        let empty = self.width - filled;

        format!("[{}{}]",
            "=".repeat(filled),
            "-".repeat(empty)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_bar_creation() {
        let bar = ProgressBar::new(100);
        assert_eq!(bar.total, 100);
        assert_eq!(bar.current, 0);
    }

    #[test]
    fn test_progress_bar_increment() {
        let mut bar = ProgressBar::new(100);
        bar.inc(10);
        assert_eq!(bar.current, 10);
    }

    #[test]
    fn test_progress_bar_bounds() {
        let mut bar = ProgressBar::new(100);
        bar.set(150); // Should cap at 100
        assert_eq!(bar.current, 100);
    }
}
