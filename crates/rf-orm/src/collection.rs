//! # Laravel-Style Collections
//!
//! A powerful collection wrapper around Vec<T> providing Laravel Collection-like methods.
//!
//! ## Overview
//!
//! Collections provide a fluent, convenient wrapper for working with arrays of data.
//! Inspired by Laravel Collections, this provides 20+ helpful methods for transforming,
//! filtering, and manipulating data.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::collection::*;
//!
//! #[derive(Clone, PartialEq, Eq, Hash)]
//! struct User { email: String, active: bool }
//!
//! let users = Collection::new(vec![
//!     User { email: "a@example.com".into(), active: true },
//!     User { email: "b@example.com".into(), active: false },
//! ]);
//!
//! let emails = users
//!     .filter(|u| u.active)
//!     .pluck(|u| u.email.clone());
//! ```

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::hash::Hash;

/// Laravel-style Collection wrapper around Vec<T>
///
/// Provides fluent methods for data transformation and manipulation.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::collection::Collection;
///
/// let numbers = Collection::new(vec![1, 2, 3, 4, 5]);
///
/// let doubled = numbers
///     .map(|n| n * 2)
///     .filter(|n| *n > 5)
///     .to_vec();
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Collection<T> {
    items: Vec<T>,
}

impl<T> Collection<T> {
    /// Create a new collection from a vector
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    /// Create an empty collection
    pub fn empty() -> Self {
        Self { items: Vec::new() }
    }

