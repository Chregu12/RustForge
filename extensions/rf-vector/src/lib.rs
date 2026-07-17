//! # rf-vector: Vector & Semantic Search for RustForge
//!
//! Laravel-13-style vector / semantic search primitives: dense embedding
//! vectors, similarity & distance metrics, an in-memory brute-force vector
//! store, and pure SQL helpers for the Postgres `pgvector` extension.
//!
//! ## Features
//!
//! - **`Vector`**: dense `f32` embeddings with `dot`, `cosine_similarity`,
//!   `euclidean_distance`, `magnitude`, `normalized`, and checked `try_*`
//!   variants.
//! - **`DistanceMetric`**: `Cosine`, `Euclidean`, `DotProduct` with a unified
//!   "higher score = more similar" [`DistanceMetric::score`].
//! - **`InMemoryVectorStore`**: brute-force k-nearest-neighbour search with
//!   JSON metadata.
//! - **`pgvector`**: string helpers (`to_literal`, `operator`,
//!   `order_by_nearest`, `nearest_neighbor_sql`) for use with rf-orm raw query
//!   fragments — no database dependency required.
//!
//! ## Quick Start
//!
//! ```rust
//! use rf_vector::*;
//! use serde_json::json;
//!
//! // Build an in-memory store of embeddings.
//! let mut store = InMemoryVectorStore::new();
//! store.add("doc:cat", Vector::new(vec![1.0, 0.0, 0.0]), json!({"title": "cats"}));
//! store.add("doc:dog", Vector::new(vec![0.0, 1.0, 0.0]), json!({"title": "dogs"}));
//!
//! // Find the nearest neighbour to a query embedding.
//! let query = Vector::new(vec![0.9, 0.1, 0.0]);
//! let hits = store.search(&query, 1, DistanceMetric::Cosine);
//! assert_eq!(hits[0].id, "doc:cat");
//!
//! // Build a pgvector ORDER BY fragment for rf-orm's order_by_raw.
//! let fragment = pgvector::order_by_nearest("embedding", &query, DistanceMetric::Cosine);
//! assert_eq!(fragment, "embedding <=> '[0.9,0.1,0]'");
//! ```

pub mod error;
pub mod pgvector;
pub mod store;
pub mod vector;

pub use error::{VectorError, VectorResult};
pub use store::{InMemoryVectorStore, SearchResult, VectorStore};
pub use vector::{DistanceMetric, Vector};

/// Prelude module for convenient imports.
pub mod prelude {
    pub use crate::{
        DistanceMetric, InMemoryVectorStore, SearchResult, Vector, VectorError, VectorResult,
        VectorStore,
    };
}
