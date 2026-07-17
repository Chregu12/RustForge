//! Laravel-style collection API.

use std::collections::HashMap;
use std::hash::Hash;

/// A collection wrapper with fluent methods.
#[derive(Debug, Clone)]
pub struct Collection<T> {
    items: Vec<T>,
}

impl<T> Collection<T> {
    /// Create a new collection.
    pub fn new(items: Vec<T>) -> Self {
        Self { items }
    }

    /// Get the number of items.
    pub fn count(&self) -> usize {
        self.items.len()
    }

    /// Check if collection is empty.
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Get all items.
    pub fn all(&self) -> &[T] {
        &self.items
    }

    /// Get all items as a vector.
    pub fn to_vec(self) -> Vec<T> {
        self.items
    }

    /// Get the first item.
    pub fn first(&self) -> Option<&T> {
        self.items.first()
    }

    /// Get the last item.
    pub fn last(&self) -> Option<&T> {
        self.items.last()
    }

    /// Map over the collection.
    pub fn map<U, F>(self, f: F) -> Collection<U>
    where
        F: FnMut(T) -> U,
    {
        Collection::new(self.items.into_iter().map(f).collect())
    }

    /// Filter the collection.
    pub fn filter<F>(self, mut f: F) -> Collection<T>
    where
        F: FnMut(&T) -> bool,
    {
        Collection::new(self.items.into_iter().filter(|item| f(item)).collect())
    }

    /// Take the first n items.
    pub fn take(self, n: usize) -> Collection<T> {
        Collection::new(self.items.into_iter().take(n).collect())
    }

    /// Skip the first n items.
    pub fn skip(self, n: usize) -> Collection<T> {
        Collection::new(self.items.into_iter().skip(n).collect())
    }

    /// Chunk the collection into smaller collections.
    pub fn chunk(self, size: usize) -> Vec<Collection<T>>
    where
        T: Clone,
    {
        let size = size.max(1);
        self.items
            .chunks(size)
            .map(|chunk| Collection::new(chunk.to_vec()))
            .collect()
    }

    /// Reverse the collection.
    pub fn reverse(mut self) -> Self {
        self.items.reverse();
        self
    }

    /// Execute a closure on each item.
    pub fn each<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T),
    {
        for item in &self.items {
            f(item);
        }
        self
    }

    /// Reduce the collection to a single value.
    pub fn reduce<U, F>(self, init: U, f: F) -> U
    where
        F: FnMut(U, T) -> U,
    {
        self.items.into_iter().fold(init, f)
    }

    /// Check if any item satisfies the predicate.
    pub fn any<F>(&self, f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        self.items.iter().any(f)
    }

    /// Check if all items satisfy the predicate.
    pub fn all_match<F>(&self, f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        self.items.iter().all(f)
    }

    /// Find the first item matching the predicate.
    pub fn find<F>(&self, mut f: F) -> Option<&T>
    where
        F: FnMut(&T) -> bool,
    {
        self.items.iter().find(|item| f(item))
    }

    /// Pluck values by key.
    pub fn pluck<U, F>(self, f: F) -> Collection<U>
    where
        F: FnMut(&T) -> U,
    {
        let items: Vec<U> = self.items.iter().map(f).collect();
        Collection::new(items)
    }
}

impl<T: Clone> Collection<T> {
    /// Get a specific item by index.
    pub fn get(&self, index: usize) -> Option<T> {
        self.items.get(index).cloned()
    }
}

impl<T: PartialEq> Collection<T> {
    /// Check if collection contains an item.
    pub fn contains(&self, item: &T) -> bool {
        self.items.contains(item)
    }
}

/// Implement PartialEq for comparing Collection with Vec
impl<T: PartialEq> PartialEq<Vec<T>> for Collection<T> {
    fn eq(&self, other: &Vec<T>) -> bool {
        self.items == *other
    }
}

impl<T: PartialEq + Clone> Collection<T> {
    /// Remove duplicates.
    pub fn unique(mut self) -> Self {
        let mut seen = Vec::new();
        self.items.retain(|item| {
            if seen.contains(item) {
                false
            } else {
                seen.push(item.clone());
                true
            }
        });
        self
    }
}

impl<T: Clone> Collection<T>
where
    T: PartialEq,
{
    /// Filter where field equals value.
    pub fn where_eq<F, U>(self, f: F, value: U) -> Self
    where
        F: Fn(&T) -> U,
        U: PartialEq,
    {
        self.filter(|item| f(item) == value)
    }

    /// Filter where field is in values.
    pub fn where_in<F, U>(self, f: F, values: &[U]) -> Self
    where
        F: Fn(&T) -> U,
        U: PartialEq,
    {
        self.filter(|item| values.contains(&f(item)))
    }
}

