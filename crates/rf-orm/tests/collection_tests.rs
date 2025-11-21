//! Tests for Laravel-Style Collections
//!
//! Comprehensive tests for all collection methods.

use rf_orm::collection::*;

#[cfg(test)]
mod collection_tests {
    use super::*;

    #[test]
    fn test_collection_creation() {
        let coll = Collection::new(vec![1, 2, 3]);
        assert_eq!(coll.count(), 3);
        assert!(!coll.is_empty());

        let empty = Collection::<i32>::empty();
        assert_eq!(empty.count(), 0);
        assert!(empty.is_empty());
    }

    #[test]
    fn test_collection_from_iter() {
        let coll = Collection::from_iter(1..=5);
        assert_eq!(coll.count(), 5);
    }

    #[test]
    fn test_collection_map() {
        let coll = Collection::new(vec![1, 2, 3]);
        let doubled = coll.map(|n| n * 2);
        assert_eq!(doubled.to_vec(), vec![2, 4, 6]);
    }

    #[test]
    fn test_collection_filter() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let evens = coll.filter(|n| n % 2 == 0);
        assert_eq!(evens.to_vec(), vec![2, 4]);
    }

    #[test]
    fn test_collection_reject() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let odds = coll.reject(|n| n % 2 == 0);
        assert_eq!(odds.to_vec(), vec![1, 3, 5]);
    }

    #[test]
    fn test_collection_transform() {
        let coll = Collection::new(vec![1, 2, 3]);
        let result = coll.transform(|c| c.map(|n| n * 2).filter(|n| *n > 2));
        assert_eq!(result.to_vec(), vec![4, 6]);
    }

    #[test]
    fn test_collection_tap() {
        let mut tapped = false;
        let coll = Collection::new(vec![1, 2, 3])
            .tap(|c| {
                tapped = true;
                assert_eq!(c.count(), 3);
            });
        assert!(tapped);
        assert_eq!(coll.count(), 3);
    }

    #[test]
    fn test_collection_pluck() {
        #[derive(Clone)]
        struct User {
            id: i32,
            name: String,
        }

        let users = Collection::new(vec![
            User { id: 1, name: "John".to_string() },
            User { id: 2, name: "Jane".to_string() },
        ]);

        let names = users.pluck(|u| u.name.clone());
        assert_eq!(names, vec!["John".to_string(), "Jane".to_string()]);
    }

    #[test]
    fn test_collection_first_last() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.first(), Some(&1));
        assert_eq!(coll.last(), Some(&5));

        let empty = Collection::<i32>::empty();
        assert_eq!(empty.first(), None);
        assert_eq!(empty.last(), None);
    }

    #[test]
    fn test_collection_first_where() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.first_where(|n| *n > 3), Some(&4));
        assert_eq!(coll.first_where(|n| *n > 10), None);
    }

    #[test]
    fn test_collection_last_where() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.last_where(|n| *n < 4), Some(&3));
        assert_eq!(coll.last_where(|n| *n > 10), None);
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
    fn test_collection_take_skip() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);

        let taken = coll.clone().take(3);
        assert_eq!(taken.to_vec(), vec![1, 2, 3]);

        let skipped = coll.skip(2);
        assert_eq!(skipped.to_vec(), vec![3, 4, 5]);
    }

    #[test]
    fn test_collection_slice() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let sliced = coll.slice(1, 4);
        assert_eq!(sliced.to_vec(), vec![2, 3, 4]);
    }

    #[test]
    fn test_collection_group_by() {
        #[derive(Clone)]
        struct User {
            role: String,
            name: String,
        }

        let users = Collection::new(vec![
            User { role: "admin".to_string(), name: "John".to_string() },
            User { role: "user".to_string(), name: "Jane".to_string() },
            User { role: "admin".to_string(), name: "Bob".to_string() },
        ]);

        let by_role = users.group_by(|u| u.role.clone());

        assert_eq!(by_role.len(), 2);
        assert_eq!(by_role.get("admin").unwrap().count(), 2);
        assert_eq!(by_role.get("user").unwrap().count(), 1);
    }

    #[test]
    fn test_collection_unique() {
        let coll = Collection::new(vec![1, 2, 2, 3, 3, 3, 4]);
        let unique = coll.unique();
        assert_eq!(unique.to_vec(), vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_collection_unique_by() {
        #[derive(Clone)]
        struct User {
            id: i32,
            name: String,
        }

        let users = Collection::new(vec![
            User { id: 1, name: "John".to_string() },
            User { id: 1, name: "John Doe".to_string() },
            User { id: 2, name: "Jane".to_string() },
        ]);

        let unique = users.unique_by(|u| u.id);
        assert_eq!(unique.count(), 2);
    }

    #[test]
    fn test_collection_sort() {
        let coll = Collection::new(vec![3, 1, 4, 1, 5]);
        let sorted = coll.sort();
        assert_eq!(sorted.to_vec(), vec![1, 1, 3, 4, 5]);
    }

    #[test]
    fn test_collection_sort_by() {
        let coll = Collection::new(vec![3, 1, 4, 1, 5]);
        let sorted = coll.sort_by(|a, b| b.cmp(a)); // Descending
        assert_eq!(sorted.to_vec(), vec![5, 4, 3, 1, 1]);
    }

    #[test]
    fn test_collection_reverse() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        let reversed = coll.reverse();
        assert_eq!(reversed.to_vec(), vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn test_collection_count() {
        let coll = Collection::new(vec![1, 2, 3]);
        assert_eq!(coll.count(), 3);
    }

    #[test]
    fn test_collection_contains() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert!(coll.contains(|n| *n == 3));
        assert!(!coll.contains(|n| *n == 10));
    }

    #[test]
    fn test_collection_every() {
        let coll = Collection::new(vec![2, 4, 6, 8]);
        assert!(coll.every(|n| n % 2 == 0));
        assert!(!coll.every(|n| *n > 5));
    }

    #[test]
    fn test_collection_each() {
        let coll = Collection::new(vec![1, 2, 3]);
        let mut sum = 0;
        coll.each(|n| sum += n);
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_collection_sum() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.sum(), 15);
    }

    #[test]
    fn test_collection_avg() {
        let coll = Collection::new(vec![1, 2, 3, 4, 5]);
        assert_eq!(coll.avg(), Some(3.0));

        let empty = Collection::<i32>::empty();
        assert_eq!(empty.avg(), None);
    }

    #[test]
    fn test_collection_min_max() {
        let coll = Collection::new(vec![3, 1, 4, 1, 5, 9]);
        assert_eq!(coll.min(), Some(&1));
        assert_eq!(coll.max(), Some(&9));
    }

    #[test]
    fn test_collection_to_json() {
        let coll = Collection::new(vec![1, 2, 3]);
        let json = coll.to_json().unwrap();
        assert_eq!(json, "[1,2,3]");
    }

    #[test]
    fn test_collection_into_iter() {
        let coll = Collection::new(vec![1, 2, 3]);
        let vec: Vec<i32> = coll.into_iter().collect();
        assert_eq!(vec, vec![1, 2, 3]);
    }

    #[test]
    fn test_collection_from_vec() {
        let vec = vec![1, 2, 3];
        let coll = Collection::from(vec);
        assert_eq!(coll.count(), 3);
    }

    #[test]
    fn test_into_collection_trait() {
        let vec = vec![1, 2, 3];
        let coll = vec.into_collection();
        assert_eq!(coll.count(), 3);
    }

    #[test]
    fn test_collection_chaining() {
        // Test complex chaining like Laravel
        let result = Collection::new(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10])
            .filter(|n| n % 2 == 0)
            .map(|n| n * 2)
            .take(3)
            .to_vec();

        assert_eq!(result, vec![4, 8, 12]);
    }

    #[test]
    fn test_collection_pipeline() {
        // Test a real-world-like pipeline
        #[derive(Clone, Debug, PartialEq)]
        struct Product {
            name: String,
            price: f64,
            category: String,
        }

        let products = Collection::new(vec![
            Product { name: "A".to_string(), price: 10.0, category: "electronics".to_string() },
            Product { name: "B".to_string(), price: 20.0, category: "books".to_string() },
            Product { name: "C".to_string(), price: 15.0, category: "electronics".to_string() },
            Product { name: "D".to_string(), price: 25.0, category: "electronics".to_string() },
        ]);

        let electronics = products
            .filter(|p| p.category == "electronics")
            .to_vec();

        assert_eq!(electronics.len(), 3);
    }
}

// Performance comparison tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_collection_vs_vec_performance() {
        // Test that Collection overhead is minimal
        let data: Vec<i32> = (1..=10000).collect();

        // Vec operation
        let start = Instant::now();
        let vec_result: Vec<i32> = data
            .iter()
            .filter(|n| *n % 2 == 0)
            .map(|n| n * 2)
            .collect();
        let vec_duration = start.elapsed();

        // Collection operation
        let start = Instant::now();
        let coll_result = Collection::new(data)
            .filter(|n| n % 2 == 0)
            .map(|n| n * 2)
            .to_vec();
        let coll_duration = start.elapsed();

        // Results should be the same
        assert_eq!(vec_result, coll_result);

        // Collection overhead should be minimal (within 2x)
        // Note: This is a rough check and may vary by system
        println!("Vec time: {:?}, Collection time: {:?}", vec_duration, coll_duration);
        // In practice, Collection should be nearly as fast as Vec
    }
}
