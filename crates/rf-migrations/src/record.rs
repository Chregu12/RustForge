//! Result types returned by migration operations.

use std::fmt;

/// Result of a single `run`, `rollback`, `reset`, `fresh`, or `refresh` call.
#[derive(Debug, Clone, Default)]
pub struct RunResult {
    /// Migrations that were successfully applied or reversed.
    pub applied: Vec<String>,
    /// Migrations that failed, together with their error messages.
    pub failed: Vec<(String, String)>,
    /// The batch number used for this run (0 for rollback operations).
    pub batch: u32,
}

impl RunResult {
    pub(crate) fn new(batch: u32) -> Self {
        Self {
            batch,
            applied: Vec::new(),
            failed: Vec::new(),
        }
    }

    /// `true` if no migration failed.
    pub fn is_ok(&self) -> bool {
        self.failed.is_empty()
    }

    /// Total number of migrations that were touched (success + failure).
    pub fn count(&self) -> usize {
        self.applied.len() + self.failed.len()
    }
}

impl fmt::Display for RunResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Batch {}: {} applied, {} failed",
            self.batch,
            self.applied.len(),
            self.failed.len()
        )
    }
}
