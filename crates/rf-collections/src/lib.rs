//! # rf-collections
//!
//! Laravel-style collection API for Rust.
//!
//! ## Features
//!
//! - Rich collection API (map, filter, reduce, etc.)
//! - groupBy, sortBy, whereIn, etc.
//! - Lazy collections for large datasets
//! - Higher-order methods
//! - Fluent interface
//!
//! ## Example
//!
//! ```rust
//! use rf_collections::{collect, Collection};
//!
//! let collection = collect(vec![1, 2, 3, 4, 5])
//!     .filter(|x| x % 2 == 0)
//!     .map(|x| x * 2)
//!     .take(2)
//!     .to_vec();
//!
//! assert_eq!(collection, vec![4, 8]);
//! ```

pub mod collection;
pub mod lazy;
pub mod methods;

pub use collection::{collect, Collection};
pub use lazy::{collect_lazy, LazyCollection};
pub use methods::{CollectionMethods, Pipe, Tap};

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct User {
        id: i64,
        name: String,
        active: bool,
    }

    #[test]
    fn test_integration_collection_chain() {
        let users = vec![
            User {
                id: 1,
                name: "Alice".to_string(),
                active: true,
            },
            User {
                id: 2,
                name: "Bob".to_string(),
                active: false,
            },
            User {
                id: 3,
                name: "Charlie".to_string(),
                active: true,
            },
        ];

        let names: Vec<String> = collect(users).filter(|u| u.active).map(|u| u.name).to_vec();

        assert_eq!(names, vec!["Alice".to_string(), "Charlie".to_string()]);
    }

    #[test]
    fn test_integration_lazy_collection() {
        let items = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let result: Vec<i32> = collect_lazy(items.into_iter())
            .filter(|x| x % 2 == 0)
            .map(|x| x * 2)
            .take(3)
            .collect();

        assert_eq!(result, vec![4, 8, 12]);
    }
}
