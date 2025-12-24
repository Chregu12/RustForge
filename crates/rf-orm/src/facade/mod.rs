//! DB facade module providing Laravel-style static database API

pub mod db;
pub mod db_manager;
pub mod model;
pub mod query_builder;

pub use db::DB;
pub use db_manager::{DBManager, GLOBAL_DB};
pub use model::Model;
pub use query_builder::{LazyCollection, PaginatedResult, QueryBuilder};

use serde::Serialize;
use serde_json::Value;

/// Trait for converting data to JSON Value.
///
/// This enables struct-based updates without requiring `json!()` macro:
///
/// ```rust,ignore
/// // Before (with json! macro):
/// User::update_by_id(1, json!({"name": "John"})).await?;
///
/// // After (with struct):
/// #[derive(Serialize)]
/// struct UserUpdate { name: String }
/// User::update_by_id(1, UserUpdate { name: "John".into() }).await?;
///
/// // Both still work!
/// ```
pub trait ToValue {
    /// Convert self to a serde_json::Value
    fn to_value(self) -> Value;
}

// Blanket implementation for anything that implements Serialize
impl<T: Serialize> ToValue for T {
    fn to_value(self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}
