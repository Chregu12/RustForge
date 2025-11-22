//! Storage for Telescope entries

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Type of telescope entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EntryType {
    Request,
    Query,
    Exception,
    Cache,
    Job,
    Mail,
}

/// Base telescope entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub entry_type: EntryType,
    pub content: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl Entry {
    /// Create a new entry
    pub fn new(entry_type: EntryType, content: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            entry_type,
            content,
            created_at: Utc::now(),
            tags: Vec::new(),
        }
    }

    /// Add a tag to the entry
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }
}

/// Storage for telescope entries
#[derive(Clone)]
pub struct Storage {
    entries: Arc<RwLock<HashMap<String, Entry>>>,
    entries_by_type: Arc<RwLock<HashMap<EntryType, Vec<String>>>>,
}

impl Storage {
    /// Create a new storage instance
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            entries_by_type: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store an entry
    pub async fn store(&self, entry: Entry) {
        let entry_id = entry.id.clone();
        let entry_type = entry.entry_type.clone();

        // Store the entry
        {
            let mut entries = self.entries.write().await;
            entries.insert(entry_id.clone(), entry);
        }

        // Index by type
        {
            let mut by_type = self.entries_by_type.write().await;
            by_type
                .entry(entry_type)
                .or_insert_with(Vec::new)
                .push(entry_id);
        }
    }

    /// Get an entry by ID
    pub async fn get(&self, id: &str) -> Option<Entry> {
        let entries = self.entries.read().await;
        entries.get(id).cloned()
    }

    /// Get all entries
    pub async fn all(&self) -> Vec<Entry> {
        let entries = self.entries.read().await;
        let mut result: Vec<Entry> = entries.values().cloned().collect();
        result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        result
    }

    /// Get entries by type
    pub async fn by_type(&self, entry_type: EntryType) -> Vec<Entry> {
        let by_type = self.entries_by_type.read().await;
        let entries = self.entries.read().await;

        if let Some(ids) = by_type.get(&entry_type) {
            let mut result: Vec<Entry> = ids
                .iter()
                .filter_map(|id| entries.get(id).cloned())
                .collect();
            result.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            result
        } else {
            Vec::new()
        }
    }

    /// Get entries with pagination
    pub async fn paginate(
        &self,
        entry_type: Option<EntryType>,
        page: usize,
        per_page: usize,
    ) -> (Vec<Entry>, usize) {
        let all_entries = if let Some(et) = entry_type {
            self.by_type(et).await
        } else {
            self.all().await
        };

        let total = all_entries.len();
        let start = page * per_page;
        let end = (start + per_page).min(total);

        let entries = if start < total {
            all_entries[start..end].to_vec()
        } else {
            Vec::new()
        };

        (entries, total)
    }

    /// Prune old entries
    pub async fn prune(&self, older_than_hours: i64) {
        let cutoff = Utc::now() - Duration::hours(older_than_hours);
        let mut entries = self.entries.write().await;
        let mut by_type = self.entries_by_type.write().await;

        // Find entries to remove
        let to_remove: Vec<String> = entries
            .iter()
            .filter(|(_, entry)| entry.created_at < cutoff)
            .map(|(id, _)| id.clone())
            .collect();

        // Remove from main storage
        for id in &to_remove {
            if let Some(entry) = entries.remove(id) {
                // Remove from type index
                if let Some(ids) = by_type.get_mut(&entry.entry_type) {
                    ids.retain(|i| i != id);
                }
            }
        }
    }

    /// Clear all entries
    pub async fn clear(&self) {
        let mut entries = self.entries.write().await;
        let mut by_type = self.entries_by_type.write().await;
        entries.clear();
        by_type.clear();
    }

