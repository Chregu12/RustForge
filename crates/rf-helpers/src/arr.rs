//! Array/Vec helper functions (Laravel Arr::* equivalents)

use rand::seq::SliceRandom;
use std::collections::HashMap;
use std::hash::Hash;

/// Get only the specified items from an array
pub fn only<T: Clone>(items: &[T], indices: &[usize]) -> Vec<T> {
    indices
        .iter()
        .filter_map(|&i| items.get(i).cloned())
        .collect()
}

/// Get all items except the specified indices
pub fn except<T: Clone>(items: &[T], indices: &[usize]) -> Vec<T> {
    items
        .iter()
        .enumerate()
        .filter_map(|(i, item)| {
            if indices.contains(&i) {
                None
            } else {
                Some(item.clone())
            }
        })
        .collect()
}

/// Flatten a multi-dimensional array into a single level
pub fn flatten<T: Clone>(items: Vec<Vec<T>>) -> Vec<T> {
    items.into_iter().flatten().collect()
}

/// Collapse an array of arrays into a single array
pub fn collapse<T: Clone>(items: Vec<Vec<T>>) -> Vec<T> {
    flatten(items)
}

/// Divide an array into two arrays - keys and values
pub fn divide<T: Clone>(items: Vec<T>) -> (Vec<usize>, Vec<T>) {
    let keys: Vec<usize> = (0..items.len()).collect();
    (keys, items)
}

/// Get the first element that matches the predicate
pub fn first<T, F>(items: &[T], mut predicate: F) -> Option<&T>
where
    F: FnMut(&T) -> bool,
{
    items.iter().find(|item| predicate(item))
}

/// Get the last element that matches the predicate
pub fn last<T, F>(items: &[T], mut predicate: F) -> Option<&T>
where
    F: FnMut(&T) -> bool,
{
    items.iter().rev().find(|item| predicate(item))
}

/// Get a random element from the array
pub fn random<T>(items: &[T]) -> Option<&T> {
    let mut rng = rand::thread_rng();
    items.choose(&mut rng)
}

/// Get multiple random elements from the array
pub fn random_multiple<T: Clone>(items: &[T], count: usize) -> Vec<T> {
    let mut rng = rand::thread_rng();
    items
        .choose_multiple(&mut rng, count)
        .cloned()
        .collect()
}

/// Shuffle the array randomly
pub fn shuffle<T: Clone>(mut items: Vec<T>) -> Vec<T> {
    let mut rng = rand::thread_rng();
    items.shuffle(&mut rng);
    items
}

/// Get a subset of items starting at an offset
pub fn slice<T: Clone>(items: &[T], offset: usize, length: Option<usize>) -> Vec<T> {
    let end = length.map(|l| (offset + l).min(items.len())).unwrap_or(items.len());
    items[offset..end].to_vec()
}

/// Prepend a value to the array
pub fn prepend<T>(mut items: Vec<T>, value: T) -> Vec<T> {
    items.insert(0, value);
    items
}

/// Push a value to the end of the array
pub fn push<T>(mut items: Vec<T>, value: T) -> Vec<T> {
    items.push(value);
    items
}

/// Remove and return the last element
pub fn pop<T>(mut items: Vec<T>) -> (Vec<T>, Option<T>) {
    let last = items.pop();
    (items, last)
}

/// Remove and return the first element
pub fn shift<T: Clone>(items: Vec<T>) -> (Vec<T>, Option<T>) {
    if items.is_empty() {
        return (items, None);
    }
    let first = items[0].clone();
    let rest = items[1..].to_vec();
    (rest, Some(first))
}

/// Check if array contains a value
pub fn contains<T: PartialEq>(items: &[T], value: &T) -> bool {
    items.contains(value)
}

/// Check if any element matches the predicate
pub fn any<T, F>(items: &[T], mut predicate: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    items.iter().any(|item| predicate(item))
}

/// Check if all elements match the predicate
pub fn all<T, F>(items: &[T], mut predicate: F) -> bool
where
    F: FnMut(&T) -> bool,
{
    items.iter().all(|item| predicate(item))
}

/// Create a HashMap from keys and values arrays
pub fn combine<K: Eq + Hash, V>(keys: Vec<K>, values: Vec<V>) -> HashMap<K, V> {
    keys.into_iter().zip(values).collect()
}

