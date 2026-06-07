use async_trait::async_trait;

use crate::{
    embedding::{Embedding, VectorDocument, VectorSearchResult},
    error::VectorResult,
};

/// Trait that all vector-storage backends must implement.
///
/// Every method is async and returns a [`VectorResult`].
#[async_trait]
pub trait VectorDriver: Send + Sync {
    /// Insert or update a document in the store.
    async fn upsert(&self, doc: VectorDocument) -> VectorResult<()>;

    /// Bulk-insert or update multiple documents.
    async fn upsert_many(&self, docs: Vec<VectorDocument>) -> VectorResult<()>;

    /// Delete a document by its identifier.
    ///
    /// Returns `true` if the document existed, `false` otherwise.
    async fn delete(&self, id: &str) -> VectorResult<bool>;

    /// Retrieve a document by its identifier.
    async fn get(&self, id: &str) -> VectorResult<Option<VectorDocument>>;

    /// Perform a nearest-neighbor search against the stored documents.
    ///
    /// Returns at most `limit` results ordered by descending similarity score.
    async fn search(
        &self,
        query: &Embedding,
        limit: usize,
    ) -> VectorResult<Vec<VectorSearchResult>>;

    /// Return the total number of documents in the store.
    async fn count(&self) -> VectorResult<usize>;

    /// Remove all documents from the store.
    async fn clear(&self) -> VectorResult<()>;
}