impl<T: Ord> Collection<T> {
    /// Sort the collection.
    pub fn sort(mut self) -> Self {
        self.items.sort();
        self
    }
}

impl<T> Collection<T> {
    /// Sort by a key function.
    pub fn sort_by<F, U>(mut self, mut f: F) -> Self
    where
        F: FnMut(&T) -> U,
        U: Ord,
    {
        self.items.sort_by_key(|item| f(item));
        self
    }

    /// Group by a key function.
    pub fn group_by<F, K>(self, mut f: F) -> HashMap<K, Vec<T>>
    where
        F: FnMut(&T) -> K,
        K: Eq + Hash,
    {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();
        for item in self.items {
            let key = f(&item);
            groups.entry(key).or_default().push(item);
        }
        groups
    }

    // ========== NEW METHODS (30+) ==========

    /// Flatten nested collections into a single level
    pub fn flatten<U>(self) -> Collection<U>
    where
        T: IntoIterator<Item = U>,
    {
        Collection::new(self.items.into_iter().flatten().collect())
    }

    /// Collapse a collection of collections
    pub fn collapse<U>(self) -> Collection<U>
    where
        T: IntoIterator<Item = U>,
    {
        self.flatten()
    }

    /// Get only the specified indices
    pub fn only(self, indices: &[usize]) -> Collection<T>
    where
        T: Clone,
    {
        Collection::new(
            indices
                .iter()
                .filter_map(|&i| self.items.get(i).cloned())
                .collect(),
        )
    }

    /// Get all except the specified indices
    pub fn except(self, indices: &[usize]) -> Collection<T> {
        Collection::new(
            self.items
                .into_iter()
                .enumerate()
                .filter_map(|(i, item)| {
                    if indices.contains(&i) {
                        None
                    } else {
                        Some(item)
                    }
                })
                .collect(),
        )
    }

    /// Prepend an item to the collection
    pub fn prepend(mut self, item: T) -> Self {
        self.items.insert(0, item);
        self
    }

    /// Push an item to the collection
    pub fn push(mut self, item: T) -> Self {
        self.items.push(item);
        self
    }

    /// Remove and return the last item
    pub fn pop(mut self) -> (Self, Option<T>) {
        let last = self.items.pop();
        (self, last)
    }

    /// Remove and return the first item
    pub fn shift(self) -> (Self, Option<T>)
    where
        T: Clone,
    {
        if self.items.is_empty() {
            return (self, None);
        }
        let first = self.items[0].clone();
        let rest = self.items[1..].to_vec();
        (Collection::new(rest), Some(first))
    }

    /// Get a slice of the collection
    pub fn slice(self, offset: usize, length: Option<usize>) -> Self
    where
        T: Clone,
    {
        let offset = offset.min(self.items.len());
        let end = length
            .map(|l| (offset + l).min(self.items.len()))
            .unwrap_or(self.items.len());
        Collection::new(self.items[offset..end].to_vec())
    }

    /// Partition the collection by a predicate
    pub fn partition<F>(self, mut f: F) -> (Collection<T>, Collection<T>)
    where
        F: FnMut(&T) -> bool,
    {
        let (matched, unmatched): (Vec<T>, Vec<T>) =
            self.items.into_iter().partition(|item| f(item));
        (Collection::new(matched), Collection::new(unmatched))
    }

    /// Zip with another collection or vector
    pub fn zip<U, I>(self, other: I) -> Collection<(T, U)>
    where
        I: IntoIterator<Item = U>,
    {
        Collection::new(self.items.into_iter().zip(other).collect())
    }

    /// Key the collection by a function
    pub fn key_by<F, K>(self, mut f: F) -> HashMap<K, T>
    where
        F: FnMut(&T) -> K,
        K: Eq + Hash,
    {
        self.items
            .into_iter()
            .map(|item| {
                let key = f(&item);
                (key, item)
            })
            .collect()
    }

    /// Map with keys
    pub fn map_with_keys<F, K, V>(self, f: F) -> HashMap<K, V>
    where
        F: FnMut(T) -> (K, V),
        K: Eq + Hash,
    {
        self.items.into_iter().map(f).collect()
    }

