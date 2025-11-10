//! Pagination utilities for RustForge
//!
//! This crate provides offset-based and cursor-based pagination with metadata and links.

use serde::{Deserialize, Serialize};
use std::fmt;
use thiserror::Error;

/// Pagination errors
#[derive(Debug, Error)]
pub enum PaginationError {
    #[error("Invalid page number: {0}")]
    InvalidPage(i64),

    #[error("Invalid per_page value: {0}")]
    InvalidPerPage(i64),

    #[error("Invalid cursor: {0}")]
    InvalidCursor(String),
}

pub type PaginationResult<T> = Result<T, PaginationError>;

/// Offset-based paginator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginator {
    /// Total number of items
    pub total: i64,
    /// Items per page
    pub per_page: i64,
    /// Current page (1-indexed)
    pub current_page: i64,
    /// Total number of pages
    pub last_page: i64,
}

impl Paginator {
    /// Create a new paginator
    pub fn new(total: i64, per_page: i64, current_page: i64) -> PaginationResult<Self> {
        if per_page <= 0 {
            return Err(PaginationError::InvalidPerPage(per_page));
        }

        if current_page <= 0 {
            return Err(PaginationError::InvalidPage(current_page));
        }

        let last_page = (total as f64 / per_page as f64).ceil() as i64;

        Ok(Self {
            total,
            per_page,
            current_page,
            last_page,
        })
    }

    /// Get the offset for SQL queries
    pub fn offset(&self) -> i64 {
        (self.current_page - 1) * self.per_page
    }

    /// Get the limit for SQL queries
    pub fn limit(&self) -> i64 {
        self.per_page
    }

    /// Check if there is a next page
    pub fn has_next(&self) -> bool {
        self.current_page < self.last_page
    }

    /// Check if there is a previous page
    pub fn has_prev(&self) -> bool {
        self.current_page > 1
    }

    /// Get the next page number
    pub fn next_page(&self) -> Option<i64> {
        if self.has_next() {
            Some(self.current_page + 1)
        } else {
            None
        }
    }

    /// Get the previous page number
    pub fn prev_page(&self) -> Option<i64> {
        if self.has_prev() {
            Some(self.current_page - 1)
        } else {
            None
        }
    }

    /// Get starting item number (1-indexed)
    pub fn from(&self) -> i64 {
        if self.total == 0 {
            0
        } else {
            self.offset() + 1
        }
    }

    /// Get ending item number (1-indexed)
    pub fn to(&self) -> i64 {
        let end = self.offset() + self.per_page;
        if end > self.total {
            self.total
        } else {
            end
        }
    }
}

/// Pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub total: i64,
    pub per_page: i64,
    pub current_page: i64,
    pub last_page: i64,
    pub from: i64,
    pub to: i64,
}

impl From<Paginator> for PaginationMeta {
    fn from(paginator: Paginator) -> Self {
        Self {
            total: paginator.total,
            per_page: paginator.per_page,
            current_page: paginator.current_page,
            last_page: paginator.last_page,
            from: paginator.from(),
            to: paginator.to(),
        }
    }
}

/// Pagination links
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationLinks {
    pub first: Option<String>,
    pub last: Option<String>,
    pub prev: Option<String>,
    pub next: Option<String>,
}

impl PaginationLinks {
    /// Create pagination links with a base URL
    pub fn new(base_url: &str, paginator: &Paginator) -> Self {
        let first = Some(format!("{}?page=1", base_url));
        let last = Some(format!("{}?page={}", base_url, paginator.last_page));
        let prev = paginator
            .prev_page()
            .map(|p| format!("{}?page={}", base_url, p));
        let next = paginator
            .next_page()
            .map(|p| format!("{}?page={}", base_url, p));

        Self {
            first,
            last,
            prev,
            next,
        }
    }
}

/// Paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub meta: PaginationMeta,
    pub links: Option<PaginationLinks>,
}

impl<T> PaginatedResponse<T> {
    /// Create a new paginated response
    pub fn new(
        data: Vec<T>,
        paginator: Paginator,
        base_url: Option<&str>,
    ) -> Self {
        let meta = PaginationMeta::from(paginator.clone());
        let links = base_url.map(|url| PaginationLinks::new(url, &paginator));

        Self { data, meta, links }
    }
}

/// Cursor for cursor-based pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// The cursor value (typically an ID or timestamp)
    pub value: String,
    /// Direction (before or after)
    pub direction: CursorDirection,
}

/// Cursor direction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CursorDirection {
    Before,
    After,
}

impl fmt::Display for CursorDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CursorDirection::Before => write!(f, "before"),
            CursorDirection::After => write!(f, "after"),
        }
    }
}

