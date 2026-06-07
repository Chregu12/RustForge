use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::{
    driver::VectorDriver,
    embedding::{Embedding, VectorDocument, VectorSearchResult},
    error::VectorResult,
};

/// In-memory vector store backed by a `HashMap` protected with a `RwLock`.
///
/// This driver is suitable for testing and small-scale use cases.  All data
/// is lost when the driver is dropped.
#[derive(Debug, Clone)]
pub struct MemoryDriver {
    store: Arc<RwLock<HashMap<String, VectorDocument>>>,
}

impl MemoryDriver {
    /// Create a new, empty in-memory driver.
    pub fn new() -> Self {
        Self {
            store: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for MemoryDriver {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VectorDriver for MemoryDriver {
    async fn upsert(&self, doc: VectorDocument) -> VectorResult<()> {
        let mut store = self.store.write().await;
        store.insert(doc.id.clone(), doc);
        Ok(())
    }

    async fn upsert_many(&self, docs: Vec<VectorDocument>) -> VectorResult<()> {
        let mut store = self.store.write().await;
        for doc in docs {
            store.insert(doc.id.clone(), doc);
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> VectorResult<bool> {
        let mut store = self.store.write().await;
        Ok(store.remove(id).is_some())
    }

    async fn get(&self, id: &str) -> VectorResult<Option<VectorDocument>> {
        let store = self.store.read().await;
        Ok(store.get(id).cloned())
    }

    async fn search(
        &self,
        query: &Embedding,
        limit: usize,
    ) -> VectorResult<Vec<VectorSearchResult>> {
        let store = self.store.read().await;

        // Score every document.
        let mut scored: Vec<(f32, VectorDocument)> = store
            .values()
            .map(|doc| {
                let score = query.cosine_similarity(&doc.embedding);
                (score, doc.clone())
            })
            .collect();

        // Sort by descending score (highest similarity first).
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        // Take at most `limit` results and assign 1-based ranks.
        let results = scored
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(i, (score, document))| VectorSearchResult {
                document,
                score,
                rank: i + 1,
            })
            .collect();

        Ok(results)
    }

    async fn count(&self) -> VectorResult<usize> {
        let store = self.store.read().await;
        Ok(store.len())
    }

    async fn clear(&self) -> VectorResult<()> {
        let mut store = self.store.write().await;
        store.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedding::Embedding;

    fn make_doc(id: &str, values: Vec<f32>) -> VectorDocument {
        VectorDocument::new(id, format!("content for {id}"), Embedding::new(values))
    }

    #[tokio::test]
    async fn test_upsert_and_get() {
        let driver = MemoryDriver::new();
        let doc = make_doc("doc1", vec![1.0, 0.0, 0.0]);
        driver.upsert(doc.clone()).await.unwrap();

        let retrieved = driver.get("doc1").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "doc1");
    }

    #[tokio::test]
    async fn test_get_nonexistent_returns_none() {
        let driver = MemoryDriver::new();
        let result = driver.get("nonexistent").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_upsert_overwrites_existing() {
        let driver = MemoryDriver::new();
        let doc1 = make_doc("id1", vec![1.0, 0.0]);
        let doc2 = VectorDocument::new("id1", "updated content", Embedding::new(vec![0.0, 1.0]));

        driver.upsert(doc1).await.unwrap();
        driver.upsert(doc2).await.unwrap();

        let retrieved = driver.get("id1").await.unwrap().unwrap();
        assert_eq!(retrieved.content, "updated content");
    }

    #[tokio::test]
    async fn test_upsert_many() {
        let driver = MemoryDriver::new();
        let docs = vec![
            make_doc("a", vec![1.0, 0.0]),
            make_doc("b", vec![0.0, 1.0]),
            make_doc("c", vec![1.0, 1.0]),
        ];
        driver.upsert_many(docs).await.unwrap();
        assert_eq!(driver.count().await.unwrap(), 3);
    }

    #[tokio::test]
    async fn test_delete_existing_returns_true() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("del1", vec![1.0])).await.unwrap();
        let deleted = driver.delete("del1").await.unwrap();
        assert!(deleted);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_returns_false() {
        let driver = MemoryDriver::new();
        let deleted = driver.delete("ghost").await.unwrap();
        assert!(!deleted);
    }

    #[tokio::test]
    async fn test_delete_removes_document() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("d1", vec![1.0])).await.unwrap();
        driver.delete("d1").await.unwrap();
        assert!(driver.get("d1").await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_count_empty() {
        let driver = MemoryDriver::new();
        assert_eq!(driver.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_count_after_inserts() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("x", vec![1.0])).await.unwrap();
        driver.upsert(make_doc("y", vec![2.0])).await.unwrap();
        assert_eq!(driver.count().await.unwrap(), 2);
    }

    #[tokio::test]
    async fn test_clear_empties_store() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("c1", vec![1.0])).await.unwrap();
        driver.upsert(make_doc("c2", vec![2.0])).await.unwrap();
        driver.clear().await.unwrap();
        assert_eq!(driver.count().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn test_search_finds_most_similar() {
        let driver = MemoryDriver::new();
        // doc_a is aligned with the query; doc_b is orthogonal
        let doc_a = make_doc("a", vec![1.0, 0.0]);
        let doc_b = make_doc("b", vec![0.0, 1.0]);
        driver.upsert(doc_a).await.unwrap();
        driver.upsert(doc_b).await.unwrap();

        let query = Embedding::new(vec![1.0, 0.0]);
        let results = driver.search(&query, 2).await.unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document.id, "a", "most similar should be doc_a");
    }

    #[tokio::test]
    async fn test_search_respects_limit() {
        let driver = MemoryDriver::new();
        for i in 0..10 {
            driver
                .upsert(make_doc(
                    &format!("doc{i}"),
                    vec![i as f32, 0.0],
                ))
                .await
                .unwrap();
        }

        let query = Embedding::new(vec![1.0, 0.0]);
        let results = driver.search(&query, 3).await.unwrap();
        assert_eq!(results.len(), 3);
    }

    #[tokio::test]
    async fn test_search_rank_is_one_based() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("r1", vec![1.0, 0.0])).await.unwrap();
        driver.upsert(make_doc("r2", vec![0.0, 1.0])).await.unwrap();

        let query = Embedding::new(vec![1.0, 0.0]);
        let results = driver.search(&query, 2).await.unwrap();

        assert_eq!(results[0].rank, 1);
        assert_eq!(results[1].rank, 2);
    }

    #[tokio::test]
    async fn test_search_empty_store() {
        let driver = MemoryDriver::new();
        let query = Embedding::new(vec![1.0, 0.0]);
        let results = driver.search(&query, 5).await.unwrap();
        assert!(results.is_empty());
    }
}
