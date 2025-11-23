//! Console output formatting and styling for Foundry CLI
//!
//! This crate provides beautiful, Laravel-inspired console output with:
//! - Colored text and backgrounds
//! - Tables with borders and formatting
//! - Progress bars and spinners
//! - Styled sections (info, success, warning, error)
//! - Panels and boxes

mod colors;
mod list;
mod panel;
mod progress;
mod sections;
mod spinner;
mod styled;
mod table;

pub use colors::{Color, Colorize, Style};
pub use list::{List, ListStyle};
pub use panel::{Panel, PanelStyle};
pub use progress::{ProgressBar, ProgressStyle};
pub use sections::{debug, error, header, info, line, success, warning};
pub use spinner::{Spinner, SpinnerStyle};
pub use styled::{bold, dim, italic, underline};
pub use table::{BorderStyle, Table, TableCell, TableRow};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_colorize() {
        let text = "Hello".green();
        assert!(text.contains("Hello"));
    }
}
