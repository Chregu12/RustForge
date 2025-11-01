use anyhow::Context;
use foundry_plugins::{ArtifactPort, CommandError};
use std::fs;
use std::path::{Path, PathBuf};

pub struct LocalArtifactPort {
    root: PathBuf,
}

impl LocalArtifactPort {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn resolve(&self, path: &str) -> PathBuf {
        let candidate = Path::new(path);
        if candidate.is_absolute() {
            candidate.to_path_buf()
        } else {
            self.root.join(candidate)
        }
    }
}

impl Default for LocalArtifactPort {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
        }
    }
}

impl ArtifactPort for LocalArtifactPort {
    fn write_file(&self, path: &str, contents: &str, force: bool) -> Result<(), CommandError> {
        let absolute = self.resolve(path);

        if absolute.exists() && !force {
            return Err(CommandError::Message(format!(
                "Artefakt '{}' existiert bereits. Nutze --force zum Überschreiben.",
                absolute.display()
            )));
        }

        if let Some(parent) = absolute.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("konnte Verzeichnis {} nicht anlegen", parent.display())
            })?;
        }

        fs::write(&absolute, contents)
            .with_context(|| format!("konnte Datei {} nicht schreiben", absolute.display()))?;

        Ok(())
    }
}
