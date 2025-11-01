//! Console output formatting and styling for Foundry CLI
//!
//! This crate provides beautiful, Laravel-inspired console output with:
//! - Colored text and backgrounds
//! - Tables with borders and formatting
//! - Progress bars and spinners
//! - Styled sections (info, success, warning, error)
//! - Panels and boxes

mod colors;
mod table;
mod progress;
mod list;
mod panel;
mod spinner;
mod sections;
mod styled;

pub use colors::{Color, Style, Colorize};
pub use table::{Table, TableRow, TableCell, BorderStyle};
pub use progress::{ProgressBar, ProgressStyle};
pub use list::{List, ListStyle};
pub use panel::{Panel, PanelStyle};
pub use spinner::{Spinner, SpinnerStyle};
pub use sections::{info, success, warning, error, debug, header, line};
pub use styled::{bold, italic, underline, dim};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_colorize() {
        let text = "Hello".green();
        assert!(text.contains("Hello"));
    }
}
