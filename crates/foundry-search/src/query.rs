//! Search query builder

use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub fields: Vec<String>,
    pub limit: usize,
    pub offset: usize,
    pub filters: Vec<SearchFilter>,
}

impl SearchQuery {
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            fields: Vec::new(),
            limit: 10,
            offset: 0,
            filters: Vec::new(),
        }
    }

    pub fn fields(mut self, fields: Vec<String>) -> Self {
        self.fields = fields;
        self
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFilter {
    pub field: String,
    pub value: serde_json::Value,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<serde_json::Value>,
    pub total: usize,
    pub took: u64,
}
