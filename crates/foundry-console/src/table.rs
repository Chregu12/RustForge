use crate::colors::Colorize;

#[derive(Debug, Clone, Copy)]
pub enum BorderStyle {
    Single,
    Double,
    Rounded,
    None,
}

impl BorderStyle {
    fn chars(&self) -> TableChars {
        match self {
            BorderStyle::Single => TableChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
                cross: '┼',
                left_t: '├',
                right_t: '┤',
                top_t: '┬',
                bottom_t: '┴',
            },
            BorderStyle::Double => TableChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                horizontal: '═',
                vertical: '║',
                cross: '╬',
                left_t: '╠',
                right_t: '╣',
                top_t: '╦',
                bottom_t: '╩',
            },
            BorderStyle::Rounded => TableChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                horizontal: '─',
                vertical: '│',
                cross: '┼',
                left_t: '├',
                right_t: '┤',
                top_t: '┬',
                bottom_t: '┴',
            },
            BorderStyle::None => TableChars {
                top_left: ' ',
                top_right: ' ',
                bottom_left: ' ',
                bottom_right: ' ',
                horizontal: ' ',
                vertical: ' ',
                cross: ' ',
                left_t: ' ',
                right_t: ' ',
                top_t: ' ',
                bottom_t: ' ',
            },
        }
    }
}

struct TableChars {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    horizontal: char,
    vertical: char,
    cross: char,
    left_t: char,
    right_t: char,
    top_t: char,
    bottom_t: char,
}

#[derive(Debug, Clone)]
pub struct TableCell {
    pub content: String,
    pub align: Alignment,
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
}

