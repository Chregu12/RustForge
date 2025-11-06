//! Command history management for Tinker REPL

use anyhow::Result;
use rustyline::history::{DefaultHistory, History};
use std::fs;
use std::path::{Path, PathBuf};

/// Tinker command history manager
pub struct TinkerHistory {
    history_path: PathBuf,
    history: DefaultHistory,
}

impl TinkerHistory {
    /// Create a new history manager
    pub fn new(history_path: PathBuf) -> Self {
        Self {
            history_path,
            history: DefaultHistory::new(),
        }
    }

    /// Load history from file
    pub fn load(&mut self) -> Result<()> {
        if self.history_path.exists() {
            let contents = fs::read_to_string(&self.history_path)?;
            for line in contents.lines() {
                if !line.is_empty() {
                    let _ = self.history.add(line);
                }
            }
        }
        Ok(())
    }

    /// Save history to file
    pub fn save(&self) -> Result<()> {
        if let Some(parent) = self.history_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut contents = String::new();
        for entry in self.history.iter() {
            contents.push_str(entry);
            contents.push('\n');
        }

        fs::write(&self.history_path, contents)?;
        Ok(())
    }

    /// Add entry to history
    pub fn add(&mut self, line: &str) -> Result<()> {
        let _ = self.history.add(line);
        Ok(())
    }

    /// Get all history entries
    pub fn entries(&self) -> Vec<String> {
        self.history
            .iter()
            .map(|entry| entry.to_string())
            .collect()
    }

    /// Get history size
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Check if history is empty
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Clear all history and remove history file
    pub fn clear_with_file(&mut self) -> Result<()> {
        self.history.clear();
        if self.history_path.exists() {
            fs::remove_file(&self.history_path)?;
        }
        Ok(())
    }

    /// Get last N entries
    pub fn last_n(&self, n: usize) -> Vec<String> {
        let entries = self.entries();
        let start = if entries.len() > n {
            entries.len() - n
        } else {
            0
        };
        entries[start..].to_vec()
    }

    /// Get history path
    pub fn path(&self) -> &Path {
        &self.history_path
    }
}

// Note: TinkerHistory wraps DefaultHistory for file persistence.
// For use with rustyline Editor, use DefaultHistory directly.

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_new_history() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");
        let history = TinkerHistory::new(history_path);
        assert!(history.is_empty());
    }

    #[test]
    fn test_add_entry() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");
        let mut history = TinkerHistory::new(history_path);

        history.add("helpers").unwrap();
        history.add("models").unwrap();

        assert_eq!(history.len(), 2);
        assert!(!history.is_empty());
    }

    #[test]
    fn test_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");

        {
            let mut history = TinkerHistory::new(history_path.clone());
            history.add("command1").unwrap();
            history.add("command2").unwrap();
            history.save().unwrap();
        }

        {
            let mut history = TinkerHistory::new(history_path.clone());
            history.load().unwrap();
            assert_eq!(history.len(), 2);
        }
    }

    #[test]
    fn test_last_n() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");
        let mut history = TinkerHistory::new(history_path);

        for i in 0..10 {
            history.add(&format!("command{}", i)).unwrap();
        }

        let last_5 = history.last_n(5);
        assert_eq!(last_5.len(), 5);
        assert_eq!(last_5[4], "command9");
    }

    #[test]
    fn test_clear() {
        let temp_dir = TempDir::new().unwrap();
        let history_path = temp_dir.path().join("history");
        let mut history = TinkerHistory::new(history_path);

        history.add("command1").unwrap();
        history.add("command2").unwrap();
        history.save().unwrap();

        history.clear_with_file().unwrap();
        assert!(history.is_empty());
    }
}
