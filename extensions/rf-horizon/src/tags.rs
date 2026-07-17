//! Job tagging system for filtering and organizing jobs

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Job tags container
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JobTags {
    pub tags: Vec<String>,
}

impl JobTags {
    /// Create new empty tags
    pub fn new() -> Self {
        Self { tags: Vec::new() }
    }

    /// Create tags from a list
    pub fn from_vec(tags: Vec<String>) -> Self {
        Self { tags }
    }

    /// Add a tag
    pub fn add(&mut self, tag: impl Into<String>) -> &mut Self {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
        self
    }

    /// Add multiple tags
    pub fn add_many(&mut self, tags: Vec<String>) -> &mut Self {
        for tag in tags {
            self.add(tag);
        }
        self
    }

    /// Check if contains a tag
    pub fn contains(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get all tags
    pub fn all(&self) -> &[String] {
        &self.tags
    }

    /// Create tags from a model ID
    pub fn from_model(model_name: &str, model_id: impl std::fmt::Display) -> Self {
        let mut tags = Self::new();
        tags.add(format!("{}:{}", model_name, model_id));
        tags
    }

    /// Auto-tag from user ID
    pub fn from_user(user_id: impl std::fmt::Display) -> Self {
        let mut tags = Self::new();
        tags.add(format!("user:{}", user_id));
        tags
    }

    /// Auto-tag from request ID
    pub fn from_request(request_id: impl std::fmt::Display) -> Self {
        let mut tags = Self::new();
        tags.add(format!("request:{}", request_id));
        tags
    }
}

impl Default for JobTags {
    fn default() -> Self {
        Self::new()
    }
}

impl From<Vec<String>> for JobTags {
    fn from(tags: Vec<String>) -> Self {
        Self::from_vec(tags)
    }
}

impl From<&[&str]> for JobTags {
    fn from(tags: &[&str]) -> Self {
        Self::from_vec(tags.iter().map(|s| s.to_string()).collect())
    }
}

/// Tagged job information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaggedJob {
    pub job_id: String,
    pub job_type: String,
    pub queue: String,
    pub tags: JobTags,
    pub payload: serde_json::Value,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl TaggedJob {
    /// Create a new tagged job
    pub fn new(
        job_id: impl Into<String>,
        job_type: impl Into<String>,
        queue: impl Into<String>,
        tags: JobTags,
    ) -> Self {
        Self {
            job_id: job_id.into(),
            job_type: job_type.into(),
            queue: queue.into(),
            tags,
            payload: serde_json::Value::Null,
            created_at: chrono::Utc::now(),
        }
    }

    /// Set payload
    pub fn with_payload(mut self, payload: serde_json::Value) -> Self {
        self.payload = payload;
        self
    }

    /// Check if job has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.contains(tag)
    }

    /// Check if job has any of the given tags
    pub fn has_any_tag(&self, tags: &[String]) -> bool {
        tags.iter().any(|tag| self.tags.contains(tag))
    }

    /// Check if job has all of the given tags
    pub fn has_all_tags(&self, tags: &[String]) -> bool {
        tags.iter().all(|tag| self.tags.contains(tag))
    }
}

/// Tag registry for tracking jobs by tags
#[derive(Clone)]
pub struct TagRegistry {
    /// Map of tag -> set of job IDs
    tag_to_jobs: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    /// Map of job ID -> tagged job
    jobs: Arc<RwLock<HashMap<String, TaggedJob>>>,
}

