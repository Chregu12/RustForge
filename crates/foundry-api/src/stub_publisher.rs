/// Stub Publishing System
///
/// Provides functionality for publishing stubs to the application,
/// similar to Laravel's `vendor:publish --tag=stubs`.
///
/// This allows users to customize built-in stubs for their own use.

use crate::stubs::{Stub, StubManager};
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for stub publishing
#[derive(Clone, Debug)]
pub struct PublishConfig {
    /// Source directory for stubs
    pub source: PathBuf,
    /// Destination directory for published stubs
    pub destination: PathBuf,
    /// Whether to overwrite existing stubs
    pub force: bool,
    /// Whether to create destination if it doesn't exist
    pub create_dirs: bool,
}

impl PublishConfig {
    /// Create a new publish configuration
    pub fn new(source: impl AsRef<Path>, destination: impl AsRef<Path>) -> Self {
        Self {
            source: source.as_ref().to_path_buf(),
            destination: destination.as_ref().to_path_buf(),
            force: false,
            create_dirs: true,
        }
    }

    /// Set force overwrite flag
    pub fn force(mut self, force: bool) -> Self {
        self.force = force;
        self
    }

    /// Set create directories flag
    pub fn create_dirs(mut self, create: bool) -> Self {
        self.create_dirs = create;
        self
    }
}

/// Published stub information
#[derive(Clone, Debug)]
pub struct PublishedStub {
    pub id: String,
    pub source: PathBuf,
    pub destination: PathBuf,
    pub overwritten: bool,
    pub size: u64,
}

impl PublishedStub {
    /// Get relative path in destination
    pub fn relative_path(&self) -> String {
        self.destination
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| self.id.clone())
    }
}

/// Stub Publisher
pub struct StubPublisher {
    manager: StubManager,
    config: PublishConfig,
}

impl StubPublisher {
    /// Create a new stub publisher
    pub fn new(manager: StubManager, config: PublishConfig) -> Self {
        Self { manager, config }
    }

    /// Publish stubs
    pub fn publish(&self) -> Result<Vec<PublishedStub>, PublishError> {
        let mut published = Vec::new();

        // Create destination directory if needed
        if self.config.create_dirs && !self.config.destination.exists() {
            fs::create_dir_all(&self.config.destination).map_err(|e| {
                PublishError::DirectoryError(format!(
                    "Failed to create directory: {}",
                    e
                ))
            })?;
        }

        // Check if destination exists
        if !self.config.destination.exists() {
            return Err(PublishError::DestinationNotFound(
                self.config.destination.to_string_lossy().to_string(),
            ));
        }

        // Publish each stub
        for stub_id in self.manager.list() {
            match self.manager.get(&stub_id) {
                Ok(stub) => match self.publish_stub(&stub) {
                    Ok(pub_stub) => published.push(pub_stub),
                    Err(e) => {
                        return Err(PublishError::PublishFailed(
                            stub_id,
                            Box::new(e),
                        ))
                    }
                },
                Err(e) => {
                    return Err(PublishError::StubLoadFailed(
                        stub_id,
                        e.to_string(),
                    ))
                }
            }
        }

        Ok(published)
    }

    /// Publish a single stub
    fn publish_stub(&self, stub: &Stub) -> Result<PublishedStub, PublishError> {
        let filename = format!("{}.{}", stub.id, stub.extension);
        let destination = self.config.destination.join(&filename);

        // Check if file exists and we're not forcing
        if destination.exists() && !self.config.force {
            return Err(PublishError::FileExists(
                destination.to_string_lossy().to_string(),
            ));
        }

        // Write the stub content
        fs::write(&destination, &stub.content).map_err(|e| {
            PublishError::WriteError(format!("Failed to write stub: {}", e))
        })?;

        let size = stub.content.len() as u64;
        let overwritten = destination.exists();

        Ok(PublishedStub {
            id: stub.id.clone(),
            source: self.config.source.join(&filename),
            destination,
            overwritten,
            size,
        })
    }

    /// Publish stubs with specific tag/category
    pub fn publish_tagged(&self, tag: &str) -> Result<Vec<PublishedStub>, PublishError> {
        let mut published = Vec::new();

        // Create destination directory if needed
        if self.config.create_dirs && !self.config.destination.exists() {
            fs::create_dir_all(&self.config.destination).map_err(|e| {
                PublishError::DirectoryError(format!(
                    "Failed to create directory: {}",
                    e
                ))
            })?;
        }

        // Get stubs by category
        let stubs = self.manager.by_category(tag);

        if stubs.is_empty() {
            return Err(PublishError::NoStubsFound(tag.to_string()));
        }

        // Publish each stub
        for stub in stubs {
            match self.publish_stub(&stub) {
                Ok(pub_stub) => published.push(pub_stub),
                Err(e) => {
                    return Err(PublishError::PublishFailed(
                        stub.id,
                        Box::new(e),
                    ))
                }
            }
        }

        Ok(published)
    }

    /// Get list of published stubs without actually publishing
    pub fn preview(&self) -> Result<Vec<PublishedStub>, PublishError> {
        let mut stubs = Vec::new();

        for stub_id in self.manager.list() {
            match self.manager.get(&stub_id) {
                Ok(stub) => {
                    let filename = format!("{}.{}", stub.id, stub.extension);
                    let destination = self.config.destination.join(&filename);

                    stubs.push(PublishedStub {
                        id: stub.id.clone(),
                        source: self.config.source.join(&filename),
                        destination,
                        overwritten: false,
                        size: stub.content.len() as u64,
                    });
                }
                Err(e) => {
                    return Err(PublishError::StubLoadFailed(
                        stub_id,
                        e.to_string(),
                    ))
                }
            }
        }

        Ok(stubs)
    }
}

/// Publishing errors
#[derive(Debug)]
pub enum PublishError {
    /// Destination directory not found
    DestinationNotFound(String),
    /// Directory creation failed
    DirectoryError(String),
    /// File already exists
    FileExists(String),
    /// Failed to write stub
    WriteError(String),
    /// Stub loading failed
    StubLoadFailed(String, String),
    /// Publishing failed
    PublishFailed(String, Box<PublishError>),
    /// No stubs found for tag
    NoStubsFound(String),
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishError::DestinationNotFound(path) => {
                write!(f, "Destination directory not found: {}", path)
            }
            PublishError::DirectoryError(msg) => write!(f, "{}", msg),
            PublishError::FileExists(path) => write!(f, "File already exists: {}", path),
            PublishError::WriteError(msg) => write!(f, "{}", msg),
            PublishError::StubLoadFailed(id, msg) => {
                write!(f, "Failed to load stub '{}': {}", id, msg)
            }
            PublishError::PublishFailed(id, err) => {
                write!(f, "Failed to publish stub '{}': {}", id, err)
            }
            PublishError::NoStubsFound(tag) => {
                write!(f, "No stubs found for tag: {}", tag)
            }
        }
    }
}

impl std::error::Error for PublishError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::Stub;

    #[test]
    fn test_publish_config() {
        let config = PublishConfig::new("src", "dest").force(true);
        assert!(config.force);
        assert!(config.create_dirs);
    }

    #[test]
    fn test_published_stub() {
        let pub_stub = PublishedStub {
            id: "test".to_string(),
            source: PathBuf::from("src/test.rs"),
            destination: PathBuf::from("dest/test.rs"),
            overwritten: false,
            size: 100,
        };

        assert_eq!(pub_stub.id, "test");
        assert_eq!(pub_stub.size, 100);
    }
}
