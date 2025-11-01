use crate::colors::Colorize;
use crate::panel::{Panel, PanelStyle};

/// Print an info message
pub fn info(message: &str) {
    println!("{} {}", "ℹ".cyan().bold(), message);
}

/// Print a success message
pub fn success(message: &str) {
    println!("{} {}", "✓".green().bold(), message);
}

/// Print a warning message
pub fn warning(message: &str) {
    println!("{} {}", "⚠".yellow().bold(), message);
}

/// Print an error message
pub fn error(message: &str) {
    eprintln!("{} {}", "✗".red().bold(), message);
}

/// Print a debug message
pub fn debug(message: &str) {
    println!("{} {}", "⚙".bright_black().bold(), message);
}

/// Print a header/section title
pub fn header(message: &str) {
    println!("\n{}\n{}", message.bold().underline(), "=".repeat(message.len()));
}

/// Print a simple line
pub fn line(message: &str) {
    println!("{}", message);
}

/// Print a blank line
pub fn blank() {
    println!();
}

/// Print a section with a panel
pub fn section(title: &str, content: &str) {
    Panel::new(content)
        .with_title(title)
        .with_style(PanelStyle::Rounded)
        .print();
}

/// Print a comment (dimmed text)
pub fn comment(message: &str) {
    println!("{}", message.dim());
}

/// Print a question
pub fn question(message: &str) {
    println!("{} {}", "?".blue().bold(), message);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sections_compile() {
        // Just ensure the functions compile
        // We can't test actual output without capturing stdout
    }
}
