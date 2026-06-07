use crate::{
    driver::VectorDriver,
    embedding::{Embedding, VectorSearchResult},
    error::{VectorError, VectorResult},
};

/// Builder for constructing and executing vector similarity queries.
///
/// # Example
///
/// ```rust,ignore
/// let results = VectorQuery::new(&driver)
///     .similar_to(query_embedding)
///     .limit(10)
///     .min_score(0.7)
///     .get()
///     .await?;
/// ```
pub struct VectorQuery<'a> {
    driver: &'a dyn VectorDriver,
    embedding: Option<Embedding>,
    limit: usize,
    min_score: Option<f32>,
}

impl<'a> VectorQuery<'a> {
    /// Create a new query builder targeting the given driver.
    pub fn new(driver: &'a dyn VectorDriver) -> Self {
        Self {
            driver,
            embedding: None,
            limit: 10,
            min_score: None,
        }
    }

    /// Set the query embedding (required before calling [`get`]).
    pub fn similar_to(mut self, embedding: Embedding) -> Self {
        self.embedding = Some(embedding);
        self
    }

    /// Maximum number of results to return (default: 10).
    pub fn limit(mut self, n: usize) -> Self {
        self.limit = n;
        self
    }

    /// Filter out results with a score below `s`.
    pub fn min_score(mut self, s: f32) -> Self {
        self.min_score = Some(s);
        self
    }

    /// Execute the query and return the matching documents.
    ///
    /// Returns an error if no embedding has been provided via [`similar_to`].
    pub async fn get(self) -> VectorResult<Vec<VectorSearchResult>> {
        let embedding = self.embedding.ok_or(VectorError::EmptyEmbedding)?;

        // Fetch more than needed when min_score filtering is active, because
        // we may discard some results.
        let fetch_limit = if self.min_score.is_some() {
            self.limit.saturating_mul(4).max(self.limit)
        } else {
            self.limit
        };

        let mut results = self.driver.search(&embedding, fetch_limit).await?;

        // Apply minimum-score filter.
        if let Some(min) = self.min_score {
            results.retain(|r| r.score >= min);
        }

        // Trim to the requested limit and fix up ranks.
        results.truncate(self.limit);
        for (i, result) in results.iter_mut().enumerate() {
            result.rank = i + 1;
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        drivers::memory::MemoryDriver,
        embedding::{Embedding, VectorDocument},
    };

    fn make_doc(id: &str, values: Vec<f32>) -> VectorDocument {
        VectorDocument::new(id, format!("doc {id}"), Embedding::new(values))
    }

    #[tokio::test]
    async fn test_query_without_embedding_returns_error() {
        let driver = MemoryDriver::new();
        let result = VectorQuery::new(&driver).get().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_returns_results() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("q1", vec![1.0, 0.0])).await.unwrap();
        driver.upsert(make_doc("q2", vec![0.0, 1.0])).await.unwrap();

        let query_emb = Embedding::new(vec![1.0, 0.0]);
        let results = VectorQuery::new(&driver)
            .similar_to(query_emb)
            .limit(2)
            .get()
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].document.id, "q1");
    }

    #[tokio::test]
    async fn test_min_score_filters_low_results() {
        let driver = MemoryDriver::new();
        // doc_a has very high similarity; doc_b is orthogonal (score ≈ 0).
        driver.upsert(make_doc("a", vec![1.0, 0.0])).await.unwrap();
        driver.upsert(make_doc("b", vec![0.0, 1.0])).await.unwrap();

        let query = Embedding::new(vec![1.0, 0.0]);
        let results = VectorQuery::new(&driver)
            .similar_to(query)
            .limit(10)
            .min_score(0.5)
            .get()
            .await
            .unwrap();

        // Only doc_a should pass the threshold.
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "a");
    }

    #[tokio::test]
    async fn test_query_limit_respected() {
        let driver = MemoryDriver::new();
        for i in 0..5 {
            driver
                .upsert(make_doc(&format!("d{i}"), vec![1.0, i as f32]))
                .await
                .unwrap();
        }

        let query = Embedding::new(vec![1.0, 0.0]);
        let results = VectorQuery::new(&driver)
            .similar_to(query)
            .limit(2)
            .get()
            .await
            .unwrap();

        assert_eq!(results.len(), 2);
    }

    #[tokio::test]
    async fn test_query_rank_reassigned_after_filter() {
        let driver = MemoryDriver::new();
        driver.upsert(make_doc("r1", vec![1.0, 0.0])).await.unwrap();
        driver.upsert(make_doc("r2", vec![0.9, 0.1])).await.unwrap();
        driver.upsert(make_doc("r3", vec![0.0, 1.0])).await.unwrap();

        let query = Embedding::new(vec![1.0, 0.0]);
        let results = VectorQuery::new(&driver)
            .similar_to(query)
            .limit(10)
            .min_score(0.5)
            .get()
            .await
            .unwrap();

        for (i, r) in results.iter().enumerate() {
            assert_eq!(r.rank, i + 1, "ranks should be 1-based after filtering");
        }
    }
}