impl TagRegistry {
    /// Create a new tag registry
    pub fn new() -> Self {
        Self {
            tag_to_jobs: Arc::new(RwLock::new(HashMap::new())),
            jobs: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a tagged job
    pub async fn register(&self, job: TaggedJob) {
        let job_id = job.job_id.clone();
        let tags = job.tags.tags.clone();

        // Store job
        self.jobs.write().await.insert(job_id.clone(), job);

        // Index by tags
        let mut tag_map = self.tag_to_jobs.write().await;
        for tag in tags {
            tag_map
                .entry(tag)
                .or_insert_with(HashSet::new)
                .insert(job_id.clone());
        }
    }

    /// Remove a job from registry
    pub async fn remove(&self, job_id: &str) -> Option<TaggedJob> {
        let job = self.jobs.write().await.remove(job_id)?;

        // Remove from tag index
        let mut tag_map = self.tag_to_jobs.write().await;
        for tag in &job.tags.tags {
            if let Some(job_set) = tag_map.get_mut(tag) {
                job_set.remove(job_id);
                if job_set.is_empty() {
                    tag_map.remove(tag);
                }
            }
        }

        Some(job)
    }

    /// Get jobs by tag
    pub async fn jobs_by_tag(&self, tag: &str) -> Vec<TaggedJob> {
        let tag_map = self.tag_to_jobs.read().await;
        let jobs_store = self.jobs.read().await;

        tag_map
            .get(tag)
            .map(|job_ids| {
                job_ids
                    .iter()
                    .filter_map(|id| jobs_store.get(id).cloned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get jobs by multiple tags (any match)
    pub async fn jobs_by_any_tag(&self, tags: &[String]) -> Vec<TaggedJob> {
        let mut result = HashSet::new();
        let tag_map = self.tag_to_jobs.read().await;
        let jobs_store = self.jobs.read().await;

        for tag in tags {
            if let Some(job_ids) = tag_map.get(tag) {
                result.extend(job_ids.clone());
            }
        }

        result
            .iter()
            .filter_map(|id| jobs_store.get(id).cloned())
            .collect()
    }

    /// Get jobs by multiple tags (all match)
    pub async fn jobs_by_all_tags(&self, tags: &[String]) -> Vec<TaggedJob> {
        if tags.is_empty() {
            return Vec::new();
        }

        let jobs_store = self.jobs.read().await;
        jobs_store
            .values()
            .filter(|job| job.has_all_tags(tags))
            .cloned()
            .collect()
    }

    /// Get a specific job by ID
    pub async fn get(&self, job_id: &str) -> Option<TaggedJob> {
        self.jobs.read().await.get(job_id).cloned()
    }

    /// Get all tags
    pub async fn all_tags(&self) -> Vec<String> {
        self.tag_to_jobs
            .read()
            .await
            .keys()
            .cloned()
            .collect()
    }

    /// Get job count for a tag
    pub async fn count_by_tag(&self, tag: &str) -> usize {
        self.tag_to_jobs
            .read()
            .await
            .get(tag)
            .map(|set| set.len())
            .unwrap_or(0)
    }

    /// Clear all jobs
    pub async fn clear(&self) {
        self.tag_to_jobs.write().await.clear();
        self.jobs.write().await.clear();
    }

    /// Get total job count
    pub async fn count(&self) -> usize {
        self.jobs.read().await.len()
    }
}

impl Default for TagRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_job_tags_new() {
        let tags = JobTags::new();
        assert_eq!(tags.tags.len(), 0);
    }

    #[test]
    fn test_job_tags_add() {
        let mut tags = JobTags::new();
        tags.add("user:123").add("priority:high");

        assert_eq!(tags.tags.len(), 2);
        assert!(tags.contains("user:123"));
        assert!(tags.contains("priority:high"));
    }

    #[test]
    fn test_job_tags_no_duplicates() {
        let mut tags = JobTags::new();
        tags.add("user:123").add("user:123");

        assert_eq!(tags.tags.len(), 1);
    }

    #[test]
    fn test_job_tags_from_model() {
        let tags = JobTags::from_model("User", 42);
        assert_eq!(tags.tags.len(), 1);
        assert!(tags.contains("User:42"));
    }

    #[test]
    fn test_job_tags_from_user() {
        let tags = JobTags::from_user(123);
        assert_eq!(tags.tags.len(), 1);
        assert!(tags.contains("user:123"));
    }

    #[test]
    fn test_tagged_job_has_tag() {
        let mut tags = JobTags::new();
        tags.add("user:123").add("priority:high");

        let job = TaggedJob::new("job-1", "SendEmail", "emails", tags);
        assert!(job.has_tag("user:123"));
        assert!(job.has_tag("priority:high"));
        assert!(!job.has_tag("user:456"));
    }

    #[test]
    fn test_tagged_job_has_any_tag() {
        let mut tags = JobTags::new();
        tags.add("user:123");

        let job = TaggedJob::new("job-1", "SendEmail", "emails", tags);
        assert!(job.has_any_tag(&["user:123".to_string(), "user:456".to_string()]));
        assert!(!job.has_any_tag(&["user:456".to_string()]));
    }

    #[tokio::test]
    async fn test_tag_registry_register() {
        let registry = TagRegistry::new();

        let mut tags = JobTags::new();
        tags.add("user:123");

        let job = TaggedJob::new("job-1", "SendEmail", "emails", tags);
        registry.register(job).await;

        assert_eq!(registry.count().await, 1);
    }

    #[tokio::test]
    async fn test_tag_registry_jobs_by_tag() {
        let registry = TagRegistry::new();

        let mut tags1 = JobTags::new();
        tags1.add("user:123");
        let job1 = TaggedJob::new("job-1", "SendEmail", "emails", tags1);

        let mut tags2 = JobTags::new();
        tags2.add("user:456");
        let job2 = TaggedJob::new("job-2", "SendEmail", "emails", tags2);

        registry.register(job1).await;
        registry.register(job2).await;

        let jobs = registry.jobs_by_tag("user:123").await;
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].job_id, "job-1");
    }

    #[tokio::test]
    async fn test_tag_registry_jobs_by_any_tag() {
        let registry = TagRegistry::new();

        let mut tags1 = JobTags::new();
        tags1.add("user:123").add("priority:high");
        let job1 = TaggedJob::new("job-1", "SendEmail", "emails", tags1);

        let mut tags2 = JobTags::new();
        tags2.add("user:456");
        let job2 = TaggedJob::new("job-2", "SendEmail", "emails", tags2);

        registry.register(job1).await;
        registry.register(job2).await;

        let jobs = registry
            .jobs_by_any_tag(&["user:123".to_string(), "user:456".to_string()])
            .await;
        assert_eq!(jobs.len(), 2);
    }

    #[tokio::test]
    async fn test_tag_registry_remove() {
        let registry = TagRegistry::new();

        let mut tags = JobTags::new();
        tags.add("user:123");
        let job = TaggedJob::new("job-1", "SendEmail", "emails", tags);

        registry.register(job).await;
        assert_eq!(registry.count().await, 1);

        let removed = registry.remove("job-1").await;
        assert!(removed.is_some());
        assert_eq!(registry.count().await, 0);

        let jobs = registry.jobs_by_tag("user:123").await;
        assert_eq!(jobs.len(), 0);
    }
}
