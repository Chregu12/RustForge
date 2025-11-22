//! Search driver implementations

pub mod meilisearch;
pub mod postgresql;

#[cfg(feature = "algolia")]
pub mod algolia;

pub use meilisearch::MeilisearchDriver;
pub use postgresql::PostgresSearchDriver;

#[cfg(feature = "algolia")]
pub use algolia::{AlgoliaConfig, AlgoliaDriver};