impl TableCell {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            align: Alignment::Left,
        }
    }

    pub fn with_align(mut self, align: Alignment) -> Self {
        self.align = align;
        self
    }

    fn render(&self, width: usize) -> String {
        let content = strip_ansi(&self.content);
        let ansi_overhead = self.content.len() - content.len();

        match self.align {
            Alignment::Left => format!("{:<width$}", self.content, width = width + ansi_overhead),
            Alignment::Center => {
                let padding = width.saturating_sub(content.len());
                let left_pad = padding / 2;
                let right_pad = padding - left_pad;
                format!("{}{}{}", " ".repeat(left_pad), self.content, " ".repeat(right_pad))
            }
            Alignment::Right => format!("{:>width$}", self.content, width = width + ansi_overhead),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TableRow {
    pub cells: Vec<TableCell>,
}

impl TableRow {
    pub fn new(cells: Vec<TableCell>) -> Self {
        Self { cells }
    }

    pub fn from_strings(strings: Vec<String>) -> Self {
        Self {
            cells: strings.into_iter().map(TableCell::new).collect(),
        }
    }
}

pub struct Table {
    headers: Option<TableRow>,
    rows: Vec<TableRow>,
    border_style: BorderStyle,
    title: Option<String>,
}

impl Table {
    pub fn new() -> Self {
        Self {
            headers: None,
            rows: Vec::new(),
            border_style: BorderStyle::Single,
            title: None,
        }
    }

    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = Some(TableRow::from_strings(headers));
        self
    }

    pub fn add_row(&mut self, row: TableRow) {
        self.rows.push(row);
    }

    pub fn add_row_strings(&mut self, cells: Vec<String>) {
        self.rows.push(TableRow::from_strings(cells));
    }

    pub fn border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn render(&self) -> String {
        let chars = self.border_style.chars();
        let col_count = self.column_count();
        let col_widths = self.calculate_column_widths();

        let mut output = Vec::new();

        // Top border
        output.push(self.render_border(&chars, &col_widths, true));

        // Title
        if let Some(title) = &self.title {
            let total_width: usize = col_widths.iter().sum::<usize>() + (col_count - 1) * 3 + 2;
            let title_stripped = strip_ansi(title);
            let padding = total_width.saturating_sub(title_stripped.len() + 2);
            let left_pad = padding / 2;
            let right_pad = padding - left_pad;

            output.push(format!(
                "{}{}{}{}{}",
                chars.vertical,
                " ".repeat(left_pad),
                title.bold(),
                " ".repeat(right_pad),
                chars.vertical
            ));
            output.push(self.render_separator(&chars, &col_widths));
        }

        // Headers
        if let Some(headers) = &self.headers {
            output.push(self.render_row(headers, &col_widths, &chars));
            output.push(self.render_separator(&chars, &col_widths));
        }

        // Rows
        for (idx, row) in self.rows.iter().enumerate() {
            output.push(self.render_row(row, &col_widths, &chars));
            if idx < self.rows.len() - 1 {
                // Optional: Add separators between rows
                // output.push(self.render_separator(&chars, &col_widths));
            }
        }

        // Bottom border
        output.push(self.render_border(&chars, &col_widths, false));

        output.join("\n")
    }

    fn column_count(&self) -> usize {
        self.headers
            .as_ref()
            .map(|h| h.cells.len())
            .or_else(|| self.rows.first().map(|r| r.cells.len()))
            .unwrap_or(0)
    }

    fn calculate_column_widths(&self) -> Vec<usize> {
        let col_count = self.column_count();
        let mut widths = vec![0; col_count];

        // Check headers
        if let Some(headers) = &self.headers {
            for (idx, cell) in headers.cells.iter().enumerate() {
                widths[idx] = widths[idx].max(strip_ansi(&cell.content).len());
            }
        }

        // Check all rows
        for row in &self.rows {
            for (idx, cell) in row.cells.iter().enumerate() {
                if idx < widths.len() {
                    widths[idx] = widths[idx].max(strip_ansi(&cell.content).len());
                }
            }
        }

        widths
    }

    fn render_border(&self, chars: &TableChars, col_widths: &[usize], is_top: bool) -> String {
        let mut parts = Vec::new();
        parts.push((if is_top {
            chars.top_left
        } else {
            chars.bottom_left
        }).to_string());

        for (idx, width) in col_widths.iter().enumerate() {
            parts.push(chars.horizontal.to_string().repeat(width + 2));
            if idx < col_widths.len() - 1 {
                parts.push((if is_top {
                    chars.top_t
                } else {
                    chars.bottom_t
                }).to_string());
            }
        }

        parts.push((if is_top {
            chars.top_right
        } else {
            chars.bottom_right
        }).to_string());

        parts.concat()
    }

    fn render_separator(&self, chars: &TableChars, col_widths: &[usize]) -> String {
        let mut parts = Vec::new();
        parts.push(chars.left_t.to_string());

        for (idx, width) in col_widths.iter().enumerate() {
            parts.push(chars.horizontal.to_string().repeat(width + 2));
            if idx < col_widths.len() - 1 {
                parts.push(chars.cross.to_string());
            }
        }

        parts.push(chars.right_t.to_string());
        parts.concat()
    }

    fn render_row(&self, row: &TableRow, col_widths: &[usize], chars: &TableChars) -> String {
        let mut parts = Vec::new();
        parts.push(format!("{} ", chars.vertical));

        for (idx, cell) in row.cells.iter().enumerate() {
            let width = col_widths.get(idx).copied().unwrap_or(0);
            parts.push(cell.render(width));
            if idx < row.cells.len() - 1 {
                parts.push(format!(" {} ", chars.vertical));
            }
        }

        parts.push(format!(" {}", chars.vertical));
        parts.concat()
    }
}

impl Default for Table {
    fn default() -> Self {
        Self::new()
    }
}

/// Strip ANSI escape codes from a string for length calculation
fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip until 'm'
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_creation() {
        let table = Table::new()
            .with_headers(vec!["Name".to_string(), "Age".to_string()]);

        assert!(table.headers.is_some());
    }

    #[test]
    fn test_strip_ansi() {
        let colored = "\x1b[32mHello\x1b[0m";
        let stripped = strip_ansi(colored);
        assert_eq!(stripped, "Hello");
    }
}
