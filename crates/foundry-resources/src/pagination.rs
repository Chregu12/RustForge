//! Pagination support for resource collections

use serde::{Serialize, Deserialize};

/// Pagination parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    /// Current page (1-indexed)
    pub page: u64,
    /// Items per page
    pub per_page: u64,
}

impl Pagination {
    /// Create new pagination
    pub fn new(page: u64, per_page: u64) -> Self {
        Self {
            page: page.max(1),
            per_page: per_page.max(1).min(100), // Max 100 items per page
        }
    }

    /// Get offset for database queries
    pub fn offset(&self) -> u64 {
        (self.page - 1) * self.per_page
    }

    /// Get limit for database queries
    pub fn limit(&self) -> u64 {
        self.per_page
    }

    /// Calculate total pages
    pub fn total_pages(&self, total: u64) -> u64 {
        (total as f64 / self.per_page as f64).ceil() as u64
    }

    /// Check if there's a next page
    pub fn has_next(&self, total: u64) -> bool {
        self.page < self.total_pages(total)
    }

    /// Check if there's a previous page
    pub fn has_prev(&self) -> bool {
        self.page > 1
    }
}

impl Default for Pagination {
    fn default() -> Self {
        Self {
            page: 1,
            per_page: 15,
        }
    }
}

/// Pagination metadata for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    /// Current page
    pub current_page: u64,
    /// Items per page
    pub per_page: u64,
    /// Total items
    pub total: u64,
    /// Total pages
    pub total_pages: u64,
    /// First item number
    pub from: Option<u64>,
    /// Last item number
    pub to: Option<u64>,
    /// Links to other pages
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<PaginationLinks>,
}

impl PaginationMeta {
    /// Create metadata from pagination and total
    pub fn from_pagination(pagination: &Pagination, total: u64) -> Self {
        let total_pages = pagination.total_pages(total);
        let from = if total > 0 {
            Some(pagination.offset() + 1)
        } else {
            None
        };
        let to = if total > 0 {
            Some((pagination.offset() + pagination.per_page).min(total))
        } else {
            None
        };

        Self {
            current_page: pagination.page,
            per_page: pagination.per_page,
            total,
            total_pages,
            from,
            to,
            links: None,
        }
    }

    /// Add links
    pub fn with_links(mut self, links: PaginationLinks) -> Self {
        self.links = Some(links);
        self
    }
}

/// Pagination links for HATEOAS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationLinks {
    /// Link to first page
    pub first: String,
    /// Link to last page
    pub last: String,
    /// Link to previous page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    /// Link to next page
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl PaginationLinks {
    /// Create links from base URL and pagination
    pub fn from_url(base_url: &str, pagination: &Pagination, total: u64) -> Self {
        let total_pages = pagination.total_pages(total);

        let first = format!("{}?page=1&per_page={}", base_url, pagination.per_page);
        let last = format!("{}?page={}&per_page={}", base_url, total_pages, pagination.per_page);

        let prev = if pagination.has_prev() {
            Some(format!(
                "{}?page={}&per_page={}",
                base_url,
                pagination.page - 1,
                pagination.per_page
            ))
        } else {
            None
        };

        let next = if pagination.has_next(total) {
            Some(format!(
                "{}?page={}&per_page={}",
                base_url,
                pagination.page + 1,
                pagination.per_page
            ))
        } else {
            None
        };

        Self {
            first,
            last,
            prev,
            next,
        }
    }
}

/// Cursor-based pagination for large datasets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorPagination {
    /// Cursor (usually an ID or timestamp)
    pub cursor: Option<String>,
    /// Items per page
    pub limit: u64,
}

impl CursorPagination {
    pub fn new(cursor: Option<String>, limit: u64) -> Self {
        Self {
            cursor,
            limit: limit.max(1).min(100),
        }
    }
}

/// Cursor pagination metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CursorMeta {
    /// Next cursor
    pub next_cursor: Option<String>,
    /// Previous cursor
    pub prev_cursor: Option<String>,
    /// Has more items
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pagination_offset() {
        let pagination = Pagination::new(2, 10);
        assert_eq!(pagination.offset(), 10);
        assert_eq!(pagination.limit(), 10);
    }

    #[test]
    fn test_total_pages() {
        let pagination = Pagination::new(1, 10);
        assert_eq!(pagination.total_pages(25), 3);
        assert_eq!(pagination.total_pages(10), 1);
    }

    #[test]
    fn test_has_next() {
        let pagination = Pagination::new(1, 10);
        assert!(pagination.has_next(25));
        assert!(!pagination.has_next(10));
    }

    #[test]
    fn test_pagination_meta() {
        let pagination = Pagination::new(2, 10);
        let meta = PaginationMeta::from_pagination(&pagination, 25);

        assert_eq!(meta.current_page, 2);
        assert_eq!(meta.total, 25);
        assert_eq!(meta.from, Some(11));
        assert_eq!(meta.to, Some(20));
    }

    #[test]
    fn test_pagination_links() {
        let pagination = Pagination::new(2, 10);
        let links = PaginationLinks::from_url("/api/users", &pagination, 25);

        assert!(links.prev.is_some());
        assert!(links.next.is_some());
        assert_eq!(links.first, "/api/users?page=1&per_page=10");
    }
}