    /// Get count of entries by type
    pub async fn count(&self, entry_type: Option<EntryType>) -> usize {
        if let Some(et) = entry_type {
            let by_type = self.entries_by_type.read().await;
            by_type.get(&et).map(|ids| ids.len()).unwrap_or(0)
        } else {
            let entries = self.entries.read().await;
            entries.len()
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_storage_store_and_get() {
        let storage = Storage::new();
        let entry = Entry::new(EntryType::Request, json!({"method": "GET", "path": "/"}));
        let id = entry.id.clone();

        storage.store(entry).await;

        let retrieved = storage.get(&id).await;
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().content["method"], "GET");
    }

    #[tokio::test]
    async fn test_storage_all() {
        let storage = Storage::new();

        storage
            .store(Entry::new(EntryType::Request, json!({})))
            .await;
        storage.store(Entry::new(EntryType::Query, json!({}))).await;
        storage
            .store(Entry::new(EntryType::Exception, json!({})))
            .await;

        let all = storage.all().await;
        assert_eq!(all.len(), 3);
    }

    #[tokio::test]
    async fn test_storage_by_type() {
        let storage = Storage::new();

        storage
            .store(Entry::new(EntryType::Request, json!({})))
            .await;
        storage
            .store(Entry::new(EntryType::Request, json!({})))
            .await;
        storage.store(Entry::new(EntryType::Query, json!({}))).await;

        let requests = storage.by_type(EntryType::Request).await;
        assert_eq!(requests.len(), 2);

        let queries = storage.by_type(EntryType::Query).await;
        assert_eq!(queries.len(), 1);
    }

    #[tokio::test]
    async fn test_storage_count() {
        let storage = Storage::new();

        storage
            .store(Entry::new(EntryType::Request, json!({})))
            .await;
        storage
            .store(Entry::new(EntryType::Request, json!({})))
            .await;
        storage.store(Entry::new(EntryType::Query, json!({}))).await;

        assert_eq!(storage.count(None).await, 3);
        assert_eq!(storage.count(Some(EntryType::Request)).await, 2);
        assert_eq!(storage.count(Some(EntryType::Query)).await, 1);
        assert_eq!(storage.count(Some(EntryType::Exception)).await, 0);
    }

    #[tokio::test]
    async fn test_storage_pagination() {
        let storage = Storage::new();

        // Create 25 entries
        for _ in 0..25 {
            storage
                .store(Entry::new(EntryType::Request, json!({})))
                .await;
        }

        let (page1, total) = storage.paginate(None, 0, 10).await;
        assert_eq!(page1.len(), 10);
        assert_eq!(total, 25);

        let (page2, _) = storage.paginate(None, 1, 10).await;
        assert_eq!(page2.len(), 10);

        let (page3, _) = storage.paginate(None, 2, 10).await;
        assert_eq!(page3.len(), 5);
    }

    #[tokio::test]
    async fn test_storage_pagination_by_type() {
        let storage = Storage::new();

        for _ in 0..15 {
            storage
                .store(Entry::new(EntryType::Request, json!({})))
                .await;
        }
        for _ in 0..10 {
            storage.store(Entry::new(EntryType::Query, json!({}))).await;
        }

        let (requests, total) = storage.paginate(Some(EntryType::Request), 0, 10).await;
        assert_eq!(requests.len(), 10);
        assert_eq!(total, 15);

        let (queries, total) = storage.paginate(Some(EntryType::Query), 0, 10).await;
        assert_eq!(queries.len(), 10);
        assert_eq!(total, 10);
    }

    #[tokio::test]
    async fn test_storage_prune() {
        let storage = Storage::new();

        // Create entries with different timestamps
        let old_entry = Entry {
            id: Uuid::new_v4().to_string(),
            entry_type: EntryType::Request,
            content: json!({}),
            created_at: Utc::now() - Duration::hours(48),
            tags: vec![],
        };

        let new_entry = Entry::new(EntryType::Request, json!({}));

        storage.store(old_entry).await;
        storage.store(new_entry).await;

        assert_eq!(storage.count(None).await, 2);

        // Prune entries older than 24 hours
        storage.prune(24).await;

        assert_eq!(storage.count(None).await, 1);
    }

    #[tokio::test]
    async fn test_storage_clear() {
        let storage = Storage::new();

        storage
            .store(Entry::new(EntryType::Request, json!({})))
            .await;
        storage.store(Entry::new(EntryType::Query, json!({}))).await;
        storage
            .store(Entry::new(EntryType::Exception, json!({})))
            .await;

        assert_eq!(storage.count(None).await, 3);

        storage.clear().await;

        assert_eq!(storage.count(None).await, 0);
    }

    #[tokio::test]
    async fn test_entry_with_tags() {
        let storage = Storage::new();

        let entry = Entry::new(EntryType::Request, json!({}))
            .with_tag("slow")
            .with_tag("api");

        assert_eq!(entry.tags.len(), 2);
        assert!(entry.tags.contains(&"slow".to_string()));
        assert!(entry.tags.contains(&"api".to_string()));

        storage.store(entry).await;
    }

    #[tokio::test]
    async fn test_entry_with_multiple_tags() {
        let entry = Entry::new(EntryType::Request, json!({})).with_tags(vec![
            "tag1".to_string(),
            "tag2".to_string(),
            "tag3".to_string(),
        ]);

        assert_eq!(entry.tags.len(), 3);
    }

    #[tokio::test]
    async fn test_storage_get_nonexistent() {
        let storage = Storage::new();
        let result = storage.get("nonexistent-id").await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_storage_sorted_by_created_at() {
        let storage = Storage::new();

        // Add entries with slight delays to ensure different timestamps
        storage
            .store(Entry::new(EntryType::Request, json!({"order": 1})))
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        storage
            .store(Entry::new(EntryType::Request, json!({"order": 2})))
            .await;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        storage
            .store(Entry::new(EntryType::Request, json!({"order": 3})))
            .await;

        let all = storage.all().await;
        // Should be sorted newest first
        assert_eq!(all[0].content["order"], 3);
        assert_eq!(all[1].content["order"], 2);
        assert_eq!(all[2].content["order"], 1);
    }
}
