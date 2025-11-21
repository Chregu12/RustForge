//! Content revision management and versioning

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::{CmsError, CmsResult};

/// Content revision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    /// Revision ID
    pub id: String,

    /// Content ID this revision belongs to
    pub content_id: String,

    /// Revision number
    pub version: u32,

    /// Content data
    pub data: serde_json::Value,

    /// User who created this revision
    pub author: String,

    /// Comment/description
    pub comment: Option<String>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Size in bytes
    pub size: usize,
}

/// Diff between two revisions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionDiff {
    /// Old revision version
    pub from_version: u32,

    /// New revision version
    pub to_version: u32,

    /// Added fields/content
    pub added: Vec<String>,

    /// Removed fields/content
    pub removed: Vec<String>,

    /// Modified fields/content
    pub modified: Vec<String>,
}

/// Revision manager
pub struct RevisionManager {
    /// In-memory storage (in production, use database)
    revisions: Arc<RwLock<HashMap<String, Vec<Revision>>>>,

    /// Maximum revisions per content
    max_revisions: usize,
}

impl RevisionManager {
    /// Create a new revision manager
    pub fn new() -> Self {
        Self {
            revisions: Arc::new(RwLock::new(HashMap::new())),
            max_revisions: 50,
        }
    }

    /// Set maximum revisions to keep
    pub fn max_revisions(mut self, max: usize) -> Self {
        self.max_revisions = max;
        self
    }

    /// Create a new revision
    pub async fn create_revision(
        &self,
        content_id: &str,
        data: serde_json::Value,
        author: &str,
        comment: Option<String>,
    ) -> CmsResult<Revision> {
        let mut revisions = self.revisions.write().await;

        // Get existing revisions for this content
        let content_revisions = revisions.entry(content_id.to_string()).or_insert_with(Vec::new);

        // Calculate next version (based on highest existing version, not length)
        let version = content_revisions
            .last()
            .map(|r| r.version + 1)
            .unwrap_or(1);

        // Create revision
        let revision = Revision {
            id: format!("{}_{}", content_id, version),
            content_id: content_id.to_string(),
            version,
            data: data.clone(),
            author: author.to_string(),
            comment,
            created_at: Utc::now(),
            size: serde_json::to_string(&data)
                .map(|s| s.len())
                .unwrap_or(0),
        };

        // Add revision
        content_revisions.push(revision.clone());

        // Enforce max revisions limit
        while content_revisions.len() > self.max_revisions {
            content_revisions.remove(0);
        }

        Ok(revision)
    }

    /// Get all revisions for content
    pub async fn get_revisions(&self, content_id: &str) -> CmsResult<Vec<Revision>> {
        let revisions = self.revisions.read().await;

        Ok(revisions
            .get(content_id)
            .cloned()
            .unwrap_or_default())
    }

    /// Get specific revision
    pub async fn get_revision(
        &self,
        content_id: &str,
        version: u32,
    ) -> CmsResult<Revision> {
        let revisions = self.revisions.read().await;

        revisions
            .get(content_id)
            .and_then(|revs| revs.iter().find(|r| r.version == version))
            .cloned()
            .ok_or_else(|| {
                CmsError::RevisionError(format!(
                    "Revision not found: {} v{}",
                    content_id, version
                ))
            })
    }

    /// Get latest revision
    pub async fn get_latest(&self, content_id: &str) -> CmsResult<Revision> {
        let revisions = self.revisions.read().await;

        revisions
            .get(content_id)
            .and_then(|revs| revs.last())
            .cloned()
            .ok_or_else(|| {
                CmsError::RevisionError(format!("No revisions found for: {}", content_id))
            })
    }

    /// Rollback to specific revision
    pub async fn rollback(
        &self,
        content_id: &str,
        to_version: u32,
        author: &str,
    ) -> CmsResult<Revision> {
        // Get the target revision
        let target = self.get_revision(content_id, to_version).await?;

        // Create a new revision with the old data
        self.create_revision(
            content_id,
            target.data.clone(),
            author,
            Some(format!("Rollback to version {}", to_version)),
        )
        .await
    }

    /// Compare two revisions
    pub async fn diff(
        &self,
        content_id: &str,
        from_version: u32,
        to_version: u32,
    ) -> CmsResult<RevisionDiff> {
        let from_rev = self.get_revision(content_id, from_version).await?;
        let to_rev = self.get_revision(content_id, to_version).await?;

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let mut modified = Vec::new();

        // Compare as objects
        if let (Some(from_obj), Some(to_obj)) = (from_rev.data.as_object(), to_rev.data.as_object()) {
            // Check for added/modified keys
            for (key, to_value) in to_obj {
                if let Some(from_value) = from_obj.get(key) {
                    if from_value != to_value {
                        modified.push(key.clone());
                    }
                } else {
                    added.push(key.clone());
                }
            }

            // Check for removed keys
            for key in from_obj.keys() {
                if !to_obj.contains_key(key) {
                    removed.push(key.clone());
                }
            }
        }

        Ok(RevisionDiff {
            from_version,
            to_version,
            added,
            removed,
            modified,
        })
    }

