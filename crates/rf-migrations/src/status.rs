//! Per-migration status information.

use chrono::{DateTime, Utc};
use std::fmt;

/// Status of a single registered migration.
#[derive(Debug, Clone)]
pub struct MigrationStatus {
    /// Migration name (as returned by [`Migration::name`]).
    pub name: String,
    /// Whether the migration has been applied.
    pub applied: bool,
    /// Batch number in which the migration was applied (if applied).
    pub batch: Option<u32>,
    /// Timestamp at which the migration was applied (if applied).
    pub applied_at: Option<DateTime<Utc>>,
}

impl fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.applied {
            write!(
                f,
                "[✓] {} (batch: {}, at: {})",
                self.name,
                self.batch.unwrap_or(0),
                self.applied_at
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            )
        } else {
            write!(f, "[ ] {} (pending)", self.name)
        }
    }
}