/// Cursor-based paginator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginator {
    pub per_page: i64,
    pub cursor: Option<Cursor>,
}

impl CursorPaginator {
    /// Create a new cursor paginator
    pub fn new(per_page: i64) -> PaginationResult<Self> {
        if per_page <= 0 {
            return Err(PaginationError::InvalidPerPage(per_page));
        }

        Ok(Self {
            per_page,
            cursor: None,
        })
    }

    /// Set cursor to paginate after
    pub fn after(mut self, cursor: String) -> Self {
        self.cursor = Some(Cursor {
            value: cursor,
            direction: CursorDirection::After,
        });
        self
    }

    /// Set cursor to paginate before
    pub fn before(mut self, cursor: String) -> Self {
        self.cursor = Some(Cursor {
            value: cursor,
            direction: CursorDirection::Before,
        });
        self
    }

    /// Get the limit
    pub fn limit(&self) -> i64 {
        self.per_page
    }
}

/// Cursor-based paginated response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPaginatedResponse<T> {
    pub data: Vec<T>,
    pub has_more: bool,
    pub next_cursor: Option<String>,
    pub prev_cursor: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginator_new() {
        let paginator = Paginator::new(100, 10, 1).unwrap();
        assert_eq!(paginator.total, 100);
        assert_eq!(paginator.per_page, 10);
        assert_eq!(paginator.current_page, 1);
        assert_eq!(paginator.last_page, 10);
    }

    #[test]
    fn test_paginator_offset_limit() {
        let paginator = Paginator::new(100, 10, 3).unwrap();
        assert_eq!(paginator.offset(), 20);
        assert_eq!(paginator.limit(), 10);
    }

    #[test]
    fn test_paginator_has_next_prev() {
        let paginator = Paginator::new(100, 10, 5).unwrap();
        assert!(paginator.has_next());
        assert!(paginator.has_prev());

        let first = Paginator::new(100, 10, 1).unwrap();
        assert!(first.has_next());
        assert!(!first.has_prev());

        let last = Paginator::new(100, 10, 10).unwrap();
        assert!(!last.has_next());
        assert!(last.has_prev());
    }

    #[test]
    fn test_paginator_from_to() {
        let paginator = Paginator::new(100, 10, 3).unwrap();
        assert_eq!(paginator.from(), 21);
        assert_eq!(paginator.to(), 30);

        let empty = Paginator::new(0, 10, 1).unwrap();
        assert_eq!(empty.from(), 0);
        assert_eq!(empty.to(), 0);
    }

    #[test]
    fn test_pagination_meta() {
        let paginator = Paginator::new(100, 10, 3).unwrap();
        let meta = PaginationMeta::from(paginator);
        assert_eq!(meta.total, 100);
        assert_eq!(meta.current_page, 3);
        assert_eq!(meta.from, 21);
        assert_eq!(meta.to, 30);
    }

    #[test]
    fn test_pagination_links() {
        let paginator = Paginator::new(100, 10, 3).unwrap();
        let links = PaginationLinks::new("/api/users", &paginator);
        assert_eq!(links.first, Some("/api/users?page=1".to_string()));
        assert_eq!(links.prev, Some("/api/users?page=2".to_string()));
        assert_eq!(links.next, Some("/api/users?page=4".to_string()));
        assert_eq!(links.last, Some("/api/users?page=10".to_string()));
    }

    #[test]
    fn test_paginated_response() {
        let paginator = Paginator::new(3, 10, 1).unwrap();
        let response = PaginatedResponse::new(
            vec![1, 2, 3],
            paginator,
            Some("/api/items"),
        );
        assert_eq!(response.data.len(), 3);
        assert_eq!(response.meta.total, 3);
        assert!(response.links.is_some());
    }

    #[test]
    fn test_cursor_paginator() {
        let paginator = CursorPaginator::new(25).unwrap();
        assert_eq!(paginator.per_page, 25);
        assert!(paginator.cursor.is_none());

        let with_cursor = paginator.after("cursor123".to_string());
        assert!(with_cursor.cursor.is_some());
        assert_eq!(with_cursor.cursor.unwrap().direction, CursorDirection::After);
    }

    #[test]
    fn test_cursor_direction() {
        assert_eq!(CursorDirection::After.to_string(), "after");
        assert_eq!(CursorDirection::Before.to_string(), "before");
    }

    #[test]
    fn test_invalid_per_page() {
        let result = Paginator::new(100, 0, 1);
        assert!(result.is_err());

        let result2 = Paginator::new(100, -5, 1);
        assert!(result2.is_err());
    }

    #[test]
    fn test_invalid_page() {
        let result = Paginator::new(100, 10, 0);
        assert!(result.is_err());

        let result2 = Paginator::new(100, 10, -1);
        assert!(result2.is_err());
    }
}