/// Count occurrences of each element
pub fn count_by<T, K, F>(items: Vec<T>, mut key_fn: F) -> HashMap<K, usize>
where
    K: Eq + Hash,
    F: FnMut(&T) -> K,
{
    let mut counts = HashMap::new();
    for item in items.iter() {
        let key = key_fn(item);
        *counts.entry(key).or_insert(0) += 1;
    }
    counts
}

/// Group elements by a key function
pub fn group_by<T, K, F>(items: Vec<T>, mut key_fn: F) -> HashMap<K, Vec<T>>
where
    K: Eq + Hash,
    F: FnMut(&T) -> K,
    T: Clone,
{
    let mut groups: HashMap<K, Vec<T>> = HashMap::new();
    for item in items {
        let key = key_fn(&item);
        groups.entry(key).or_insert_with(Vec::new).push(item);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(only(&items, &[0, 2, 4]), vec![1, 3, 5]);
    }

    #[test]
    fn test_except() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(except(&items, &[0, 2, 4]), vec![2, 4]);
    }

    #[test]
    fn test_flatten() {
        let nested = vec![vec![1, 2], vec![3, 4], vec![5]];
        assert_eq!(flatten(nested), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_first() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(first(&items, |&x| x > 2), Some(&3));
        assert_eq!(first(&items, |&x| x > 10), None);
    }

    #[test]
    fn test_last() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(last(&items, |&x| x > 2), Some(&5));
        assert_eq!(last(&items, |&x| x > 10), None);
    }

    #[test]
    fn test_random() {
        let items = vec![1, 2, 3, 4, 5];
        let result = random(&items);
        assert!(result.is_some());
        assert!(items.contains(result.unwrap()));
    }

    #[test]
    fn test_shuffle() {
        let items = vec![1, 2, 3, 4, 5];
        let shuffled = shuffle(items.clone());
        assert_eq!(shuffled.len(), items.len());
        // All original items should be present
        for item in &items {
            assert!(shuffled.contains(item));
        }
    }

    #[test]
    fn test_slice() {
        let items = vec![1, 2, 3, 4, 5];
        assert_eq!(slice(&items, 1, Some(3)), vec![2, 3, 4]);
        assert_eq!(slice(&items, 2, None), vec![3, 4, 5]);
    }

    #[test]
    fn test_prepend() {
        let items = vec![2, 3, 4];
        assert_eq!(prepend(items, 1), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_push() {
        let items = vec![1, 2, 3];
        assert_eq!(push(items, 4), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_pop() {
        let items = vec![1, 2, 3];
        let (remaining, popped) = pop(items);
        assert_eq!(remaining, vec![1, 2]);
        assert_eq!(popped, Some(3));
    }

    #[test]
    fn test_shift() {
        let items = vec![1, 2, 3];
        let (remaining, shifted) = shift(items);
        assert_eq!(remaining, vec![2, 3]);
        assert_eq!(shifted, Some(1));
    }

    #[test]
    fn test_any() {
        let items = vec![1, 2, 3, 4, 5];
        assert!(any(&items, |&x| x > 3));
        assert!(!any(&items, |&x| x > 10));
    }

    #[test]
    fn test_all() {
        let items = vec![1, 2, 3, 4, 5];
        assert!(all(&items, |&x| x > 0));
        assert!(!all(&items, |&x| x > 2));
    }

    #[test]
    fn test_combine() {
        let keys = vec!["a", "b", "c"];
        let values = vec![1, 2, 3];
        let result = combine(keys, values);
        assert_eq!(result.get("a"), Some(&1));
        assert_eq!(result.get("b"), Some(&2));
        assert_eq!(result.get("c"), Some(&3));
    }

    #[test]
    fn test_count_by() {
        let items = vec![1, 2, 2, 3, 3, 3];
        let counts = count_by(items, |&x| x);
        assert_eq!(counts.get(&1), Some(&1));
        assert_eq!(counts.get(&2), Some(&2));
        assert_eq!(counts.get(&3), Some(&3));
    }

    #[test]
    fn test_group_by() {
        let items = vec![1, 2, 3, 4, 5, 6];
        let groups = group_by(items, |&x| x % 2);
        assert_eq!(groups.get(&0), Some(&vec![2, 4, 6]));
        assert_eq!(groups.get(&1), Some(&vec![1, 3, 5]));
    }
}
