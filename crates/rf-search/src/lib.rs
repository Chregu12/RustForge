//! # rf-search: Full-Text Search for RustForge
//!
//! Production-ready full-text search with multiple backend drivers.
//!
//! ## Features
//!
//! - **Multiple Drivers**: In-memory, PostgreSQL FTS, Meilisearch
//! - **Searchable Trait**: Simple trait for making models searchable
//! - **Flexible Queries**: Filters, sorting, pagination, highlighting
//! - **Type-Safe**: Full Rust type safety with generics
//! - **Async**: Built on async/await
//!
//! ## Quick Start
//!
//! ```ignore
//! use rf_search::{Searchable, SearchDriver, drivers::MeilisearchDriver};
//! use serde::{Serialize, Deserialize};
//! use async_trait::async_trait;
//!
//! #[derive(Serialize, Deserialize)]
//! struct Article {
//!     id: String,
//!     title: String,
//!     content: String,
//! }
//!
//! #[async_trait]
//! impl Searchable for Article {
//!     type Model = Article;
//!
//!     fn searchable_fields() -> Vec<&'static str> {
//!         vec!["title", "content"]
//!     }
//!
//!     fn to_searchable(&self) -> Self::Model {
//!         self.clone()
//!     }
//!
//!     fn search_id(&self) -> String {
//!         self.id.clone()
//!     }
//!
//!     fn index_name() -> &'static str {
//!         "articles"
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create driver
//! let driver = MeilisearchDriver::new("http://localhost:7700", Some("key"));
//!
//! // Index a document
//! let article = Article {
//!     id: "1".to_string(),
//!     title: "Rust Web Framework".to_string(),
//!     content: "Building web apps with Rust".to_string(),
//! };
//! driver.index(&article).await?;
//!
//! // Search
//! let results = driver.search::<Article>("rust", None).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Using PostgreSQL Full-Text Search
//!
//! ```ignore
//! use rf_search::drivers::PostgresSearchDriver;
//! use sqlx::PgPool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let pool = PgPool::connect("postgresql://localhost/mydb").await?;
//! let driver = PostgresSearchDriver::new(pool);
//!
//! // Create FTS index
//! driver.create_fts_index("articles", vec!["title", "content"]).await?;
//!
//! // Search (indexing happens automatically on INSERT/UPDATE)
//! let results = driver.search::<Article>("rust framework", None).await?;
//! # Ok(())
//! # }
//! ```

// Re-export the original in-memory search engine
pub use in_memory::*;

// Export new driver-based search
pub mod driver;
pub mod drivers;
pub mod searchable;

#[cfg(feature = "metrics")]
pub mod metrics;

pub use driver::{ConfigurableDriver, Result, SearchDriver, SearchError};
pub use searchable::{SearchHit, SearchOptions, SearchResult, Searchable};

#[cfg(feature = "metrics")]
pub use metrics::MetricsWrapper;

