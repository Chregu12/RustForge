//! # Foundry Full-Text Search
//!
//! Full-text search with database and Elasticsearch support.

pub mod traits;
pub mod engines;
pub mod query;

pub use traits::{Searchable, SearchEngine};
pub use engines::{DatabaseSearch, ElasticsearchEngine};
pub use query::{SearchQuery, SearchResult};

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Search engine error: {0}")]
    EngineError(String),

    #[error("Database error: {0}")]
    DatabaseError(#[from] sea_orm::DbErr),
}

pub type Result<T> = std::result::Result<T, SearchError>;
