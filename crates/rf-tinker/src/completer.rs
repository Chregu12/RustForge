//! Tab completion for Tinker REPL

use rustyline::completion::{Completer, Pair};
use rustyline::Context;

/// Tab completer for Tinker commands and SQL keywords
pub struct TinkerCompleter {
    commands: Vec<String>,
    sql_keywords: Vec<String>,
    db_methods: Vec<String>,
}

impl TinkerCompleter {
    pub fn new() -> Self {
        Self {
            commands: vec![
                ".help".to_string(),
                ".exit".to_string(),
                ".quit".to_string(),
                ".tables".to_string(),
                ".schema".to_string(),
                ".databases".to_string(),
                ".clear".to_string(),
                ".reconnect".to_string(),
                ".env".to_string(),
                ".history".to_string(),
            ],
            sql_keywords: vec![
                "SELECT".to_string(),
                "FROM".to_string(),
                "WHERE".to_string(),
                "INSERT".to_string(),
                "INTO".to_string(),
                "VALUES".to_string(),
                "UPDATE".to_string(),
                "SET".to_string(),
                "DELETE".to_string(),
                "CREATE".to_string(),
                "TABLE".to_string(),
                "DROP".to_string(),
                "ALTER".to_string(),
                "JOIN".to_string(),
                "LEFT".to_string(),
                "RIGHT".to_string(),
                "INNER".to_string(),
                "OUTER".to_string(),
                "ON".to_string(),
                "AND".to_string(),
                "OR".to_string(),
                "NOT".to_string(),
                "IN".to_string(),
                "LIKE".to_string(),
                "ORDER".to_string(),
                "BY".to_string(),
                "ASC".to_string(),
                "DESC".to_string(),
                "LIMIT".to_string(),
                "OFFSET".to_string(),
                "GROUP".to_string(),
                "HAVING".to_string(),
                "COUNT".to_string(),
                "SUM".to_string(),
                "AVG".to_string(),
                "MIN".to_string(),
                "MAX".to_string(),
                "DISTINCT".to_string(),
                "AS".to_string(),
                "NULL".to_string(),
                "IS".to_string(),
                "BETWEEN".to_string(),
                "EXISTS".to_string(),
                "CASE".to_string(),
                "WHEN".to_string(),
                "THEN".to_string(),
                "ELSE".to_string(),
                "END".to_string(),
            ],
            db_methods: vec![
                "DB::table".to_string(),
                "DB::select".to_string(),
                "DB::insert".to_string(),
                "DB::update".to_string(),
                "DB::delete".to_string(),
                "DB::statement".to_string(),
                "DB::raw".to_string(),
                "DB::transaction".to_string(),
                ".get()".to_string(),
                ".first()".to_string(),
                ".count()".to_string(),
                ".where(".to_string(),
                ".whereIn(".to_string(),
                ".whereNull(".to_string(),
                ".whereNotNull(".to_string(),
                ".orWhere(".to_string(),
                ".orderBy(".to_string(),
                ".orderByDesc(".to_string(),
                ".limit(".to_string(),
                ".offset(".to_string(),
                ".join(".to_string(),
                ".leftJoin(".to_string(),
                ".select(".to_string(),
                ".distinct()".to_string(),
                ".groupBy(".to_string(),
                ".having(".to_string(),
                ".pluck(".to_string(),
                ".value(".to_string(),
                ".exists()".to_string(),
                ".doesntExist()".to_string(),
                "Cache::get".to_string(),
                "Cache::put".to_string(),
                "Cache::forget".to_string(),
                "Cache::flush".to_string(),
            ],
        }
    }

    fn get_completions(&self, word: &str) -> Vec<Pair> {
        let mut completions = Vec::new();
        let word_lower = word.to_lowercase();

        // Check meta commands
        if word.starts_with('.') {
            for cmd in &self.commands {
                if cmd.to_lowercase().starts_with(&word_lower) {
                    completions.push(Pair {
                        display: cmd.clone(),
                        replacement: cmd.clone(),
                    });
                }
            }
            return completions;
        }

        // Check DB methods
        if word.starts_with("DB::") || word.starts_with("db::") {
            for method in &self.db_methods {
                if method.to_lowercase().starts_with(&word_lower) {
                    completions.push(Pair {
                        display: method.clone(),
                        replacement: method.clone(),
                    });
                }
            }
            return completions;
        }

        // Check SQL keywords
        for keyword in &self.sql_keywords {
            if keyword.to_lowercase().starts_with(&word_lower) {
                completions.push(Pair {
                    display: keyword.clone(),
                    replacement: keyword.clone(),
                });
            }
        }

        // Check DB methods
        for method in &self.db_methods {
            if method.to_lowercase().starts_with(&word_lower) {
                completions.push(Pair {
                    display: method.clone(),
                    replacement: method.clone(),
                });
            }
        }

        completions
    }
}

impl Default for TinkerCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl Completer for TinkerCompleter {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        // Find the word being completed
        let line_up_to_pos = &line[..pos];
        let word_start = line_up_to_pos
            .rfind(|c: char| c.is_whitespace() || c == '(' || c == ',')
            .map(|i| i + 1)
            .unwrap_or(0);

        let word = &line[word_start..pos];
        let completions = self.get_completions(word);

        Ok((word_start, completions))
    }
}
