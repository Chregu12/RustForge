//! In-memory vector store with brute-force k-nearest-neighbour search.

use serde_json::Value;

use crate::vector::{DistanceMetric, Vector};

/// A single ranked hit returned from a [`VectorStore`] search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Identifier of the stored entry.
    pub id: String,
    /// Similarity score (higher = more similar, per [`DistanceMetric::score`]).
    pub score: f32,
    /// Arbitrary metadata stored alongside the vector.
    pub metadata: Value,
}

/// A store of vectors that can be queried for nearest neighbours.
///
/// `add` takes `impl Into<String>` and is therefore **not** object-safe; use a
/// concrete type such as [`InMemoryVectorStore`] when you call it. The remaining
/// methods are object-safe.
pub trait VectorStore {
    /// Insert (or overwrite) a vector with associated metadata.
    fn add(&mut self, id: impl Into<String>, vector: Vector, metadata: Value);

    /// Remove an entry by id. Returns `true` if something was removed.
    fn remove(&mut self, id: &str) -> bool;

    /// Number of stored vectors.
    fn len(&self) -> usize;

    /// `true` if the store holds no vectors.
    fn is_empty(&self) -> bool;

    /// Return the `k` entries most similar to `query` under `metric`,
    /// sorted by descending score (most similar first).
    fn search(&self, query: &Vector, k: usize, metric: DistanceMetric) -> Vec<SearchResult>;
}

struct Entry {
    id: String,
    vector: Vector,
    metadata: Value,
}

/// Brute-force, in-memory implementation of [`VectorStore`].
///
/// Search scores every stored vector with the chosen [`DistanceMetric`] and
/// returns the top `k`. Vectors whose dimension differs from the query are
/// skipped.
///
/// # Examples
///
/// ```rust
/// use rf_vector::{InMemoryVectorStore, Vector, VectorStore, DistanceMetric};
/// use serde_json::json;
///
/// let mut store = InMemoryVectorStore::new();
/// store.add("a", Vector::new(vec![1.0, 0.0]), json!({"label": "a"}));
/// store.add("b", Vector::new(vec![0.0, 1.0]), json!({"label": "b"}));
///
/// let hits = store.search(&Vector::new(vec![0.9, 0.1]), 1, DistanceMetric::Cosine);
/// assert_eq!(hits[0].id, "a");
/// ```
#[derive(Default)]
pub struct InMemoryVectorStore {
    entries: Vec<Entry>,
    default_metric: DistanceMetric,
}

impl InMemoryVectorStore {
    /// Create an empty store (default metric: cosine).
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            default_metric: DistanceMetric::Cosine,
        }
    }

    /// Create an empty store with a default metric used by [`Self::nearest`].
    pub fn with_metric(metric: DistanceMetric) -> Self {
        Self {
            entries: Vec::new(),
            default_metric: metric,
        }
    }

    /// Search using the store's configured default metric.
    pub fn nearest(&self, query: &Vector, k: usize) -> Vec<SearchResult> {
        self.search(query, k, self.default_metric)
    }
}

impl VectorStore for InMemoryVectorStore {
    fn add(&mut self, id: impl Into<String>, vector: Vector, metadata: Value) {
        let id = id.into();
        if let Some(existing) = self.entries.iter_mut().find(|e| e.id == id) {
            existing.vector = vector;
            existing.metadata = metadata;
        } else {
            self.entries.push(Entry { id, vector, metadata });
        }
    }

    fn remove(&mut self, id: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.id != id);
        self.entries.len() != before
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn search(&self, query: &Vector, k: usize, metric: DistanceMetric) -> Vec<SearchResult> {
        let mut results: Vec<SearchResult> = self
            .entries
            .iter()
            .filter(|e| e.vector.dimension() == query.dimension())
            .map(|e| SearchResult {
                id: e.id.clone(),
                score: metric.score(query, &e.vector),
                metadata: e.metadata.clone(),
            })
            .collect();

        // Descending by score (most similar first); NaN sinks to the bottom.
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results.truncate(k);
        results
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn seeded_store() -> InMemoryVectorStore {
        let mut store = InMemoryVectorStore::new();
        store.add("a", Vector::new(vec![1.0, 0.0, 0.0]), json!({"n": "a"}));
        store.add("b", Vector::new(vec![0.0, 1.0, 0.0]), json!({"n": "b"}));
        store.add("c", Vector::new(vec![0.9, 0.1, 0.0]), json!({"n": "c"}));
        store
    }

    #[test]
    fn search_returns_nearest_first_with_k() {
        let store = seeded_store();
        let q = Vector::new(vec![1.0, 0.0, 0.0]);
        let hits = store.search(&q, 2, DistanceMetric::Cosine);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].id, "a");
        assert_eq!(hits[1].id, "c");
        assert!(hits[0].score >= hits[1].score);
    }

    #[test]
    fn euclidean_ranks_nearest_first() {
        let store = seeded_store();
        let q = Vector::new(vec![0.9, 0.1, 0.0]);
        let hits = store.search(&q, 1, DistanceMetric::Euclidean);
        assert_eq!(hits[0].id, "c");
    }

    #[test]
    fn add_overwrites_existing_id() {
        let mut store = seeded_store();
        assert_eq!(store.len(), 3);
        store.add("a", Vector::new(vec![0.0, 0.0, 1.0]), json!({"n": "a2"}));
        assert_eq!(store.len(), 3);
    }

    #[test]
    fn remove_works() {
        let mut store = seeded_store();
        assert!(store.remove("a"));
        assert!(!store.remove("a"));
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());
    }

    #[test]
    fn mismatched_dimensions_skipped() {
        let mut store = InMemoryVectorStore::new();
        store.add("ok", Vector::new(vec![1.0, 0.0]), json!(null));
        store.add("bad", Vector::new(vec![1.0, 0.0, 0.0]), json!(null));
        let hits = store.search(&Vector::new(vec![1.0, 0.0]), 10, DistanceMetric::Cosine);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].id, "ok");
    }
}