    /// Create a collection from an iterator
    pub fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self {
            items: iter.into_iter().collect(),
        }
    }

    // ===== Transformation Methods =====

    /// Transform each item in the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3]);
    /// let doubled = numbers.map(|n| n * 2);
    /// ```
    pub fn map<U, F>(self, f: F) -> Collection<U>
    where
        F: FnMut(T) -> U,
    {
        Collection {
            items: self.items.into_iter().map(f).collect(),
        }
    }

    /// Filter items that match a predicate
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3, 4, 5]);
    /// let evens = numbers.filter(|n| *n % 2 == 0);
    /// ```
    pub fn filter<F>(self, mut predicate: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Self {
            items: self
                .items
                .into_iter()
                .filter(|item| predicate(item))
                .collect(),
        }
    }

    /// Filter items that don't match a predicate (opposite of filter)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3, 4, 5]);
    /// let odds = numbers.reject(|n| *n % 2 == 0);
    /// ```
    pub fn reject<F>(self, mut predicate: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Self {
            items: self
                .items
                .into_iter()
                .filter(|item| !predicate(item))
                .collect(),
        }
    }

    /// Transform the collection with mutable access
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let collection = Collection::new(vec![1, 2, 3]);
    /// let result = collection.transform(|coll| {
    ///     coll.map(|n| n * 2)
    /// });
    /// ```
    pub fn transform<F>(self, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        f(self)
    }

    /// Execute a callback on the collection and return the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let collection = Collection::new(vec![1, 2, 3])
    ///     .tap(|coll| println!("Count: {}", coll.count()))
    ///     .map(|n| n * 2);
    /// ```
    pub fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self),
    {
        f(&self);
        self
    }

    // ===== Data Extraction =====

    /// Extract a single property from each item
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// # struct User { id: i32, name: String }
    /// let users = Collection::new(vec![
    ///     User { id: 1, name: "John".to_string() },
    ///     User { id: 2, name: "Jane".to_string() },
    /// ]);
    /// let names = users.pluck(|u| &u.name);
    /// ```
    pub fn pluck<U, F>(&self, f: F) -> Vec<U>
    where
        F: Fn(&T) -> U,
    {
        self.items.iter().map(f).collect()
    }

    /// Get the first item
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3]);
    /// assert_eq!(numbers.first(), Some(&1));
    /// ```
    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    /// Get the first item matching a predicate
    pub fn first_where<F>(&self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().find(|item| predicate(item))
    }

    /// Get the last item
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3]);
    /// assert_eq!(numbers.last(), Some(&3));
    /// ```
    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }

    /// Get the last item matching a predicate
    pub fn last_where<F>(&self, predicate: F) -> Option<&T>
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().rev().find(|item| predicate(item))
    }

    // ===== Slicing & Chunking =====

    /// Split the collection into chunks of a given size
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3, 4, 5]);
    /// let chunks = numbers.chunk(2);
    /// // Returns: [[1, 2], [3, 4], [5]]
    /// ```
    pub fn chunk(self, size: usize) -> Vec<Collection<T>>
    where
        T: Clone,
    {
        self.items
            .chunks(size)
            .map(|chunk| Collection::new(chunk.to_vec()))
            .collect()
    }

    /// Take the first n items
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3, 4, 5]);
    /// let first_three = numbers.take(3);
    /// ```
    pub fn take(self, n: usize) -> Self {
        Self {
            items: self.items.into_iter().take(n).collect(),
        }
    }

    /// Skip the first n items
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3, 4, 5]);
    /// let after_two = numbers.skip(2);
    /// ```
    pub fn skip(self, n: usize) -> Self {
        Self {
            items: self.items.into_iter().skip(n).collect(),
        }
    }

    /// Slice the collection from start to end
    pub fn slice(self, start: usize, end: usize) -> Self
    where
        T: Clone,
    {
        let end = end.min(self.items.len());
        let start = start.min(end);
        Self {
            items: self.items[start..end].to_vec(),
        }
    }

    // ===== Grouping & Sorting =====

    /// Group items by a key function
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// # struct User { role: String, name: String }
    /// let users = Collection::new(vec![
    ///     User { role: "admin".to_string(), name: "John".to_string() },
    ///     User { role: "user".to_string(), name: "Jane".to_string() },
    /// ]);
    /// let by_role = users.group_by(|u| u.role.clone());
    /// ```
    pub fn group_by<K, F>(self, f: F) -> HashMap<K, Collection<T>>
    where
        K: Eq + Hash,
        F: Fn(&T) -> K,
    {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();
        for item in self.items {
            let key = f(&item);
            groups.entry(key).or_default().push(item);
        }
        groups
            .into_iter()
            .map(|(k, v)| (k, Collection::new(v)))
            .collect()
    }

    /// Remove duplicate items based on a key function
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// # struct User { id: i32, name: String }
    /// let users = Collection::new(vec![
    ///     User { id: 1, name: "John".to_string() },
    ///     User { id: 1, name: "John Doe".to_string() },
    /// ]);
    /// let unique = users.unique_by(|u| u.id);
    /// ```
    pub fn unique_by<K, F>(self, f: F) -> Self
    where
        K: Eq + Hash,
        F: Fn(&T) -> K,
    {
        let mut seen = HashSet::new();
        Self {
            items: self
                .items
                .into_iter()
                .filter(|item| seen.insert(f(item)))
                .collect(),
        }
    }

    /// Remove duplicate items (requires T: Eq + Hash)
    pub fn unique(self) -> Self
    where
        T: Eq + Hash + Clone,
    {
        let mut seen = HashSet::new();
        Self {
            items: self
                .items
                .into_iter()
                .filter(|item| seen.insert(item.clone()))
                .collect(),
        }
    }

    /// Sort items using a comparator function
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![3, 1, 4, 1, 5]);
    /// let sorted = numbers.sort_by(|a, b| a.cmp(b));
    /// ```
    pub fn sort_by<F>(mut self, compare: F) -> Self
    where
        F: FnMut(&T, &T) -> std::cmp::Ordering,
    {
        self.items.sort_by(compare);
        self
    }

    /// Sort items (requires T: Ord)
    pub fn sort(mut self) -> Self
    where
        T: Ord,
    {
        self.items.sort();
        self
    }

    /// Reverse the collection order
    pub fn reverse(mut self) -> Self {
        self.items.reverse();
        self
    }

    // ===== Aggregation =====

    /// Count the items in the collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let numbers = Collection::new(vec![1, 2, 3]);
    /// assert_eq!(numbers.count(), 3);
    /// ```
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Check if any item matches a predicate
    pub fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().any(predicate)
    }

    /// Check if all items match a predicate
    pub fn every<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        self.items.iter().all(predicate)
    }

    // ===== Iteration =====

    /// Execute a callback for each item
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// Collection::new(vec![1, 2, 3])
    ///     .each(|n| println!("{}", n));
    /// ```
    pub fn each<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T),
    {
        for item in &self.items {
            f(item);
        }
        self
    }

    // ===== Conversion =====

    /// Convert the collection to a Vec
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::collection::Collection;
    /// let collection = Collection::new(vec![1, 2, 3]);
    /// let vec = collection.to_vec();
    /// ```
    pub fn to_vec(self) -> Vec<T> {
        self.items
    }

    /// Convert the collection to JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error>
    where
        T: Serialize,
    {
        serde_json::to_string(&self.items)
    }

    /// Get an iterator over the items
    pub fn iter(&self) -> std::slice::Iter<'_, T> {
        self.items.iter()
    }

    /// Get a mutable iterator over the items
    pub fn iter_mut(&mut self) -> std::slice::IterMut<'_, T> {
        self.items.iter_mut()
    }

    // ===== Numeric Methods (for numeric types) =====

    /// Sum all items (for numeric types)
    pub fn sum(&self) -> T
    where
        T: std::iter::Sum + Copy,
    {
        self.items.iter().copied().sum()
    }

    /// Calculate the average (for numeric types)
    pub fn avg(&self) -> Option<f64>
    where
        T: Into<f64> + Copy,
    {
        if self.is_empty() {
            None
        } else {
            let sum: f64 = self.items.iter().map(|&x| x.into()).sum();
            Some(sum / self.count() as f64)
        }
    }

    /// Find the minimum value
    pub fn min(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.items.iter().min()
    }

    /// Find the maximum value
    pub fn max(&self) -> Option<&T>
    where
        T: Ord,
    {
        self.items.iter().max()
    }

    /// Paginate the collection into pages
    ///
    /// # Arguments
    ///
    /// * `per_page` - Number of items per page (minimum 1)
    /// * `page` - Page number (1-indexed, minimum 1)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::collection::Collection;
    ///
    /// let numbers = Collection::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    /// let page = numbers.paginate(3, 2);
    ///
    /// assert_eq!(page.current_page, 2);
    /// assert_eq!(page.per_page, 3);
    /// assert_eq!(page.total, 10);
    /// assert_eq!(page.last_page, 4);
    /// assert_eq!(page.items.count(), 3);
    /// ```
    pub fn paginate(&self, per_page: usize, page: usize) -> PaginatedCollection<T>
    where
        T: Clone,
    {
        let total = self.items.len();
        let per_page = per_page.max(1);
        let page = page.max(1);
        let last_page = if total == 0 {
            1
        } else {
            (total + per_page - 1) / per_page
        };
        let offset = page.saturating_sub(1) * per_page;
        let items = if offset >= total {
            Vec::new()
        } else {
            self.items[offset..(offset + per_page).min(total)].to_vec()
        };
        PaginatedCollection {
            items: Collection::new(items),
            total,
            per_page,
            current_page: page,
            last_page,
        }
    }
}

