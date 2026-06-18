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

#[cfg(test)]
mod adversarial {
    use super::*;
    use serde_json::json;

    // Independently compute the expected ranking and compare against the store.
    fn brute_force_rank(
        items: &[(&str, Vec<f32>)],
        query: &Vector,
        metric: DistanceMetric,
    ) -> Vec<String> {
        let mut scored: Vec<(String, f32)> = items
            .iter()
            .filter(|(_, v)| v.len() == query.dimension())
            .map(|(id, v)| {
                ((*id).to_string(), metric.score(query, &Vector::new(v.clone())))
            })
            .collect();
        scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        scored.into_iter().map(|(id, _)| id).collect()
    }

    #[test]
    fn fresh_store_is_empty() {
        let store = InMemoryVectorStore::new();
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
        // searching an empty store yields nothing, not a panic
        let hits = store.search(&Vector::new(vec![1.0, 0.0]), 5, DistanceMetric::Cosine);
        assert!(hits.is_empty());
    }

    #[test]
    fn results_sorted_descending_under_all_metrics() {
        let items = [
            ("a", vec![1.0, 0.0, 0.0]),
            ("b", vec![0.0, 1.0, 0.0]),
            ("c", vec![0.9, 0.1, 0.0]),
            ("d", vec![0.5, 0.5, 0.7]),
            ("e", vec![-1.0, 0.0, 0.2]),
        ];
        let q = Vector::new(vec![1.0, 0.05, 0.0]);

        for metric in [
            DistanceMetric::Cosine,
            DistanceMetric::Euclidean,
            DistanceMetric::DotProduct,
        ] {
            let mut store = InMemoryVectorStore::new();
            for (id, v) in &items {
                store.add(*id, Vector::new(v.clone()), json!({ "id": id }));
            }
            let hits = store.search(&q, items.len(), metric);

            // (a) strictly non-increasing score
            for w in hits.windows(2) {
                assert!(
                    w[0].score >= w[1].score,
                    "metric {metric:?} not sorted: {} then {}",
                    w[0].score,
                    w[1].score
                );
            }

            // (d) order matches independent brute-force ranking
            let expected = brute_force_rank(&items, &q, metric);
            let got: Vec<String> = hits.iter().map(|h| h.id.clone()).collect();
            assert_eq!(got, expected, "metric {metric:?} ranking mismatch");

            // (c) metadata travels with the right id
            for h in &hits {
                assert_eq!(h.metadata, json!({ "id": h.id }));
            }
        }
    }

    #[test]
    fn truncation_to_k() {
        let mut store = InMemoryVectorStore::new();
        for i in 0..10 {
            store.add(
                format!("v{i}"),
                Vector::new(vec![i as f32, 0.0]),
                json!(i),
            );
        }
        let q = Vector::new(vec![1.0, 0.0]);
        assert_eq!(store.search(&q, 3, DistanceMetric::Cosine).len(), 3);
        assert_eq!(store.search(&q, 0, DistanceMetric::Cosine).len(), 0);
        // k larger than store size: returns everything, no panic
        assert_eq!(store.search(&q, 100, DistanceMetric::Cosine).len(), 10);
    }

    #[test]
    fn nearest_neighbor_correct_per_metric() {
        // Distinct nearest neighbors for euclidean vs dot product can differ,
        // so verify each picks the truly-best entry.
        let items = [
            ("origin_ish", vec![0.1, 0.1]),
            ("big_same_dir", vec![10.0, 10.0]),
            ("unit", vec![1.0, 0.0]),
        ];
        let q = Vector::new(vec![0.2, 0.2]);

        let mut store = InMemoryVectorStore::new();
        for (id, v) in &items {
            store.add(*id, Vector::new(v.clone()), json!(null));
        }

        // Euclidean: closest in L2 is origin_ish.
        let euc = store.search(&q, 1, DistanceMetric::Euclidean);
        assert_eq!(euc[0].id, brute_force_rank(&items, &q, DistanceMetric::Euclidean)[0]);
        assert_eq!(euc[0].id, "origin_ish");

        // Dot product: largest dot with q is big_same_dir.
        let dp = store.search(&q, 1, DistanceMetric::DotProduct);
        assert_eq!(dp[0].id, "big_same_dir");
    }

    #[test]
    fn overwrite_updates_vector_and_metadata() {
        let mut store = InMemoryVectorStore::new();
        store.add("x", Vector::new(vec![1.0, 0.0]), json!({"v": 1}));
        store.add("y", Vector::new(vec![0.0, 1.0]), json!({"v": 2}));
        // Overwrite x to point the other way + new metadata
        store.add("x", Vector::new(vec![0.0, 1.0]), json!({"v": 99}));
        assert_eq!(store.len(), 2);

        let hits = store.search(&Vector::new(vec![0.0, 1.0]), 2, DistanceMetric::Cosine);
        // x now aligns with [0,1]; its metadata must reflect the overwrite.
        let x = hits.iter().find(|h| h.id == "x").unwrap();
        assert_eq!(x.metadata, json!({"v": 99}));
        assert!((x.score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn remove_len_is_empty_lifecycle() {
        let mut store = InMemoryVectorStore::new();
        assert!(store.is_empty());
        store.add("a", Vector::new(vec![1.0]), json!(null));
        store.add("b", Vector::new(vec![2.0]), json!(null));
        assert_eq!(store.len(), 2);
        assert!(!store.is_empty());

        assert!(store.remove("a"));
        assert_eq!(store.len(), 1);
        assert!(!store.remove("a")); // already gone
        assert!(!store.remove("nope")); // never existed
        assert!(store.remove("b"));
        assert_eq!(store.len(), 0);
        assert!(store.is_empty());
    }

    #[test]
    fn with_metric_nearest_uses_default() {
        let mut store = InMemoryVectorStore::with_metric(DistanceMetric::Euclidean);
        store.add("origin_ish", Vector::new(vec![0.1, 0.1]), json!(null));
        store.add("big", Vector::new(vec![10.0, 10.0]), json!(null));
        let q = Vector::new(vec![0.2, 0.2]);
        // Euclidean default -> origin_ish nearest, NOT big (which dot-product would pick)
        let hits = store.nearest(&q, 1);
        assert_eq!(hits[0].id, "origin_ish");
    }
}
