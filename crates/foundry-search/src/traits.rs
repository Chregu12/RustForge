//! Search traits

use async_trait::async_trait;
use crate::{Result, SearchQuery, SearchResult};

#[async_trait]
pub trait Searchable: Send + Sync {
    fn search_fields() -> Vec<&'static str>;
    fn search_weights() -> Vec<f32> {
        vec![1.0; Self::search_fields().len()]
    }
}

#[async_trait]
pub trait SearchEngine: Send + Sync {
    async fn search(&self, query: &SearchQuery) -> Result<SearchResult>;
    async fn index<T: Searchable>(&self, items: Vec<T>) -> Result<()>;
    async fn delete(&self, ids: Vec<String>) -> Result<()>;
}
