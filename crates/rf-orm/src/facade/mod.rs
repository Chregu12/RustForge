//! DB facade module providing Laravel-style static database API

pub mod db;
pub mod db_manager;
pub mod model;
pub mod query_builder;

pub use db::DB;
pub use db_manager::{DBManager, GLOBAL_DB};
pub use model::Model;
pub use query_builder::{LazyCollection, PaginatedResult, QueryBuilder};