/// A paginated view of a collection
///
/// Returned by `Collection::paginate()`, contains the items for the current page
/// along with pagination metadata.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::collection::Collection;
///
/// let coll = Collection::new((1..=25).collect::<Vec<_>>());
/// let page = coll.paginate(10, 1);
///
/// assert_eq!(page.total, 25);
/// assert_eq!(page.last_page, 3);
/// assert!(page.has_more_pages());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedCollection<T> {
    /// Items on the current page
    pub items: Collection<T>,
    /// Total number of items across all pages
    pub total: usize,
    /// Number of items per page
    pub per_page: usize,
    /// Current page number (1-indexed)
    pub current_page: usize,
    /// Last page number
    pub last_page: usize,
}

impl<T: Clone> PaginatedCollection<T> {
    /// Whether there are more pages after the current one
    pub fn has_more_pages(&self) -> bool {
        self.current_page < self.last_page
    }

    /// Whether this is the first page
    pub fn is_first_page(&self) -> bool {
        self.current_page == 1
    }

    /// Whether this is the last page
    pub fn is_last_page(&self) -> bool {
        self.current_page >= self.last_page
    }

    /// 1-indexed position of the first item on this page (0 if empty)
    pub fn from(&self) -> usize {
        if self.total == 0 {
            0
        } else {
            (self.current_page.saturating_sub(1)) * self.per_page + 1
        }
    }

    /// 1-indexed position of the last item on this page
    pub fn to(&self) -> usize {
        (self.current_page * self.per_page).min(self.total)
    }
}

// ===== Trait Implementations =====

impl<T> IntoIterator for Collection<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

impl<T> From<Vec<T>> for Collection<T> {
    fn from(items: Vec<T>) -> Self {
        Self::new(items)
    }
}

impl<T> FromIterator<T> for Collection<T> {
    fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
        Self::new(iter.into_iter().collect())
    }
}

/// Extension trait to convert Vec into Collection
pub trait IntoCollection {
    type Item;

