//! Search driver trait and implementations

use crate::searchable::{SearchOptions, SearchResult, Searchable};
use async_trait::async_trait;
use thiserror::Error;

/// Errors that can occur during search operations
#[derive(Debug, Error)]
pub enum SearchError {
    /// Error occurred while indexing a document
    #[error("Index error: {0}")]
    IndexError(String),

    /// Error occurred while searching
    #[error("Search error: {0}")]
    SearchError(String),

    /// Error connecting to search backend
    #[error("Connection error: {0}")]
    ConnectionError(String),

    /// Document not found
    #[error("Document not found: {0}")]
    DocumentNotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Feature not enabled in Cargo.toml
    #[error("Feature '{0}' is not enabled. Enable it in Cargo.toml with: features = [\"{0}\"]")]
    FeatureNotEnabled(String),

    /// Query/API error (alias used by some drivers)
    #[error("Query error: {0}")]
    QueryError(String),

    /// Invalid query parameters
    #[error("Invalid query: {0}")]
    InvalidQuery(String),
}

/// Result type for search operations
pub type Result<T> = std::result::Result<T, SearchError>;

/// Trait for search driver implementations
#[async_trait]
pub trait SearchDriver: Send + Sync {
    /// Index a single document
    async fn index<T: Searchable>(&self, document: &T) -> Result<()>;

    /// Index multiple documents in a batch
    async fn index_many<T: Searchable>(&self, documents: Vec<&T>) -> Result<()>;

    /// Search for documents
    async fn search<T: Searchable>(
        &self,
        query: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResult<T::Model>>;

    /// Delete a document from the index
    async fn delete<T: Searchable>(&self, id: &str) -> Result<()>;

    /// Update a document in the index
    async fn update<T: Searchable>(&self, document: &T) -> Result<()> {
        // Default implementation: delete and re-index
        self.delete::<T>(&document.search_id()).await?;
        self.index(document).await
    }

    /// Clear the entire index
    async fn clear_index<T: Searchable>(&self) -> Result<()>;

    /// Get the number of documents in the index
    async fn count<T: Searchable>(&self) -> Result<usize>;

    /// Check if the search backend is healthy
    async fn health_check(&self) -> Result<()>;
}

/// Trait for drivers that support index configuration
#[async_trait]
pub trait ConfigurableDriver: SearchDriver {
    /// Create or update index with specified settings
    async fn configure_index<T: Searchable>(&self) -> Result<()>;

    /// Set searchable fields for an index
    async fn set_searchable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()>;

    /// Set filterable fields for an index
    async fn set_filterable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()>;

    /// Set sortable fields for an index
    async fn set_sortable_fields<T: Searchable>(&self, fields: Vec<String>) -> Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_error_display() {
        let err = SearchError::IndexError("test error".to_string());
        assert_eq!(err.to_string(), "Index error: test error");

        let err = SearchError::ConnectionError("connection failed".to_string());
        assert_eq!(err.to_string(), "Connection error: connection failed");
    }

    #[test]
    fn test_error_variants() {
        let errors = vec![
            SearchError::IndexError("idx".to_string()),
            SearchError::SearchError("search".to_string()),
            SearchError::ConnectionError("conn".to_string()),
            SearchError::DocumentNotFound("doc1".to_string()),
            SearchError::ConfigError("cfg".to_string()),
            SearchError::SerializationError("ser".to_string()),
        ];

        assert_eq!(errors.len(), 6);
    }
}
