//! Resource collection handling with pagination support.

use serde::{Serialize, Deserialize};
use serde_json::Value;

/// Pagination metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationMeta {
    pub current_page: u32,
    pub last_page: u32,
    pub per_page: u32,
    pub total: u64,
    pub from: Option<u64>,
    pub to: Option<u64>,
}

impl PaginationMeta {
    /// Create new pagination metadata.
    pub fn new(current_page: u32, per_page: u32, total: u64) -> Self {
        let last_page = ((total as f64) / (per_page as f64)).ceil() as u32;
        let from = if total > 0 {
            Some(((current_page - 1) * per_page) as u64 + 1)
        } else {
            None
        };
        let to = if total > 0 {
            Some(std::cmp::min(current_page as u64 * per_page as u64, total))
        } else {
            None
        };

        Self {
            current_page,
            last_page,
            per_page,
            total,
            from,
            to,
        }
    }

    /// Check if there's a next page.
    pub fn has_next_page(&self) -> bool {
        self.current_page < self.last_page
    }

    /// Check if there's a previous page.
    pub fn has_previous_page(&self) -> bool {
        self.current_page > 1
    }
}

/// Trait for resource collections.
pub trait ResourceCollection: Serialize {
    type Item;

    /// Get the items in the collection.
    fn items(&self) -> &[Self::Item];

    /// Transform collection into JSON.
    fn to_json(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }

    /// Wrap the collection with a custom key.
    fn wrap(self, key: &str) -> WrappedCollection<Self>
    where
        Self: Sized,
    {
        WrappedCollection {
            collection: self,
            key: key.to_string(),
        }
    }
}

/// A collection of resources.
#[derive(Debug, Clone, Serialize)]
pub struct Collection<T> {
    data: Vec<T>,
}

impl<T> Collection<T> {
    /// Create a new collection.
    pub fn new(data: Vec<T>) -> Self {
        Self { data }
    }

    /// Create a collection from items.
    pub fn from_items(items: Vec<T>) -> Self {
        Self::new(items)
    }

    /// Get the number of items.
    pub fn count(&self) -> usize {
        self.data.len()
    }

    /// Check if collection is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

impl<T> ResourceCollection for Collection<T>
where
    T: Serialize,
{
    type Item = T;

    fn items(&self) -> &[Self::Item] {
        &self.data
    }
}

/// A paginated collection of resources.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedCollection<T> {
    data: Vec<T>,
    meta: PaginationMeta,
    #[serde(skip_serializing_if = "Option::is_none")]
    links: Option<PaginationLinks>,
}

impl<T> PaginatedCollection<T> {
    /// Create a new paginated collection.
    pub fn new(data: Vec<T>, meta: PaginationMeta) -> Self {
        Self {
            data,
            meta,
            links: None,
        }
    }

    /// Add pagination links.
    pub fn with_links(mut self, links: PaginationLinks) -> Self {
        self.links = Some(links);
        self
    }

    /// Get pagination metadata.
    pub fn meta(&self) -> &PaginationMeta {
        &self.meta
    }
}

impl<T: Serialize> ResourceCollection for PaginatedCollection<T> {
    type Item = T;

    fn items(&self) -> &[Self::Item] {
        &self.data
    }
}

/// Pagination links.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationLinks {
    pub first: String,
    pub last: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next: Option<String>,
}

impl PaginationLinks {
    /// Create new pagination links.
    pub fn new(base_url: &str, meta: &PaginationMeta) -> Self {
        let first = format!("{}?page=1", base_url);
        let last = format!("{}?page={}", base_url, meta.last_page);
        let prev = if meta.has_previous_page() {
            Some(format!("{}?page={}", base_url, meta.current_page - 1))
        } else {
            None
        };
        let next = if meta.has_next_page() {
            Some(format!("{}?page={}", base_url, meta.current_page + 1))
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

/// A wrapped collection with custom key.
#[derive(Debug, Clone, Serialize)]
pub struct WrappedCollection<T> {
    #[serde(flatten)]
    collection: T,
    #[serde(skip)]
    key: String,
}

impl<T: Serialize> WrappedCollection<T> {
    /// Get JSON representation with wrapping.
    pub fn to_json(&self) -> serde_json::Result<Value> {
        let mut map = serde_json::Map::new();
        map.insert(self.key.clone(), serde_json::to_value(&self.collection)?);
        Ok(Value::Object(map))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Serialize)]
    struct TestItem {
        id: i64,
        name: String,
    }

    #[test]
    fn test_pagination_meta() {
        let meta = PaginationMeta::new(1, 10, 25);
        assert_eq!(meta.current_page, 1);
        assert_eq!(meta.last_page, 3);
        assert_eq!(meta.total, 25);
        assert_eq!(meta.from, Some(1));
        assert_eq!(meta.to, Some(10));
        assert!(meta.has_next_page());
        assert!(!meta.has_previous_page());
    }

    #[test]
    fn test_collection() {
        let items = vec![
            TestItem { id: 1, name: "Item 1".to_string() },
            TestItem { id: 2, name: "Item 2".to_string() },
        ];

        let collection = Collection::new(items);
        assert_eq!(collection.count(), 2);
        assert!(!collection.is_empty());
    }

    #[test]
    fn test_paginated_collection() {
        let items = vec![
            TestItem { id: 1, name: "Item 1".to_string() },
        ];
        let meta = PaginationMeta::new(1, 10, 1);
        let collection = PaginatedCollection::new(items, meta);

        assert_eq!(collection.items().len(), 1);
        assert_eq!(collection.meta().total, 1);
    }

    #[test]
    fn test_pagination_links() {
        let meta = PaginationMeta::new(2, 10, 25);
        let links = PaginationLinks::new("/api/items", &meta);

        assert_eq!(links.first, "/api/items?page=1");
        assert_eq!(links.last, "/api/items?page=3");
        assert_eq!(links.prev, Some("/api/items?page=1".to_string()));
        assert_eq!(links.next, Some("/api/items?page=3".to_string()));
    }

    #[test]
    fn test_wrapped_collection() {
        let items = vec![
            TestItem { id: 1, name: "Item 1".to_string() },
        ];
        let collection = Collection::new(items);
        let wrapped = collection.wrap("items");

        let json = wrapped.to_json().unwrap();
        assert!(json["items"].is_object());
        assert!(json["items"]["data"].is_array());
    }
}