    /// Convert into a Collection
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::collection::IntoCollection;
    ///
    /// let vec = vec![1, 2, 3];
    /// let collection = vec.into_collection();
    /// ```
    fn into_collection(self) -> Collection<Self::Item>;
}

impl<T> IntoCollection for Vec<T> {
    type Item = T;

    fn into_collection(self) -> Collection<T> {
        Collection::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_creation() {
        let coll = Collection::new(vec![1, 2, 3]);
        assert_eq!(coll.count(), 3);
    }

    #[test]
    fn test_collection_map() {
        let coll = Collection::new(vec![1, 2, 3]);
        let doubled = coll.map(|n| n * 2).to_vec();
        assert_eq!(doubled, vec![2, 4, 6]);
    }

    #[test]
    fn test_collection_filter() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let evens = coll.filter(|n| n % 2 == 0).to_vec();
        assert_eq!(evens, vec![2, 4]);
    }

    #[test]
    fn test_collection_reject() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let odds = coll.reject(|n| n % 2 == 0).to_vec();
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_collection_first_last() {
        let coll = Collection::new(vec![1, 2, 3]);
        assert_eq!(coll.first(), Some(&1));
        assert_eq!(coll.last(), Some(&3));
    }

    #[test]
    fn test_collection_take_skip() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.clone().take(3).to_vec(), vec![1, 2, 3]);
        assert_eq!(coll.skip(2).to_vec(), vec![3, 4, 5]);
    }

    #[test]
    fn test_collection_chunk() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let chunks = coll.chunk(2);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].clone().to_vec(), vec![1, 2]);
        assert_eq!(chunks[1].clone().to_vec(), vec![3, 4]);
        assert_eq!(chunks[2].clone().to_vec(), vec![5]);
    }

    #[test]
    fn test_collection_unique() {
        let coll = Collection::new(vec![1, 2, 2, 3, 3, 3]);
        let unique = coll.unique().to_vec();
        assert_eq!(unique, vec![1, 2, 3]);
    }

    #[test]
    fn test_collection_sum() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.sum(), 15);
    }

    #[test]
    fn test_collection_min_max() {
        let coll = Collection::new(vec![3, 1, 4, 1, 5]);
        assert_eq!(coll.min(), Some(&1));
        assert_eq!(coll.max(), Some(&5));
    }

    #[test]
    fn test_collection_contains() {
        let coll = Collection::new(vec![1, 2, 3]);
        assert!(coll.contains(|n| *n == 2));
        assert!(!coll.contains(|n| *n == 5));
    }

    #[test]
    fn test_collection_every() {
        let coll = Collection::new(vec![2, 4, 6]);
        assert!(coll.every(|n| n % 2 == 0));
        assert!(!coll.every(|n| *n > 5));
    }

    #[test]
    fn test_into_collection() {
        let vec = vec![1, 2, 3];
        let coll = vec.into_collection();
        assert_eq!(coll.count(), 3);
    }

    #[test]
    fn test_paginate_basic() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let page = coll.paginate(3, 1);

        assert_eq!(page.total, 10);
        assert_eq!(page.per_page, 3);
        assert_eq!(page.current_page, 1);
        assert_eq!(page.last_page, 4);
        assert_eq!(page.items.count(), 3);
        assert_eq!(page.items.clone().to_vec(), vec![1, 2, 3]);
    }

    #[test]
    fn test_paginate_second_page() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let page = coll.paginate(3, 2);

        assert_eq!(page.items.clone().to_vec(), vec![4, 5, 6]);
        assert!(page.has_more_pages());
        assert!(!page.is_first_page());
    }

    #[test]
    fn test_paginate_last_page() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let page = coll.paginate(3, 4);

        assert_eq!(page.items.clone().to_vec(), vec![10]);
        assert!(page.is_last_page());
        assert!(!page.has_more_pages());
    }

    #[test]
    fn test_paginate_beyond_last_page() {
        let coll = Collection::new(vec![1, 2, 3]);
        let page = coll.paginate(10, 5);

        assert_eq!(page.items.count(), 0);
    }

    #[test]
    fn test_paginate_empty_collection() {
        let coll: Collection<i32> = Collection::empty();
        let page = coll.paginate(10, 1);

        assert_eq!(page.total, 0);
        assert_eq!(page.last_page, 1);
        assert_eq!(page.from(), 0);
    }
}
