//! Meilisearch Driver
//!
//! High-performance search engine driver using Meilisearch.

use crate::driver::{Result, SearchError};

#[cfg(feature = "meilisearch")]
use crate::driver::{ConfigurableDriver, SearchDriver};
#[cfg(feature = "meilisearch")]
use crate::searchable::{SearchHit, SearchOptions, SearchResult, Searchable};
#[cfg(feature = "meilisearch")]
use async_trait::async_trait;
#[cfg(feature = "meilisearch")]
use meilisearch_sdk::client::Client;
#[cfg(feature = "meilisearch")]
use meilisearch_sdk::indexes::Index;
#[cfg(feature = "meilisearch")]
use std::time::Instant;

/// Meilisearch driver for full-text search
#[cfg(feature = "meilisearch")]
pub struct MeilisearchDriver {
    client: Client,
}

#[cfg(feature = "meilisearch")]
impl MeilisearchDriver {
    /// Create a new Meilisearch driver
    ///
    /// # Arguments
    ///
    /// * `url` - Meilisearch server URL (e.g., "http://localhost:7700")
    /// * `api_key` - Optional API key for authentication
    ///
    /// # Example
    ///
    /// ```ignore
    /// let driver = MeilisearchDriver::new("http://localhost:7700", Some("masterKey"))?;
    /// ```
    pub fn new(url: &str, api_key: Option<&str>) -> Result<Self> {
        let client =
            Client::new(url, api_key).map_err(|e| SearchError::ConnectionError(e.to_string()))?;
        Ok(Self { client })
    }

    /// Get or create an index for a searchable type
    fn get_index(&self, index_name: &str) -> Index {
        self.client.index(index_name)
    }

    /// Create index with default settings
    pub async fn create_index<T: Searchable>(&self) -> Result<()> {
        let index_name = T::index_name();

        // Create task to create the index
        let _task = self
            .client
            .create_index(index_name, Some("id"))
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        // Wait for index creation
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    /// Delete an index
    pub async fn delete_index<T: Searchable>(&self) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let _task = index
            .delete()
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    /// Get index statistics
    pub async fn stats<T: Searchable>(&self) -> Result<usize> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let stats = index
            .get_stats()
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        Ok(stats.number_of_documents)
    }
}

#[cfg(feature = "meilisearch")]
#[async_trait]
impl SearchDriver for MeilisearchDriver {
    async fn index<T: Searchable>(&self, document: &T) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);
        let doc = document.to_searchable();

        let _task = index
            .add_documents(&[doc], Some("id"))
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    async fn index_many<T: Searchable>(&self, documents: Vec<&T>) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let docs: Vec<T::Model> = documents.into_iter().map(|d| d.to_searchable()).collect();

        let _task = index
            .add_documents(&docs, Some("id"))
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    async fn search<T: Searchable>(
        &self,
        query_text: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResult<T::Model>> {
        let start = Instant::now();
        let index_name = T::index_name();
        let index = self.get_index(index_name);
        let opts = options.unwrap_or_default();

        // Execute search directly with the SDK
        let results = index
            .search()
            .with_query(query_text)
            .with_limit(opts.limit)
            .with_offset(opts.offset)
            .execute::<T::Model>()
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        // Convert to our SearchResult format
        let hits: Vec<SearchHit<T::Model>> = results
            .hits
            .into_iter()
            .map(|hit| {
                SearchHit::new(hit.result, 1.0) // Meilisearch doesn't expose scores directly
            })
            .collect();

        let processing_time = start.elapsed().as_millis() as u64;

        Ok(SearchResult {
            hits,
            total: results.estimated_total_hits.unwrap_or(0),
            query: query_text.to_string(),
            processing_time_ms: processing_time,
            page: Some(opts.offset / opts.limit.max(1)),
            per_page: Some(opts.limit),
        })
    }

    async fn delete<T: Searchable>(&self, id: &str) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let _task = index
            .delete_document(id)
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        Ok(())
    }

    async fn update<T: Searchable>(&self, document: &T) -> Result<()> {
        // Meilisearch handles updates automatically when you add a document with the same ID
        self.index(document).await
    }

    async fn clear_index<T: Searchable>(&self) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let _task = index
            .delete_all_documents()
            .await
            .map_err(|e| SearchError::IndexError(e.to_string()))?;

        Ok(())
    }

    async fn count<T: Searchable>(&self) -> Result<usize> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let stats = index
            .get_stats()
            .await
            .map_err(|e| SearchError::SearchError(e.to_string()))?;

        Ok(stats.number_of_documents)
    }

    async fn health_check(&self) -> Result<()> {
        let _health = self
            .client
            .health()
            .await
            .map_err(|e| SearchError::ConnectionError(e.to_string()))?;

        Ok(())
    }
}

