//! Output formatting for REPL results

use crate::executor::ExecutionResult;
use colored::*;
use serde_json::Value;
// tabled is available for future use with proper table formatting

/// Output formatter for query results
pub struct OutputFormatter {
    /// Use table format for results
    pub table_format: bool,
    /// Max column width
    pub max_column_width: usize,
    /// Max rows to display
    pub max_rows: usize,
}

impl Default for OutputFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputFormatter {
    pub fn new() -> Self {
        Self {
            table_format: true,
            max_column_width: 50,
            max_rows: 100,
        }
    }

    /// Print an execution result
    pub fn print(&self, result: &ExecutionResult) {
        match result {
            ExecutionResult::Rows(rows) => self.print_rows(rows),
            ExecutionResult::Value(value) => self.print_value(value),
            ExecutionResult::Affected(count) => self.print_affected(*count),
            ExecutionResult::Message(msg) => self.print_message(msg),
            ExecutionResult::Error(err) => self.print_error(err),
            ExecutionResult::Empty => self.print_empty(),
        }
    }

    /// Print rows as a table
    fn print_rows(&self, rows: &[Value]) {
        if rows.is_empty() {
            self.print_empty();
            return;
        }

        println!();

        // Get column names from first row
        let columns: Vec<String> = if let Some(first) = rows.first() {
            if let Value::Object(obj) = first {
                obj.keys().cloned().collect()
            } else {
                vec!["value".to_string()]
            }
        } else {
            vec![]
        };

        if columns.is_empty() {
            // Print as JSON
            for row in rows.iter().take(self.max_rows) {
                println!("{}", serde_json::to_string_pretty(row).unwrap_or_default());
            }
            return;
        }

        // Build table data
        let mut table_data: Vec<Vec<String>> = Vec::new();

        for row in rows.iter().take(self.max_rows) {
            let mut row_data = Vec::new();
            for col in &columns {
                let value = if let Value::Object(obj) = row {
                    obj.get(col).cloned().unwrap_or(Value::Null)
                } else {
                    row.clone()
                };
                row_data.push(self.format_value(&value));
            }
            table_data.push(row_data);
        }

        // Print header
        print!("  ");
        for col in &columns {
            print!("{:<20} ", col.cyan());
        }
        println!();

        print!("  ");
        for _ in &columns {
            print!("{:<20} ", "-".repeat(18));
        }
        println!();

        // Print rows
        for row in &table_data {
            print!("  ");
            for cell in row {
                let truncated = if cell.len() > self.max_column_width {
                    format!("{}...", &cell[..self.max_column_width - 3])
                } else {
                    cell.clone()
                };
                print!("{:<20} ", truncated);
            }
            println!();
        }

        println!();

        // Print row count
        let displayed = std::cmp::min(rows.len(), self.max_rows);
        if rows.len() > self.max_rows {
            println!("  {} ({} of {} rows)",
                "Showing".dimmed(),
                displayed.to_string().yellow(),
                rows.len().to_string().yellow()
            );
        } else {
            println!("  {} {}",
                rows.len().to_string().green(),
                if rows.len() == 1 { "row" } else { "rows" }
            );
        }
        println!();
    }

    /// Format a single JSON value for display
    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Null => "NULL".dimmed().to_string(),
            Value::Bool(b) => if *b { "true".green() } else { "false".red() }.to_string(),
            Value::Number(n) => n.to_string().yellow().to_string(),
            Value::String(s) => s.clone(),
            Value::Array(arr) => format!("[{} items]", arr.len()),
            Value::Object(_) => "{...}".to_string(),
        }
    }

    /// Print a single value
    fn print_value(&self, value: &Value) {
        println!();
        match value {
            Value::Object(obj) => {
                for (key, val) in obj {
                    println!("  {}: {}", key.cyan(), self.format_value(val));
                }
            }
            _ => {
                println!("  => {}", serde_json::to_string_pretty(value).unwrap_or_default().green());
            }
        }
        println!();
    }

    /// Print affected rows count
    fn print_affected(&self, count: u64) {
        println!();
        println!("  {} {} affected",
            count.to_string().green(),
            if count == 1 { "row" } else { "rows" }
        );
        println!();
    }

    /// Print a message
    fn print_message(&self, msg: &str) {
        println!();
        println!("  {}", msg.yellow());
        println!();
    }

    /// Print an error
    fn print_error(&self, err: &str) {
        println!();
        println!("  {}: {}", "Error".red().bold(), err);
        println!();
    }

    /// Print empty result
    fn print_empty(&self) {
        println!();
        println!("  {}", "Empty result set".dimmed());
        println!();
    }
}