// In-memory search engine (original implementation)
mod in_memory {
    use async_trait::async_trait;
    use rust_stemmers::{Algorithm, Stemmer};
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet};
    use thiserror::Error;
    use unicode_segmentation::UnicodeSegmentation;

    /// Search errors
    #[derive(Debug, Error)]
    pub enum SearchError {
        #[error("Document not found: {0}")]
        DocumentNotFound(String),

        #[error("Index error: {0}")]
        IndexError(String),

        #[error("Query error: {0}")]
        QueryError(String),
    }

    pub type SearchResult<T> = Result<T, SearchError>;

    /// Searchable document (legacy interface)
    pub trait SearchableDoc {
        /// Get document ID
        fn id(&self) -> &str;

        /// Get searchable fields
        fn searchable_fields(&self) -> Vec<String>;
    }

    /// Document to be indexed
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Document {
        pub id: String,
        pub fields: HashMap<String, String>,
        pub metadata: HashMap<String, serde_json::Value>,
    }

    impl Document {
        /// Create a new document
        pub fn new(id: impl Into<String>) -> Self {
            Self {
                id: id.into(),
                fields: HashMap::new(),
                metadata: HashMap::new(),
            }
        }

        /// Add a field
        pub fn field(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
            self.fields.insert(name.into(), value.into());
            self
        }

        /// Add metadata
        pub fn meta<T: Serialize>(
            mut self,
            key: impl Into<String>,
            value: T,
        ) -> Result<Self, serde_json::Error> {
            self.metadata
                .insert(key.into(), serde_json::to_value(value)?);
            Ok(self)
        }
    }

    /// Search result
    #[derive(Debug, Clone)]
    pub struct SearchHit {
        pub id: String,
        pub score: f32,
        pub fields: HashMap<String, String>,
        pub metadata: HashMap<String, serde_json::Value>,
    }

    /// Search query
    pub struct Query {
        text: String,
        fuzzy: Option<f32>,
        limit: usize,
        offset: usize,
    }

    impl Query {
        /// Create a new query
        pub fn new(text: impl Into<String>) -> Self {
            Self {
                text: text.into(),
                fuzzy: None,
                limit: 10,
                offset: 0,
            }
        }

        /// Enable fuzzy matching
        pub fn fuzzy(mut self, threshold: f32) -> Self {
            self.fuzzy = Some(threshold);
            self
        }

        /// Set result limit
        pub fn limit(mut self, limit: usize) -> Self {
            self.limit = limit;
            self
        }

        /// Set result offset
        pub fn offset(mut self, offset: usize) -> Self {
            self.offset = offset;
            self
        }
    }

    /// Tokenizer for splitting text into terms
    struct Tokenizer {
        stemmer: Stemmer,
    }

    impl Tokenizer {
        fn new() -> Self {
            Self {
                stemmer: Stemmer::create(Algorithm::English),
            }
        }

        fn tokenize(&self, text: &str) -> Vec<String> {
            text.unicode_words()
                .map(|word| word.to_lowercase())
                .map(|word| self.stemmer.stem(&word).to_string())
                .collect()
        }
    }

    /// Inverted index for fast searching
    #[derive(Default)]
    struct InvertedIndex {
        index: HashMap<String, HashSet<String>>,
    }

    impl InvertedIndex {
        fn new() -> Self {
            Self {
                index: HashMap::new(),
            }
        }

        fn add_term(&mut self, term: &str, doc_id: &str) {
            self.index
                .entry(term.to_string())
                .or_insert_with(HashSet::new)
                .insert(doc_id.to_string());
        }

        fn get_documents(&self, term: &str) -> Option<&HashSet<String>> {
            self.index.get(term)
        }

        fn remove_document(&mut self, doc_id: &str) {
            for docs in self.index.values_mut() {
                docs.remove(doc_id);
            }
        }
    }

    /// In-memory search engine
    pub struct SearchEngine {
        documents: HashMap<String, Document>,
        index: InvertedIndex,
        tokenizer: Tokenizer,
    }

    impl SearchEngine {
        /// Create a new search engine
        pub fn new() -> Self {
            Self {
                documents: HashMap::new(),
                index: InvertedIndex::new(),
                tokenizer: Tokenizer::new(),
            }
        }

        /// Index a document
        pub fn index(&mut self, document: Document) -> SearchResult<()> {
            let doc_id = document.id.clone();

            // Tokenize and index all fields
            for (_field_name, field_value) in &document.fields {
                let tokens = self.tokenizer.tokenize(field_value);
                for token in tokens {
                    self.index.add_term(&token, &doc_id);
                }
            }

            self.documents.insert(doc_id, document);
            Ok(())
        }

        /// Remove a document from index
        pub fn remove(&mut self, doc_id: &str) -> SearchResult<()> {
            self.index.remove_document(doc_id);
            self.documents
                .remove(doc_id)
                .ok_or_else(|| SearchError::DocumentNotFound(doc_id.to_string()))?;
            Ok(())
        }

        /// Search for documents
        pub fn search(&self, query: &Query) -> SearchResult<Vec<SearchHit>> {
            let tokens = self.tokenizer.tokenize(&query.text);

            // Find matching documents
            let mut doc_scores: HashMap<String, f32> = HashMap::new();

            for token in &tokens {
                if let Some(docs) = self.index.get_documents(token) {
                    for doc_id in docs {
                        *doc_scores.entry(doc_id.clone()).or_insert(0.0) += 1.0;
                    }
                }
            }

            // Convert to search hits
            let mut hits: Vec<SearchHit> = doc_scores
                .into_iter()
                .filter_map(|(id, score)| {
                    self.documents.get(&id).map(|doc| SearchHit {
                        id: id.clone(),
                        score,
                        fields: doc.fields.clone(),
                        metadata: doc.metadata.clone(),
                    })
                })
                .collect();

            // Sort by score descending; treat NaN as equal to avoid panic
            hits.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Apply pagination
            let start = query.offset;
            let end = (query.offset + query.limit).min(hits.len());
            Ok(hits[start..end].to_vec())
        }

        /// Get document count
        pub fn count(&self) -> usize {
            self.documents.len()
        }

        /// Get term count in index
        pub fn term_count(&self) -> usize {
            self.index.index.len()
        }
    }

    impl Default for SearchEngine {
        fn default() -> Self {
            Self::new()
        }
    }

    /// Async search engine (for future database integration)
    #[async_trait]
    pub trait AsyncSearchEngine: Send + Sync {
        async fn index(&self, document: Document) -> SearchResult<()>;
        async fn search(&self, query: &Query) -> SearchResult<Vec<SearchHit>>;
        async fn remove(&self, doc_id: &str) -> SearchResult<()>;
        async fn count(&self) -> SearchResult<usize>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_memory_search_engine() {
        let mut engine = SearchEngine::new();
        let doc = Document::new("1").field("title", "Rust Programming");

        engine.index(doc).unwrap();
        assert_eq!(engine.count(), 1);
    }

    #[test]
    fn test_search_with_query() {
        let mut engine = SearchEngine::new();

        engine
            .index(Document::new("1").field("title", "Rust Programming"))
            .unwrap();
        engine
            .index(Document::new("2").field("title", "Python Programming"))
            .unwrap();

        let query = Query::new("Rust");
        let results = engine.search(&query).unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "1");
    }
}
