//! Lazy collections for efficient processing of large datasets.

/// A lazy collection that processes items on-demand.
pub struct LazyCollection<T, I>
where
    I: Iterator<Item = T>,
{
    iterator: I,
}

impl<T, I> LazyCollection<T, I>
where
    I: Iterator<Item = T>,
{
    /// Create a new lazy collection.
    pub fn new(iterator: I) -> Self {
        Self { iterator }
    }

    /// Map over the collection lazily.
    pub fn map<U, F>(self, f: F) -> LazyCollection<U, std::iter::Map<I, F>>
    where
        F: FnMut(T) -> U,
    {
        LazyCollection::new(self.iterator.map(f))
    }

    /// Filter the collection lazily.
    pub fn filter<F>(self, f: F) -> LazyCollection<T, std::iter::Filter<I, F>>
    where
        F: FnMut(&T) -> bool,
    {
        LazyCollection::new(self.iterator.filter(f))
    }

    /// Take the first n items lazily.
    pub fn take(self, n: usize) -> LazyCollection<T, std::iter::Take<I>> {
        LazyCollection::new(self.iterator.take(n))
    }

    /// Skip the first n items lazily.
    pub fn skip(self, n: usize) -> LazyCollection<T, std::iter::Skip<I>> {
        LazyCollection::new(self.iterator.skip(n))
    }

    /// Execute a closure on each item.
    pub fn each<F>(self, f: F) -> LazyCollection<T, std::iter::Inspect<I, F>>
    where
        F: FnMut(&T),
    {
        LazyCollection::new(self.iterator.inspect(f))
    }

    /// Collect into a vector.
    pub fn collect(self) -> Vec<T> {
        self.iterator.collect()
    }

    /// Count the items.
    pub fn count(self) -> usize {
        self.iterator.count()
    }

    /// Find the first item matching the predicate.
    pub fn find<F>(mut self, mut f: F) -> Option<T>
    where
        F: FnMut(&T) -> bool,
    {
        self.iterator.find(|item| f(item))
    }

    /// Check if any item satisfies the predicate.
    pub fn any<F>(mut self, mut f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        self.iterator.any(|item| f(&item))
    }

    /// Check if all items satisfy the predicate.
    pub fn all<F>(mut self, mut f: F) -> bool
    where
        F: FnMut(&T) -> bool,
    {
        self.iterator.all(|item| f(&item))
    }

    /// Reduce the collection to a single value.
    pub fn reduce<F>(self, mut f: F) -> Option<T>
    where
        F: FnMut(T, T) -> T,
    {
        self.iterator.reduce(|acc, item| f(acc, item))
    }
}

impl<T, I> LazyCollection<T, I>
where
    I: Iterator<Item = T>,
    T: Clone,
{
    /// Chunk the collection into smaller collections.
    pub fn chunk(self, size: usize) -> ChunkedIterator<T, I> {
        ChunkedIterator {
            iterator: self.iterator,
            size,
        }
    }
}

/// Iterator for chunked lazy collections.
pub struct ChunkedIterator<T, I>
where
    I: Iterator<Item = T>,
{
    iterator: I,
    size: usize,
}

impl<T, I> Iterator for ChunkedIterator<T, I>
where
    I: Iterator<Item = T>,
{
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut chunk = Vec::with_capacity(self.size);

        for _ in 0..self.size {
            if let Some(item) = self.iterator.next() {
                chunk.push(item);
            } else {
                break;
            }
        }

        if chunk.is_empty() {
            None
        } else {
            Some(chunk)
        }
    }
}

/// Helper function to create a lazy collection.
pub fn collect_lazy<T, I>(iterator: I) -> LazyCollection<T, I>
where
    I: Iterator<Item = T>,
{
    LazyCollection::new(iterator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lazy_collection_map() {
        let items = vec![1, 2, 3, 4, 5];
        let lazy = collect_lazy(items.into_iter());
        let doubled: Vec<i32> = lazy.map(|x| x * 2).collect();

        assert_eq!(doubled, vec![2, 4, 6, 8, 10]);
    }

    #[test]
    fn test_lazy_collection_filter() {
        let items = vec![1, 2, 3, 4, 5];
        let lazy = collect_lazy(items.into_iter());
        let evens: Vec<i32> = lazy.filter(|x| x % 2 == 0).collect();

        assert_eq!(evens, vec![2, 4]);
    }

    #[test]
    fn test_lazy_collection_take_skip() {
        let items = vec![1, 2, 3, 4, 5];
        let lazy = collect_lazy(items.into_iter());
        let taken: Vec<i32> = lazy.take(3).collect();

        assert_eq!(taken, vec![1, 2, 3]);
    }

    #[test]
    fn test_lazy_collection_chunk() {
        let items = vec![1, 2, 3, 4, 5];
        let lazy = collect_lazy(items.into_iter());
        let chunks: Vec<Vec<i32>> = lazy.chunk(2).collect();

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0], vec![1, 2]);
        assert_eq!(chunks[1], vec![3, 4]);
        assert_eq!(chunks[2], vec![5]);
    }

    #[test]
    fn test_lazy_collection_count() {
        let items = vec![1, 2, 3, 4, 5];
        let lazy = collect_lazy(items.into_iter());
        let count = lazy.filter(|x| x % 2 == 0).count();

        assert_eq!(count, 2);
    }

    #[test]
    fn test_lazy_collection_find() {
        let items = vec![1, 2, 3, 4, 5];
        let lazy = collect_lazy(items.into_iter());
        let found = lazy.find(|x| *x > 3);

        assert_eq!(found, Some(4));
    }
}
