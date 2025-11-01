use crate::store::{CacheError, CacheStore};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// Tagged cache for group operations
pub struct TaggedCache {
    store: Arc<dyn CacheStore>,
    tags: Vec<String>,
}

impl TaggedCache {
    pub fn new(store: Arc<dyn CacheStore>, tags: Vec<String>) -> Self {
        Self { store, tags }
    }

    fn tag_key(&self, tag: &str) -> String {
        format!("tag:{}:keys", tag)
    }

    fn make_tagged_key(&self, key: &str) -> String {
        let tags_str = self.tags.join(":");
        format!("tagged:{}:{}", tags_str, key)
    }

    async fn add_key_to_tags(&self, key: &str) -> Result<(), CacheError> {
        for tag in &self.tags {
            let tag_key = self.tag_key(tag);

            // Get existing keys for this tag
            let mut keys: HashSet<String> = if let Some(value) = self.store.get(&tag_key).await? {
                value.to_json().unwrap_or_default()
            } else {
                HashSet::new()
            };

            keys.insert(key.to_string());

            // Save updated key set
            let value = crate::store::CacheValue::from_json(&keys)?;
            self.store.set(&tag_key, value, None).await?;
        }

        Ok(())
    }

    /// Set a tagged value
    pub async fn set<T>(&self, key: &str, value: &T, ttl: Option<Duration>) -> Result<(), CacheError>
    where
        T: Serialize,
    {
        let tagged_key = self.make_tagged_key(key);
        let cache_value = crate::store::CacheValue::from_json(value)?;

        self.store.set(&tagged_key, cache_value, ttl).await?;
        self.add_key_to_tags(key).await?;

        Ok(())
    }

    /// Get a tagged value
    pub async fn get<T>(&self, key: &str) -> Result<Option<T>, CacheError>
    where
        T: for<'de> Deserialize<'de>,
    {
        let tagged_key = self.make_tagged_key(key);

        if let Some(value) = self.store.get(&tagged_key).await? {
            Ok(Some(value.to_json()?))
        } else {
            Ok(None)
        }
    }

    /// Flush all keys with these tags
    pub async fn flush(&self) -> Result<(), CacheError> {
        for tag in &self.tags {
            let tag_key = self.tag_key(tag);

            // Get all keys for this tag
            if let Some(value) = self.store.get(&tag_key).await? {
                let keys: HashSet<String> = value.to_json()?;

                // Delete all tagged keys
                for key in keys {
                    let tagged_key = self.make_tagged_key(&key);
                    self.store.delete(&tagged_key).await?;
                }

                // Delete the tag key itself
                self.store.delete(&tag_key).await?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::MemoryStore;

    #[tokio::test]
    async fn test_tagged_cache() {
        let store = Arc::new(MemoryStore::new());
        let cache = TaggedCache::new(store.clone(), vec!["users".to_string()]);

        cache.set("user:1", &"John", None).await.unwrap();
        cache.set("user:2", &"Jane", None).await.unwrap();

        let name: Option<String> = cache.get("user:1").await.unwrap();
        assert_eq!(name, Some("John".to_string()));

        // Flush all users
        cache.flush().await.unwrap();

        let name: Option<String> = cache.get("user:1").await.unwrap();
        assert!(name.is_none());
    }

    #[tokio::test]
    async fn test_multi_tag_cache() {
        let store = Arc::new(MemoryStore::new());
        let cache = TaggedCache::new(
            store.clone(),
            vec!["users".to_string(), "active".to_string()],
        );

        cache.set("user:1", &"John", None).await.unwrap();

        let name: Option<String> = cache.get("user:1").await.unwrap();
        assert_eq!(name, Some("John".to_string()));

        cache.flush().await.unwrap();

        let name: Option<String> = cache.get("user:1").await.unwrap();
        assert!(name.is_none());
    }
}
