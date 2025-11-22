//! # Query Module
//!
//! Advanced query building features for rf-orm including:
//! - Subquery support
//! - Advanced aggregations
//! - Query optimization helpers

pub mod aggregations;
pub mod subquery;

pub use aggregations::{AggregateType, AggregationBuilder, WithAggregates};
pub use subquery::{Subquery, SubqueryBuilder};
