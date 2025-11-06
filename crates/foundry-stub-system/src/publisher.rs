use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

use crate::error::{Result, StubError};
use crate::stub::{DefaultStubs, StubType};

/// Publisher for managing stub files
pub struct StubPublisher {
    target_dir: PathBuf,
}

impl StubPublisher {
    pub fn new(target_dir: impl AsRef<Path>) -> Self {
        Self {
            target_dir: target_dir.as_ref().to_path_buf(),
        }
    }

    /// Publish all default stubs to the target directory
    pub async fn publish_all(&self) -> Result<Vec<String>> {
        self.ensure_target_dir().await?;

        let stubs = vec![
            ("model", DefaultStubs::model()),
            ("controller", DefaultStubs::controller()),
            ("service", DefaultStubs::service()),
            ("migration", DefaultStubs::migration()),
            ("test", DefaultStubs::test()),
        ];

        let mut published = Vec::new();

        for (name, content) in stubs {
            let file_path = self.target_dir.join(format!("{}.stub", name));
            fs::write(&file_path, content).await?;
            info!("Published stub: {}", name);
            published.push(name.to_string());
        }

        Ok(published)
    }

    /// Publish a specific stub
    pub async fn publish(&self, stub_name: &str) -> Result<()> {
        self.ensure_target_dir().await?;

        let stub_type = StubType::from_str(stub_name)
            .ok_or_else(|| StubError::StubNotFound(stub_name.to_string()))?;

        let content = match stub_type {
            StubType::Model => DefaultStubs::model(),
            StubType::Controller => DefaultStubs::controller(),
            StubType::Service => DefaultStubs::service(),
            StubType::Migration => DefaultStubs::migration(),
            StubType::Test => DefaultStubs::test(),
            _ => return Err(StubError::PublishError(format!("No default stub for {}", stub_name))),
        };

        let file_path = self.target_dir.join(format!("{}.stub", stub_name));
        fs::write(&file_path, content).await?;

        info!("Published stub: {}", stub_name);
        Ok(())
    }

    /// Reset a stub to its default version
    pub async fn reset(&self, stub_name: &str) -> Result<()> {
        let file_path = self.target_dir.join(format!("{}.stub", stub_name));

        if !file_path.exists() {
            return Err(StubError::StubNotFound(format!(
                "Custom stub not found: {}",
                stub_name
            )));
        }

        fs::remove_file(&file_path).await?;
        info!("Reset stub: {}", stub_name);

        Ok(())
    }

    /// List all published custom stubs
    pub async fn list_published(&self) -> Result<Vec<String>> {
        if !self.target_dir.exists() {
            return Ok(Vec::new());
        }

        let mut stubs = Vec::new();
        let mut entries = fs::read_dir(&self.target_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("stub") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    stubs.push(name.to_string());
                }
            }
        }

        stubs.sort();
        Ok(stubs)
    }

    /// Check if a stub is published (custom)
    pub async fn is_published(&self, stub_name: &str) -> bool {
        let file_path = self.target_dir.join(format!("{}.stub", stub_name));
        file_path.exists()
    }

    /// Create a new custom stub
    pub async fn create_custom(
        &self,
        name: &str,
        content: impl AsRef<str>,
    ) -> Result<()> {
        self.ensure_target_dir().await?;

        let file_path = self.target_dir.join(format!("{}.stub", name));

        if file_path.exists() {
            return Err(StubError::PublishError(format!(
                "Stub already exists: {}",
                name
            )));
        }

        fs::write(&file_path, content.as_ref()).await?;
        info!("Created custom stub: {}", name);

        Ok(())
    }

    /// Ensure the target directory exists
    async fn ensure_target_dir(&self) -> Result<()> {
        if !self.target_dir.exists() {
            fs::create_dir_all(&self.target_dir).await?;
            debug!("Created stubs directory: {:?}", self.target_dir);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_publisher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());
        assert_eq!(publisher.target_dir, temp_dir.path());
    }

    #[tokio::test]
    async fn test_publish_all() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());

        let result = publisher.publish_all().await;
        assert!(result.is_ok());

        let published = result.unwrap();
        assert!(!published.is_empty());
        assert!(published.contains(&"model".to_string()));
    }

    #[tokio::test]
    async fn test_publish_single_stub() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());

        let result = publisher.publish("model").await;
        assert!(result.is_ok());

        let stub_path = temp_dir.path().join("model.stub");
        assert!(stub_path.exists());
    }

    #[tokio::test]
    async fn test_list_published() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());

        publisher.publish("model").await.unwrap();
        publisher.publish("controller").await.unwrap();

        let stubs = publisher.list_published().await.unwrap();
        assert_eq!(stubs.len(), 2);
        assert!(stubs.contains(&"model".to_string()));
        assert!(stubs.contains(&"controller".to_string()));
    }

    #[tokio::test]
    async fn test_reset_stub() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());

        publisher.publish("model").await.unwrap();
        assert!(publisher.is_published("model").await);

        publisher.reset("model").await.unwrap();
        assert!(!publisher.is_published("model").await);
    }

    #[tokio::test]
    async fn test_create_custom_stub() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());

        let content = "Custom stub: {{ name }}";
        let result = publisher.create_custom("my_custom", content).await;
        assert!(result.is_ok());

        let stub_path = temp_dir.path().join("my_custom.stub");
        assert!(stub_path.exists());

        let read_content = fs::read_to_string(stub_path).await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_is_published() {
        let temp_dir = TempDir::new().unwrap();
        let publisher = StubPublisher::new(temp_dir.path());

        assert!(!publisher.is_published("model").await);

        publisher.publish("model").await.unwrap();
        assert!(publisher.is_published("model").await);
    }
}
