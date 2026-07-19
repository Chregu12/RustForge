//! Higher-order collection methods.

use crate::Collection;

/// Extension trait for higher-order collection methods.
pub trait CollectionMethods<T>: Sized {
    /// Flat map over the collection.
    fn flat_map<U, F>(self, f: F) -> Collection<U>
    where
        F: FnMut(T) -> Vec<U>;

    /// Partition the collection into two based on a predicate.
    fn partition<F>(self, f: F) -> (Vec<T>, Vec<T>)
    where
        F: FnMut(&T) -> bool;

    /// Zip this collection with another.
    fn zip<U>(self, other: Vec<U>) -> Collection<(T, U)>;

    /// Sum all items (requires T: Into<i64>).
    fn sum_i64(self) -> i64
    where
        T: Into<i64>;

    /// Get the average of all items.
    fn avg(self) -> f64
    where
        T: Into<f64> + Clone;

    /// Get the minimum value.
    fn min(self) -> Option<T>
    where
        T: Ord;

    /// Get the maximum value.
    fn max(self) -> Option<T>
    where
        T: Ord;
}

impl<T> CollectionMethods<T> for Collection<T> {
    fn flat_map<U, F>(self, f: F) -> Collection<U>
    where
        F: FnMut(T) -> Vec<U>,
    {
        Collection::new(self.to_vec().into_iter().flat_map(f).collect())
    }

    fn partition<F>(self, mut f: F) -> (Vec<T>, Vec<T>)
    where
        F: FnMut(&T) -> bool,
    {
        let mut left = Vec::new();
        let mut right = Vec::new();

        for item in self.to_vec() {
            if f(&item) {
                left.push(item);
            } else {
                right.push(item);
            }
        }

        (left, right)
    }

    fn zip<U>(self, other: Vec<U>) -> Collection<(T, U)> {
        Collection::new(self.to_vec().into_iter().zip(other).collect())
    }

    fn sum_i64(self) -> i64
    where
        T: Into<i64>,
    {
        self.to_vec().into_iter().map(|x| x.into()).sum()
    }

    fn avg(self) -> f64
    where
        T: Into<f64> + Clone,
    {
        let items = self.all();
        if items.is_empty() {
            return 0.0;
        }

        let sum: f64 = items.iter().cloned().map(|x| x.into()).sum();
        sum / items.len() as f64
    }

    fn min(self) -> Option<T>
    where
        T: Ord,
    {
        self.to_vec().into_iter().min()
    }

    fn max(self) -> Option<T>
    where
        T: Ord,
    {
        self.to_vec().into_iter().max()
    }
}

/// Pipe operation for chaining transformations.
pub trait Pipe: Sized {
    /// Pipe the value through a function.
    fn pipe<F, U>(self, f: F) -> U
    where
        F: FnOnce(Self) -> U,
    {
        f(self)
    }
}

impl<T> Pipe for Collection<T> {}

/// Tap operation for side effects.
pub trait Tap: Sized {
    /// Tap into the collection for side effects.
    fn tap<F>(self, f: F) -> Self
    where
        F: FnOnce(&Self),
    {
        f(&self);
        self
    }
}

impl<T> Tap for Collection<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect;

    #[test]
    fn test_flat_map() {
        let items = vec![vec![1, 2], vec![3, 4], vec![5]];
        let collection = collect(items);
        let flattened = collection.flat_map(|v| v).to_vec();

        assert_eq!(flattened, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_partition() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let (evens, odds) = collection.partition(|x| x % 2 == 0);

        assert_eq!(evens, vec![2, 4]);
        assert_eq!(odds, vec![1, 3, 5]);
    }

    #[test]
    fn test_zip() {
        let items1 = vec![1, 2, 3];
        let items2 = vec!["a", "b", "c"];
        let collection = collect(items1);
        let zipped = collection.zip(items2).to_vec();

        assert_eq!(zipped, vec![(1, "a"), (2, "b"), (3, "c")]);
    }

    #[test]
    fn test_sum() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let sum = collection.sum_i64();

        assert_eq!(sum, 15);
    }

    #[test]
    fn test_avg() {
        let items = vec![1, 2, 3, 4, 5];
        let collection = collect(items);
        let avg = collection.avg();

        assert_eq!(avg, Some(3.0));
    }

    #[test]
    fn test_min_max() {
        let items = vec![3, 1, 4, 1, 5, 9, 2, 6];
        let collection = collect(items.clone());
        let min = collection.min();

        let collection = collect(items);
        let max = collection.max();

        assert_eq!(min, Some(1));
        assert_eq!(max, Some(9));
    }

    #[test]
    fn test_pipe() {
        let items = vec![1, 2, 3, 4, 5];
        let result = collect(items)
            .pipe(|c| c.filter(|x| x % 2 == 0))
            .pipe(|c| c.map(|x| x * 2))
            .to_vec();

        assert_eq!(result, vec![4, 8]);
    }

    #[test]
    fn test_tap() {
        let items = vec![1, 2, 3];
        let mut tapped_count = 0;

        let result = collect(items)
            .tap(|c| {
                tapped_count = c.count();
            })
            .to_vec();

        assert_eq!(result, vec![1, 2, 3]);
        assert_eq!(tapped_count, 3);
    }
}
