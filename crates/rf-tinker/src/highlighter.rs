//! Syntax highlighting for Tinker REPL

use rustyline::highlight::Highlighter;
use std::borrow::Cow;

/// Syntax highlighter for SQL and DB facade calls
pub struct TinkerHighlighter {
    sql_keywords: Vec<&'static str>,
}

impl TinkerHighlighter {
    pub fn new() -> Self {
        Self {
            sql_keywords: vec![
                "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES",
                "UPDATE", "SET", "DELETE", "CREATE", "TABLE", "DROP",
                "ALTER", "JOIN", "LEFT", "RIGHT", "INNER", "OUTER",
                "ON", "AND", "OR", "NOT", "IN", "LIKE", "ORDER", "BY",
                "ASC", "DESC", "LIMIT", "OFFSET", "GROUP", "HAVING",
                "COUNT", "SUM", "AVG", "MIN", "MAX", "DISTINCT", "AS",
                "NULL", "IS", "BETWEEN", "EXISTS", "CASE", "WHEN",
                "THEN", "ELSE", "END", "TRUE", "FALSE",
            ],
        }
    }

    fn highlight_sql(&self, input: &str) -> String {
        let mut result = input.to_string();

        // Highlight SQL keywords (case-insensitive)
        for keyword in &self.sql_keywords {
            let pattern = format!(r"(?i)\b{}\b", keyword);
            if let Ok(regex) = regex::Regex::new(&pattern) {
                result = regex
                    .replace_all(&result, |caps: &regex::Captures| {
                        format!("\x1b[1;34m{}\x1b[0m", &caps[0])
                    })
                    .to_string();
            }
        }

        // Highlight strings
        if let Ok(regex) = regex::Regex::new(r#"'[^']*'|"[^"]*""#) {
            result = regex
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\x1b[32m{}\x1b[0m", &caps[0])
                })
                .to_string();
        }

        // Highlight numbers
        if let Ok(regex) = regex::Regex::new(r"\b\d+\.?\d*\b") {
            result = regex
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\x1b[33m{}\x1b[0m", &caps[0])
                })
                .to_string();
        }

        // Highlight DB:: facade
        if let Ok(regex) = regex::Regex::new(r"DB::") {
            result = regex
                .replace_all(&result, "\x1b[1;35mDB::\x1b[0m")
                .to_string();
        }

        // Highlight Cache:: facade
        if let Ok(regex) = regex::Regex::new(r"Cache::") {
            result = regex
                .replace_all(&result, "\x1b[1;35mCache::\x1b[0m")
                .to_string();
        }

        // Highlight meta commands
        if let Ok(regex) = regex::Regex::new(r"^\.[a-z]+") {
            result = regex
                .replace_all(&result, |caps: &regex::Captures| {
                    format!("\x1b[1;36m{}\x1b[0m", &caps[0])
                })
                .to_string();
        }

        result
    }
}

impl Default for TinkerHighlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter for TinkerHighlighter {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned(self.highlight_sql(line))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        true
    }

    fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
        &'s self,
        prompt: &'p str,
        _default: bool,
    ) -> Cow<'b, str> {
        Cow::Owned(format!("\x1b[1;36m{}\x1b[0m", prompt))
    }
}