    /// Delete all revisions for content
    pub async fn delete_all(&self, content_id: &str) -> CmsResult<()> {
        let mut revisions = self.revisions.write().await;
        revisions.remove(content_id);
        Ok(())
    }

    /// Get revision count
    pub async fn count(&self, content_id: &str) -> usize {
        let revisions = self.revisions.read().await;
        revisions.get(content_id).map(|r| r.len()).unwrap_or(0)
    }
}

impl Default for RevisionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_create_revision() {
        let manager = RevisionManager::new();

        let data = json!({"title": "Test", "content": "Hello"});
        let rev = manager
            .create_revision("post_1", data, "user1", None)
            .await
            .unwrap();

        assert_eq!(rev.content_id, "post_1");
        assert_eq!(rev.version, 1);
        assert_eq!(rev.author, "user1");
    }

    #[tokio::test]
    async fn test_multiple_revisions() {
        let manager = RevisionManager::new();

        // Create 3 revisions
        for i in 1..=3 {
            let data = json!({"version": i});
            manager
                .create_revision("post_1", data, "user1", None)
                .await
                .unwrap();
        }

        let revisions = manager.get_revisions("post_1").await.unwrap();
        assert_eq!(revisions.len(), 3);
        assert_eq!(revisions[2].version, 3);
    }

    #[tokio::test]
    async fn test_get_specific_revision() {
        let manager = RevisionManager::new();

        let data1 = json!({"version": 1});
        let data2 = json!({"version": 2});

        manager
            .create_revision("post_1", data1, "user1", None)
            .await
            .unwrap();

        manager
            .create_revision("post_1", data2, "user1", None)
            .await
            .unwrap();

        let rev = manager.get_revision("post_1", 1).await.unwrap();
        assert_eq!(rev.version, 1);
        assert_eq!(rev.data["version"], 1);
    }

    #[tokio::test]
    async fn test_get_latest() {
        let manager = RevisionManager::new();

        for i in 1..=5 {
            let data = json!({"version": i});
            manager
                .create_revision("post_1", data, "user1", None)
                .await
                .unwrap();
        }

        let latest = manager.get_latest("post_1").await.unwrap();
        assert_eq!(latest.version, 5);
        assert_eq!(latest.data["version"], 5);
    }

    #[tokio::test]
    async fn test_rollback() {
        let manager = RevisionManager::new();

        let data1 = json!({"content": "version 1"});
        let data2 = json!({"content": "version 2"});
        let data3 = json!({"content": "version 3"});

        manager.create_revision("post_1", data1, "user1", None).await.unwrap();
        manager.create_revision("post_1", data2, "user1", None).await.unwrap();
        manager.create_revision("post_1", data3, "user1", None).await.unwrap();

        // Rollback to version 1
        let rolled_back = manager.rollback("post_1", 1, "user1").await.unwrap();

        assert_eq!(rolled_back.version, 4); // New version
        assert_eq!(rolled_back.data["content"], "version 1"); // Old data
        assert!(rolled_back.comment.unwrap().contains("Rollback"));
    }

    #[tokio::test]
    async fn test_diff() {
        let manager = RevisionManager::new();

        let data1 = json!({"title": "Old", "content": "Test", "removed": "value"});
        let data2 = json!({"title": "New", "content": "Test", "added": "value"});

        manager.create_revision("post_1", data1, "user1", None).await.unwrap();
        manager.create_revision("post_1", data2, "user1", None).await.unwrap();

        let diff = manager.diff("post_1", 1, 2).await.unwrap();

        assert_eq!(diff.from_version, 1);
        assert_eq!(diff.to_version, 2);
        assert!(diff.added.contains(&"added".to_string()));
        assert!(diff.removed.contains(&"removed".to_string()));
        assert!(diff.modified.contains(&"title".to_string()));
    }

    #[tokio::test]
    async fn test_max_revisions_limit() {
        let manager = RevisionManager::new().max_revisions(3);

        // Create 5 revisions (should only keep last 3)
        for i in 1..=5 {
            let data = json!({"version": i});
            manager
                .create_revision("post_1", data, "user1", None)
                .await
                .unwrap();
        }

        let revisions = manager.get_revisions("post_1").await.unwrap();
        assert_eq!(revisions.len(), 3);
        assert_eq!(revisions[0].version, 3); // Oldest kept is version 3
        assert_eq!(revisions[2].version, 5); // Latest is version 5
    }

    #[tokio::test]
    async fn test_delete_all() {
        let manager = RevisionManager::new();

        for i in 1..=3 {
            let data = json!({"version": i});
            manager
                .create_revision("post_1", data, "user1", None)
                .await
                .unwrap();
        }

        manager.delete_all("post_1").await.unwrap();

        let count = manager.count("post_1").await;
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_revision_count() {
        let manager = RevisionManager::new();

        assert_eq!(manager.count("post_1").await, 0);

        for i in 1..=3 {
            let data = json!({"version": i});
            manager
                .create_revision("post_1", data, "user1", None)
                .await
                .unwrap();
        }

        assert_eq!(manager.count("post_1").await, 3);
    }
}
