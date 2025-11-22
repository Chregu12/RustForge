//! # rf-api-resources
//!
//! Laravel-style API resource transformers for Rust.
//!
//! ## Features
//!
//! - Resource transformation with conditional attributes
//! - Resource collections with pagination
//! - Nested resources
//! - Metadata support
//! - Custom wrapping
//!
//! ## Example
//!
//! ```rust
//! use rf_api_resources::{Resource, Collection, PaginationMeta};
//! use serde::Serialize;
//!
//! #[derive(Debug, Clone, Serialize)]
//! struct UserResource {
//!     id: i64,
//!     name: String,
//!     email: String,
//! }
//!
//! impl Resource for UserResource {}
//!
//! // Single resource
//! let user = UserResource {
//!     id: 1,
//!     name: "John Doe".to_string(),
//!     email: "john@example.com".to_string(),
//! };
//! let json = user.to_json().unwrap();
//!
//! // Collection
//! let users = vec![user];
//! let collection = Collection::new(users);
//! ```

pub mod collection;
pub mod conditional;
pub mod nested;
pub mod resource;
pub mod resource_builder;

pub use collection::{
    Collection, PaginatedCollection, PaginationLinks, PaginationMeta, ResourceCollection,
};
pub use conditional::{Conditional, LoadRelations, MergeWhen, WithRelation};
pub use nested::{
    parse_with_param, LoadError, LoadsRelations, NestedResource, ResourceTransformer,
};
pub use resource::{ConditionalAttribute, Resource, ResourceWithMeta, WrappedResource};
pub use resource_builder::ResourceBuilder;

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    struct UserResource {
        id: i64,
        name: String,
        email: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        admin_field: Option<String>,
    }

    impl Resource for UserResource {}

    #[test]
    fn test_integration_single_resource() {
        let user = UserResource {
            id: 1,
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            admin_field: None,
        };

        let json = user.to_json().unwrap();
        assert_eq!(json["id"], 1);
        assert_eq!(json["name"], "John Doe");
    }

    #[test]
    fn test_integration_collection() {
        let users = vec![
            UserResource {
                id: 1,
                name: "John".to_string(),
                email: "john@example.com".to_string(),
                admin_field: None,
            },
            UserResource {
                id: 2,
                name: "Jane".to_string(),
                email: "jane@example.com".to_string(),
                admin_field: Some("admin".to_string()),
            },
        ];

        let collection = Collection::new(users);
        assert_eq!(collection.count(), 2);
    }

    #[test]
    fn test_integration_paginated_collection() {
        let users = vec![UserResource {
            id: 1,
            name: "John".to_string(),
            email: "john@example.com".to_string(),
            admin_field: None,
        }];

        let meta = PaginationMeta::new(1, 10, 25);
        let collection = PaginatedCollection::new(users, meta);

        assert_eq!(collection.items().len(), 1);
        assert_eq!(collection.meta().total, 25);
        assert!(collection.meta().has_next_page());
    }
}