    /// Map to groups
    pub fn map_to_groups<F, K>(self, mut f: F) -> HashMap<K, Vec<T>>
    where
        F: FnMut(&T) -> K,
        K: Eq + Hash,
        T: Clone,
    {
        let mut groups: HashMap<K, Vec<T>> = HashMap::new();
        for item in self.items {
            let key = f(&item);
            groups.entry(key).or_default().push(item);
        }
        groups
    }

    /// Implode collection into a string
    pub fn implode(self, glue: &str) -> String
    where
        T: std::fmt::Display,
    {
        self.items
            .into_iter()
            .map(|item| item.to_string())
            .collect::<Vec<_>>()
            .join(glue)
    }

    /// Join collection into a string
    pub fn join(self, separator: &str) -> String
    where
        T: std::fmt::Display,
    {
        self.implode(separator)
    }

    /// Reject items that match predicate
    pub fn reject<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Collection::new(self.items.into_iter().filter(|item| !f(item)).collect())
    }

    /// Tap into the collection (for debugging)
    pub fn tap<F>(self, mut f: F) -> Self
    where
        F: FnMut(&Self),
    {
        f(&self);
        self
    }

    /// Pipe the collection through a function
    pub fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(Self) -> U,
    {
        f(self)
    }

    /// Dump and die (debug helper)
    pub fn dd(self) -> !
    where
        T: std::fmt::Debug,
    {
        eprintln!("{:#?}", self.items);
        std::process::exit(1);
    }

    /// Dump (debug helper)
    pub fn dump(self) -> Self
    where
        T: std::fmt::Debug,
    {
        eprintln!("{:#?}", self.items);
        self
    }

    /// Conditionally apply a function
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Conditionally apply a function (inverse)
    pub fn unless<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        self.when(!condition, f)
    }

    /// Get items for a specific page
    pub fn for_page(self, page: usize, per_page: usize) -> Self
    where
        T: Clone,
    {
        let offset = page.saturating_sub(1) * per_page;
        self.slice(offset, Some(per_page))
    }

    /// Get the sum of all items
    pub fn sum(self) -> T
    where
        T: std::iter::Sum,
    {
        self.items.into_iter().sum()
    }

    /// Get the average of all items
    pub fn avg(self) -> Option<f64>
    where
        T: Into<f64> + Copy,
    {
        if self.items.is_empty() {
            return None;
        }
        let sum: f64 = self.items.iter().map(|&x| x.into()).sum();
        Some(sum / self.items.len() as f64)
    }

    /// Get the minimum value
    pub fn min(self) -> Option<T>
    where
        T: Ord,
    {
        self.items.into_iter().min()
    }

    /// Get the maximum value
    pub fn max(self) -> Option<T>
    where
        T: Ord,
    {
        self.items.into_iter().max()
    }

    /// Get the median value
    pub fn median(mut self) -> Option<f64>
    where
        T: Into<f64> + Copy + Ord,
    {
        if self.items.is_empty() {
            return None;
        }
        self.items.sort();
        let mid = self.items.len() / 2;
        if self.items.len().is_multiple_of(2) {
            Some((self.items[mid - 1].into() + self.items[mid].into()) / 2.0)
        } else {
            Some(self.items[mid].into())
        }
    }

    /// Get the mode (most frequent value)
    pub fn mode(self) -> Option<T>
    where
        T: Eq + Hash + Clone,
    {
        let mut counts: HashMap<T, usize> = HashMap::new();
        for item in &self.items {
            *counts.entry(item.clone()).or_insert(0) += 1;
        }
        counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .map(|(item, _)| item)
    }

    /// Count occurrences by a key function
    pub fn count_by<F, K>(self, mut f: F) -> HashMap<K, usize>
    where
        F: FnMut(&T) -> K,
        K: Eq + Hash,
    {
        let mut counts = HashMap::new();
        for item in &self.items {
            let key = f(item);
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    /// Get nth item
    pub fn nth(&self, n: usize) -> Option<&T> {
        self.items.get(n)
    }

    /// Split collection into chunks
    pub fn split(self, size: usize) -> Vec<Collection<T>>
    where
        T: Clone,
    {
        self.chunk(size)
    }

    /// Sliding window over collection
    pub fn sliding<F>(self, size: usize, mut f: F)
    where
        F: FnMut(&[T]),
    {
        let size = size.max(1);
        for window in self.items.windows(size) {
            f(window);
        }
    }

    /// Skip until predicate is true
    pub fn skip_until<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        let mut found = false;
        Collection::new(
            self.items
                .into_iter()
                .skip_while(|item| {
                    if found {
                        false
                    } else if f(item) {
                        found = true;
                        false
                    } else {
                        true
                    }
                })
                .collect(),
        )
    }

    /// Skip while predicate is true
    pub fn skip_while<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Collection::new(self.items.into_iter().skip_while(|item| f(item)).collect())
    }

    /// Take until predicate is true
    pub fn take_until<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Collection::new(self.items.into_iter().take_while(|item| !f(item)).collect())
    }

    /// Take while predicate is true
    pub fn take_while<F>(self, mut f: F) -> Self
    where
        F: FnMut(&T) -> bool,
    {
        Collection::new(self.items.into_iter().take_while(|item| f(item)).collect())
    }

    /// Combine with another collection
    pub fn merge(mut self, other: Collection<T>) -> Self {
        self.items.extend(other.items);
        self
    }

    /// Replace items at specified indices
    pub fn splice(mut self, start: usize, length: usize, replacement: Vec<T>) -> Self {
        let start = start.min(self.items.len());
        let end = (start + length).min(self.items.len());
        self.items.splice(start..end, replacement);
        self
    }

    /// Pad collection to a length
    pub fn pad(mut self, size: usize, value: T) -> Self
    where
        T: Clone,
    {
        while self.items.len() < size {
            self.items.push(value.clone());
        }
        self
    }

    /// Get random item
    pub fn random(&self) -> Option<&T> {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.items.choose(&mut rng)
    }

    /// Shuffle the collection randomly
    pub fn shuffle(mut self) -> Self
    where
        T: Clone,
    {
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        self.items.shuffle(&mut rng);
        self
    }
}