#[cfg(feature = "meilisearch")]
#[async_trait]
impl ConfigurableDriver for MeilisearchDriver {
    async fn configure_index<T: Searchable>(&self) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        // Set searchable attributes
        let searchable = T::searchable_fields()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        let _task = index
            .set_searchable_attributes(&searchable)
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        // Set filterable attributes
        let filterable = T::filterable_fields()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        if !filterable.is_empty() {
            let _task = index
                .set_filterable_attributes(&filterable)
                .await
                .map_err(|e| SearchError::ConfigError(e.to_string()))?;
        }

        // Set sortable attributes
        let sortable = T::sortable_fields()
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        if !sortable.is_empty() {
            let _task = index
                .set_sortable_attributes(&sortable)
                .await
                .map_err(|e| SearchError::ConfigError(e.to_string()))?;
        }

        Ok(())
    }

    async fn set_searchable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let _task = index
            .set_searchable_attributes(&fields)
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        Ok(())
    }

    async fn set_filterable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let _task = index
            .set_filterable_attributes(&fields)
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        Ok(())
    }

    async fn set_sortable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()> {
        let index_name = T::index_name();
        let index = self.get_index(index_name);

        let _task = index
            .set_sortable_attributes(&fields)
            .await
            .map_err(|e| SearchError::ConfigError(e.to_string()))?;

        Ok(())
    }
}

// Stub implementation when feature is not enabled
#[cfg(not(feature = "meilisearch"))]
pub struct MeilisearchDriver;

#[cfg(not(feature = "meilisearch"))]
impl MeilisearchDriver {
    /// Stub implementation when meilisearch feature is not enabled
    /// Returns an error instead of panicking
    pub fn new(_url: &str, _api_key: Option<&str>) -> Result<Self> {
        Err(SearchError::FeatureNotEnabled("meilisearch".to_string()))
    }
}

#[cfg(test)]
#[cfg(feature = "meilisearch")]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct TestDocument {
        id: String,
        title: String,
        content: String,
    }

    struct TestModel {
        id: String,
        title: String,
        content: String,
    }

    #[async_trait]
    impl Searchable for TestModel {
        type Model = TestDocument;

        fn searchable_fields() -> Vec<&'static str> {
            vec!["title", "content"]
        }

        fn to_searchable(&self) -> Self::Model {
            TestDocument {
                id: self.id.clone(),
                title: self.title.clone(),
                content: self.content.clone(),
            }
        }

        fn search_id(&self) -> String {
            self.id.clone()
        }

        fn index_name() -> &'static str {
            "test_documents"
        }

        fn filterable_fields() -> Vec<&'static str> {
            vec!["status"]
        }

        fn sortable_fields() -> Vec<&'static str> {
            vec!["created_at"]
        }
    }

    #[tokio::test]
    async fn test_meilisearch_driver_creation() {
        let result = MeilisearchDriver::new("http://localhost:7700", Some("masterKey"));
        // Just test that it constructs without panic
        assert!(result.is_ok(), "Driver creation should succeed");
    }
}
