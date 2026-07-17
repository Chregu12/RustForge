//! Pagination utilities for GraphQL
//!
//! Provides cursor-based and offset-based pagination.

use async_graphql::{InputObject, Object, SimpleObject};
use serde::{Deserialize, Serialize};

/// Cursor-based pagination input
#[derive(InputObject, Debug, Clone)]
pub struct CursorPaginationInput {
    /// The cursor to start from
    pub after: Option<String>,
    /// The cursor to end at
    pub before: Option<String>,
    /// Number of items to fetch
    pub first: Option<i32>,
    /// Number of items to fetch from the end
    pub last: Option<i32>,
}

impl Default for CursorPaginationInput {
    fn default() -> Self {
        Self {
            after: None,
            before: None,
            first: Some(10),
            last: None,
        }
    }
}

/// Offset-based pagination input
#[derive(InputObject, Debug, Clone)]
pub struct OffsetPaginationInput {
    /// Page number (0-indexed)
    pub page: Option<i32>,
    /// Items per page
    pub per_page: Option<i32>,
}

impl Default for OffsetPaginationInput {
    fn default() -> Self {
        Self {
            page: Some(0),
            per_page: Some(10),
        }
    }
}

impl OffsetPaginationInput {
    /// Calculate the offset
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(0).max(0) as i64;
        let per_page = self.per_page.unwrap_or(10).max(1) as i64;
        page * per_page
    }

    /// Get the limit
    pub fn limit(&self) -> i64 {
        self.per_page.unwrap_or(10).max(1) as i64
    }
}

/// Page information for cursor-based pagination
#[derive(SimpleObject, Debug, Clone, Serialize, Deserialize)]
pub struct PageInfo {
    /// Whether there is a next page
    pub has_next_page: bool,
    /// Whether there is a previous page
    pub has_previous_page: bool,
    /// The cursor of the first item
    pub start_cursor: Option<String>,
    /// The cursor of the last item
    pub end_cursor: Option<String>,
}

impl Default for PageInfo {
    fn default() -> Self {
        Self {
            has_next_page: false,
            has_previous_page: false,
            start_cursor: None,
            end_cursor: None,
        }
    }
}

/// Edge for cursor-based pagination
#[derive(Debug, Clone)]
pub struct Edge<T> {
    /// The cursor for this edge
    pub cursor: String,
    /// The node data
    pub node: T,
}

#[Object]
impl<T> Edge<T>
where
    T: async_graphql::OutputType,
{
    /// The cursor for this edge
    async fn cursor(&self) -> &str {
        &self.cursor
    }

    /// The node data
    async fn node(&self) -> &T {
        &self.node
    }
}

/// Connection for cursor-based pagination
#[derive(Debug, Clone)]
pub struct Connection<T> {
    /// The edges
    pub edges: Vec<Edge<T>>,
    /// Page information
    pub page_info: PageInfo,
    /// Total count (optional)
    pub total_count: Option<i64>,
}

#[Object]
impl<T> Connection<T>
where
    T: async_graphql::OutputType,
{
    /// The edges
    async fn edges(&self) -> &[Edge<T>] {
        &self.edges
    }

    /// Page information
    async fn page_info(&self) -> &PageInfo {
        &self.page_info
    }

    /// Total count
    async fn total_count(&self) -> Option<i64> {
        self.total_count
    }
}

impl<T> Connection<T> {
    /// Create a new connection
    pub fn new(edges: Vec<Edge<T>>, page_info: PageInfo) -> Self {
        Self {
            edges,
            page_info,
            total_count: None,
        }
    }

    /// Create a connection with total count
    pub fn with_total_count(mut self, total_count: i64) -> Self {
        self.total_count = Some(total_count);
        self
    }
}

/// Paginated result for offset-based pagination
#[derive(Debug, Clone)]
pub struct PaginatedResult<T> {
    /// The data items
    pub data: Vec<T>,
    /// Current page (0-indexed)
    pub page: i32,
    /// Items per page
    pub per_page: i32,
    /// Total number of items
    pub total: i64,
    /// Total number of pages
    pub total_pages: i32,
}

#[Object]
impl<T> PaginatedResult<T>
where
    T: async_graphql::OutputType,
{
    /// The data items
    async fn data(&self) -> &[T] {
        &self.data
    }

    /// Current page
    async fn page(&self) -> i32 {
        self.page
    }

    /// Items per page
    async fn per_page(&self) -> i32 {
        self.per_page
    }

    /// Total number of items
    async fn total(&self) -> i64 {
        self.total
    }

    /// Total number of pages
    async fn total_pages(&self) -> i32 {
        self.total_pages
    }

    /// Whether there is a next page
    async fn has_next_page(&self) -> bool {
        self.page < self.total_pages - 1
    }

    /// Whether there is a previous page
    async fn has_previous_page(&self) -> bool {
        self.page > 0
    }
}

impl<T> PaginatedResult<T> {
    /// Create a new paginated result
    pub fn new(data: Vec<T>, page: i32, per_page: i32, total: i64) -> Self {
        let per_page = per_page.max(1);
        let total_pages = ((total as f64) / (per_page as f64)).ceil() as i32;

        Self {
            data,
            page,
            per_page,
            total,
            total_pages,
        }
    }
}

/// Create a cursor from an ID
pub fn encode_cursor(id: i64) -> String {
    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, id.to_string())
}

/// Decode a cursor to an ID
pub fn decode_cursor(cursor: &str) -> Result<i64, String> {
    let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, cursor)
        .map_err(|e| format!("Invalid cursor: {}", e))?;

    let id_str =
        String::from_utf8(decoded).map_err(|e| format!("Invalid cursor encoding: {}", e))?;

    id_str
        .parse::<i64>()
        .map_err(|e| format!("Invalid cursor ID: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offset_pagination_calculation() {
        let input = OffsetPaginationInput {
            page: Some(2),
            per_page: Some(10),
        };

        assert_eq!(input.offset(), 20);
        assert_eq!(input.limit(), 10);
    }

    #[test]
    fn test_offset_pagination_defaults() {
        let input = OffsetPaginationInput::default();
        assert_eq!(input.offset(), 0);
        assert_eq!(input.limit(), 10);
    }

    #[test]
    fn test_paginated_result_calculations() {
        let data = vec![1, 2, 3, 4, 5];
        let result = PaginatedResult::new(data, 0, 10, 45);

        assert_eq!(result.total_pages, 5);
        assert_eq!(result.page, 0);
    }

    #[test]
    fn test_cursor_encoding() {
        let id = 42i64;
        let cursor = encode_cursor(id);
        let decoded = decode_cursor(&cursor).unwrap();

        assert_eq!(decoded, id);
    }

    #[test]
    fn test_invalid_cursor() {
        let result = decode_cursor("invalid");
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connection_creation() {
        let edges = vec![
            Edge {
                cursor: "cursor1".to_string(),
                node: 1,
            },
            Edge {
                cursor: "cursor2".to_string(),
                node: 2,
            },
        ];

        let page_info = PageInfo {
            has_next_page: true,
            has_previous_page: false,
            start_cursor: Some("cursor1".to_string()),
            end_cursor: Some("cursor2".to_string()),
        };

        let connection = Connection::new(edges, page_info).with_total_count(100);

        assert_eq!(connection.total_count, Some(100));
        assert_eq!(connection.edges.len(), 2);
    }
}
