//! Searchable trait and types for defining searchable models

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Trait for making models searchable
#[async_trait]
pub trait Searchable: Sized + Send + Sync {
    /// The type that represents the searchable document
    type Model: Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static;

    /// Get the fields that should be searchable
    fn searchable_fields() -> Vec<&'static str>;

    /// Convert this model into a searchable document
    fn to_searchable(&self) -> Self::Model;

    /// Get the unique identifier for this document in the search index
    fn search_id(&self) -> String;

    /// Name of the search index (e.g., "posts", "users")
    fn index_name() -> &'static str;

    /// Optional: Get filterable fields (for filtering search results)
    fn filterable_fields() -> Vec<&'static str> {
        Vec::new()
    }

    /// Optional: Get sortable fields
    fn sortable_fields() -> Vec<&'static str> {
        Vec::new()
    }
}

/// A search result containing hits and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult<T> {
    /// The matching documents
    pub hits: Vec<SearchHit<T>>,

    /// Total number of matching documents (before pagination)
    pub total: usize,

    /// The search query that was executed
    pub query: String,

    /// Time taken to process the search in milliseconds
    pub processing_time_ms: u64,

    /// Current page (if paginated)
    pub page: Option<usize>,

    /// Items per page (if paginated)
    pub per_page: Option<usize>,
}

impl<T> SearchResult<T> {
    /// Create a new search result
    pub fn new(query: String) -> Self {
        Self {
            hits: Vec::new(),
            total: 0,
            query,
            processing_time_ms: 0,
            page: None,
            per_page: None,
        }
    }

    /// Add a hit to the results
    pub fn add_hit(mut self, hit: SearchHit<T>) -> Self {
        self.hits.push(hit);
        self.total = self.hits.len();
        self
    }

    /// Set pagination info
    pub fn with_pagination(mut self, page: usize, per_page: usize) -> Self {
        self.page = Some(page);
        self.per_page = Some(per_page);
        self
    }

    /// Set processing time
    pub fn with_processing_time(mut self, ms: u64) -> Self {
        self.processing_time_ms = ms;
        self
    }

    /// Check if there are more results
    pub fn has_more_pages(&self) -> bool {
        if let (Some(page), Some(per_page)) = (self.page, self.per_page) {
            (page * per_page) < self.total
        } else {
            false
        }
    }
}

/// A single search hit (matching document)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit<T> {
    /// The document that matched
    pub document: T,

    /// Relevance score (higher is more relevant)
    pub score: f64,

    /// Highlighted snippets of matching text
    pub highlights: Option<HashMap<String, Vec<String>>>,

    /// Additional metadata about the match
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

impl<T> SearchHit<T> {
    /// Create a new search hit
    pub fn new(document: T, score: f64) -> Self {
        Self {
            document,
            score,
            highlights: None,
            metadata: None,
        }
    }

    /// Add highlights to the hit
    pub fn with_highlights(mut self, highlights: HashMap<String, Vec<String>>) -> Self {
        self.highlights = Some(highlights);
        self
    }

    /// Add metadata to the hit
    pub fn with_metadata(mut self, metadata: HashMap<String, serde_json::Value>) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Search options for filtering and pagination
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Filters to apply (field -> value)
    pub filters: HashMap<String, String>,

    /// Maximum number of results to return
    pub limit: usize,

    /// Offset for pagination
    pub offset: usize,

    /// Fields to highlight in results
    pub highlight_fields: Vec<String>,

    /// Sort by field (field, ascending)
    pub sort: Option<(String, bool)>,
}

impl SearchOptions {
    /// Create new search options with default limit
    pub fn new() -> Self {
        Self {
            filters: HashMap::new(),
            limit: 10,
            offset: 0,
            highlight_fields: Vec::new(),
            sort: None,
        }
    }

    /// Add a filter
    pub fn filter(mut self, field: impl Into<String>, value: impl Into<String>) -> Self {
        self.filters.insert(field.into(), value.into());
        self
    }

    /// Set result limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Set offset for pagination
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Add a field to highlight
    pub fn highlight(mut self, field: impl Into<String>) -> Self {
        self.highlight_fields.push(field.into());
        self
    }

    /// Set sort field and direction
    pub fn sort_by(mut self, field: impl Into<String>, ascending: bool) -> Self {
        self.sort = Some((field.into(), ascending));
        self
    }

    /// Paginate by page number
    pub fn page(mut self, page: usize, per_page: usize) -> Self {
        self.offset = page * per_page;
        self.limit = per_page;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Serialize, Deserialize, Clone)]
    struct TestDoc {
        id: String,
        title: String,
    }

    struct TestModel {
        id: String,
        title: String,
    }

    #[async_trait]
    impl Searchable for TestModel {
        type Model = TestDoc;

        fn searchable_fields() -> Vec<&'static str> {
            vec!["title"]
        }

        fn to_searchable(&self) -> Self::Model {
            TestDoc {
                id: self.id.clone(),
                title: self.title.clone(),
            }
        }

        fn search_id(&self) -> String {
            self.id.clone()
        }

        fn index_name() -> &'static str {
            "test_models"
        }
    }

    #[test]
    fn test_searchable_impl() {
        let model = TestModel {
            id: "1".to_string(),
            title: "Test".to_string(),
        };

        assert_eq!(model.search_id(), "1");
        assert_eq!(TestModel::index_name(), "test_models");
        assert_eq!(TestModel::searchable_fields(), vec!["title"]);
    }

    #[test]
    fn test_search_options_builder() {
        let options = SearchOptions::new()
            .filter("status", "published")
            .limit(20)
            .offset(10)
            .highlight("title")
            .sort_by("created_at", false);

        assert_eq!(options.limit, 20);
        assert_eq!(options.offset, 10);
        assert_eq!(
            options.filters.get("status"),
            Some(&"published".to_string())
        );
        assert_eq!(options.highlight_fields.len(), 1);
        assert!(options.sort.is_some());
    }

    #[test]
    fn test_search_result_builder() {
        let doc = TestDoc {
            id: "1".to_string(),
            title: "Test".to_string(),
        };

        let hit = SearchHit::new(doc, 0.95);
        let result = SearchResult::new("test query".to_string())
            .add_hit(hit)
            .with_pagination(1, 10)
            .with_processing_time(25);

        assert_eq!(result.hits.len(), 1);
        assert_eq!(result.total, 1);
        assert_eq!(result.processing_time_ms, 25);
        assert_eq!(result.page, Some(1));
    }

    #[test]
    fn test_search_options_pagination() {
        let options = SearchOptions::new().page(2, 20);

        assert_eq!(options.offset, 40);
        assert_eq!(options.limit, 20);
    }
}
