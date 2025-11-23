//! # Foundry Resources
//!
//! API Resource transformation layer for RustForge Framework.
//! Provides structured API responses with pagination, field filtering, and nested resources.
//!
//! ## Features
//! - Resource Transformation (Model → JSON)
//! - Resource Collections with Pagination
//! - Nested Resources
//! - Field Filtering & Sparse Fieldsets
//! - Resource Metadata & Links
//!
//! ## Example
//! ```rust,no_run
//! use rf_resources::{Resource, ResourceCollection, Pagination};
//! use serde::{Serialize, Deserialize};
//!
//! #[derive(Serialize)]
//! struct UserResource {
//!     id: i32,
//!     name: String,
//!     email: String,
//! }
//!
//! impl Resource for UserResource {
//!     type Model = User;
//!
//!     fn from_model(model: Self::Model) -> Self {
//!         Self {
//!             id: model.id,
//!             name: model.name,
//!             email: model.email,
//!         }
//!     }
//! }
//! ```

pub mod collection;
pub mod filter;
pub mod macros;
pub mod metadata;
pub mod pagination;
pub mod resource;
pub mod response;

pub use collection::{CollectionOptions, ResourceCollection};
pub use filter::{FieldFilter, FilterOptions};
pub use metadata::{Metadata, MetadataBuilder};
pub use pagination::{Pagination, PaginationLinks, PaginationMeta};
pub use resource::{Resource, ResourceContext, ResourceOptions};
pub use response::{ApiError, ApiResponse};

#[derive(Debug, thiserror::Error)]
pub enum ResourceError {
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Invalid field: {0}")]
    InvalidField(String),

    #[error("Invalid filter: {0}")]
    InvalidFilter(String),

    #[error("Resource not found")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, ResourceError>;
