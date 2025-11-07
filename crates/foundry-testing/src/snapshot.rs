//! Snapshot testing utilities

use serde::Serialize;
use std::fs;
use std::path::PathBuf;

/// Snapshot testing helper
pub struct Snapshot {
    base_dir: PathBuf,
}

impl Snapshot {
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        fs::create_dir_all(&base_dir).ok();
        Self { base_dir }
    }

    pub fn assert_matches<T: Serialize>(
        &self,
        name: impl AsRef<str>,
        value: &T,
    ) -> anyhow::Result<()> {
        let snapshot_path = self.base_dir.join(format!("{}.json", name.as_ref()));
        let serialized = serde_json::to_string_pretty(value)?;

        if snapshot_path.exists() {
            let existing = fs::read_to_string(&snapshot_path)?;
            if existing != serialized {
                return Err(anyhow::anyhow!(
                    "Snapshot mismatch for '{}'\nExpected:\n{}\n\nActual:\n{}",
                    name.as_ref(),
                    existing,
                    serialized
                ));
            }
        } else {
            // Create new snapshot
            fs::write(&snapshot_path, &serialized)?;
            println!("Created new snapshot: {}", snapshot_path.display());
        }

        Ok(())
    }

    pub fn update<T: Serialize>(
        &self,
        name: impl AsRef<str>,
        value: &T,
    ) -> anyhow::Result<()> {
        let snapshot_path = self.base_dir.join(format!("{}.json", name.as_ref()));
        let serialized = serde_json::to_string_pretty(value)?;
        fs::write(&snapshot_path, &serialized)?;
        Ok(())
    }

    pub fn delete(&self, name: impl AsRef<str>) -> anyhow::Result<()> {
        let snapshot_path = self.base_dir.join(format!("{}.json", name.as_ref()));
        if snapshot_path.exists() {
            fs::remove_file(&snapshot_path)?;
        }
        Ok(())
    }

    pub fn clear_all(&self) -> anyhow::Result<()> {
        if self.base_dir.exists() {
            fs::remove_dir_all(&self.base_dir)?;
            fs::create_dir_all(&self.base_dir)?;
        }
        Ok(())
    }
}

impl Default for Snapshot {
    fn default() -> Self {
        Self::new("tests/__snapshots__")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[test]
    fn test_snapshot_creation() {
        let snapshot = Snapshot::new("tests/__snapshots__/test");
        let data = TestData {
            name: "test".to_string(),
            value: 42,
        };

        snapshot.update("test_data", &data).unwrap();
        snapshot.assert_matches("test_data", &data).unwrap();
    }
}
