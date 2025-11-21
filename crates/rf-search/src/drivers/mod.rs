//! Search driver implementations

pub mod postgresql;
pub mod meilisearch;

#[cfg(feature = "algolia")]
pub mod algolia;

pub use postgresql::PostgresSearchDriver;
pub use meilisearch::MeilisearchDriver;

#[cfg(feature = "algolia")]
pub use algolia::{AlgoliaDriver, AlgoliaConfig};