/// Helper function to create a collection.
pub fn collect<T>(items: Vec<T>) -> Collection<T> {
    Collection::new(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
    struct TestItem {
        id: i64,
        name: String,
        active: bool,
    }

    #[test]
    fn test_collection_basic_operations() {
        let items = vec![
            TestItem {
                id: 1,
                name: "Item 1".to_string(),
                active: true,
            },
            TestItem {
                id: 2,
                name: "Item 2".to_string(),
                active: false,
            },
        ];

        let collection = collect(items);
        assert_eq!(collection.count(), 2);
        assert!(!collection.is_empty());
    }

    #[test]
    fn test_collection_map() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let doubled = collection.map(|x| x * 2).to_vec();

        assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_collection_filter() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let evens = collection.filter(|x| x % 2 == 0).to_vec();

        assert_eq!(evens, vec![2, 4]);
    }

    #[test]
    fn test_collection_take_skip() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);

        let taken = collection.clone().take(3).to_vec();
        assert_eq!(taken, vec![1, 2, 3]);

        let skipped = collection.skip(2).to_vec();
        assert_eq!(skipped, vec![3, 4, 5]);
    }

    #[test]
    fn test_collection_chunk() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let chunks = collection.chunk(2);

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].all(), &[1, 2]);
        assert_eq!(chunks[1].all(), &[3, 4]);
        assert_eq!(chunks[2].all(), &[5]);
    }

    #[test]
    fn test_collection_sort() {
        let items = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let collection = collect(items);
        let sorted: Vec<i32> = collection.sort().to_vec();

        assert_eq!(sorted, vec![1, 1, 2, 3, 4, 5, 6, 9]);
    }

    #[test]
    fn test_collection_group_by() {
        let items = vec![
            TestItem {
                id: 1,
                name: "Item 1".to_string(),
                active: true,
            },
            TestItem {
                id: 2,
                name: "Item 2".to_string(),
                active: false,
            },
            TestItem {
                id: 3,
                name: "Item 3".to_string(),
                active: true,
            },
        ];

        let collection = collect(items);
        let grouped = collection.group_by(|item| item.active);

        assert_eq!(grouped[&true].len(), 2);
        assert_eq!(grouped[&false].len(), 1);
    }

    #[test]
    fn test_collection_unique() {
        let items = vec![1, 2, 2, 3, 3, 3, 4];
        let collection = collect(items);
        let unique = collection.unique().to_vec();

        assert_eq!(unique, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_collection_reduce() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let sum = collection.reduce(0, |acc, x| acc + x);

        assert_eq!(sum, 15);
    }

    #[test]
    fn test_collection_any_all() {
        let items = vec![2, 4, 6, 8];
        let collection = collect(items);

        assert!(collection.any(|x| x % 2 == 0));
        assert!(collection.all_match(|x| x % 2 == 0));
        assert!(!collection.any(|x| *x > 10));
    }
}
