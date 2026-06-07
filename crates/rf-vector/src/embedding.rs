use serde::{Deserialize, Serialize};

/// A dense floating-point embedding vector.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Embedding {
    /// The raw float values of the embedding.
    pub values: Vec<f32>,
}

impl Embedding {
    /// Create a new embedding from a vector of values.
    pub fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Return the number of dimensions in this embedding.
    pub fn dimensions(&self) -> usize {
        self.values.len()
    }

    /// Compute the cosine similarity between this embedding and another.
    ///
    /// Returns a value in `[-1.0, 1.0]`. Returns `0.0` if either vector has
    /// zero magnitude.
    pub fn cosine_similarity(&self, other: &Self) -> f32 {
        let dot = self.dot_product(other);
        let mag_self = self.magnitude();
        let mag_other = other.magnitude();
        if mag_self == 0.0 || mag_other == 0.0 {
            return 0.0;
        }
        dot / (mag_self * mag_other)
    }

    /// Compute the dot product of this embedding with another.
    pub fn dot_product(&self, other: &Self) -> f32 {
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| a * b)
            .sum()
    }

    /// Compute the Euclidean (L2) distance between this embedding and another.
    pub fn euclidean_distance(&self, other: &Self) -> f32 {
        self.values
            .iter()
            .zip(other.values.iter())
            .map(|(a, b)| (a - b).powi(2))
            .sum::<f32>()
            .sqrt()
    }

    /// Compute the L2 magnitude (norm) of this embedding.
    pub fn magnitude(&self) -> f32 {
        self.values.iter().map(|v| v * v).sum::<f32>().sqrt()
    }

    /// Return a unit-length (L2-normalized) copy of this embedding.
    ///
    /// If the magnitude is zero, returns a zero vector of the same dimensions.
    pub fn normalize(&self) -> Self {
        let mag = self.magnitude();
        if mag == 0.0 {
            return self.clone();
        }
        Self::new(self.values.iter().map(|v| v / mag).collect())
    }

    /// Create an embedding of the given dimensionality filled with zeros.
    pub fn zeros(dimensions: usize) -> Self {
        Self::new(vec![0.0; dimensions])
    }

    /// Create an embedding from a slice of `f32` values.
    pub fn from_slice(values: &[f32]) -> Self {
        Self::new(values.to_vec())
    }
}

/// A document stored in the vector index, consisting of an identifier, raw
/// content, its embedding, and optional arbitrary metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorDocument {
    /// Unique identifier for this document.
    pub id: String,
    /// Human-readable text content.
    pub content: String,
    /// The dense embedding that represents this document.
    pub embedding: Embedding,
    /// Optional structured metadata (key-value pairs, tags, etc.).
    pub metadata: serde_json::Value,
}

impl VectorDocument {
    /// Create a new document with empty metadata.
    pub fn new(
        id: impl Into<String>,
        content: impl Into<String>,
        embedding: Embedding,
    ) -> Self {
        Self {
            id: id.into(),
            content: content.into(),
            embedding,
            metadata: serde_json::Value::Null,
        }
    }

    /// Attach metadata to this document (builder style).
    pub fn with_metadata(mut self, metadata: serde_json::Value) -> Self {
        self.metadata = metadata;
        self
    }
}

/// A single result from a vector similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorSearchResult {
    /// The matching document.
    pub document: VectorDocument,
    /// The similarity score (higher is more similar; exact semantics depend on
    /// the driver, but cosine similarity is used by default).
    pub score: f32,
    /// 1-based rank in the result list (1 = most similar).
    pub rank: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_sets_values() {
        let e = Embedding::new(vec![1.0, 2.0, 3.0]);
        assert_eq!(e.values, vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn test_dimensions() {
        let e = Embedding::new(vec![0.0; 128]);
        assert_eq!(e.dimensions(), 128);
    }

    #[test]
    fn test_cosine_similarity_same_vectors() {
        let e = Embedding::new(vec![1.0, 0.0, 0.0]);
        let sim = e.cosine_similarity(&e);
        assert!((sim - 1.0).abs() < 1e-6, "cosine_similarity of identical vectors should be 1.0, got {sim}");
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = Embedding::new(vec![1.0, 0.0]);
        let b = Embedding::new(vec![0.0, 1.0]);
        let sim = a.cosine_similarity(&b);
        assert!((sim - 0.0).abs() < 1e-6, "cosine_similarity of orthogonal vectors should be 0.0, got {sim}");
    }

    #[test]
    fn test_dot_product() {
        let a = Embedding::new(vec![1.0, 2.0, 3.0]);
        let b = Embedding::new(vec![4.0, 5.0, 6.0]);
        let dot = a.dot_product(&b);
        assert!((dot - 32.0).abs() < 1e-6, "dot product should be 32.0, got {dot}");
    }

    #[test]
    fn test_euclidean_distance_same_vector() {
        let e = Embedding::new(vec![1.0, 2.0, 3.0]);
        let dist = e.euclidean_distance(&e);
        assert!((dist - 0.0).abs() < 1e-6, "euclidean_distance of identical vectors should be 0.0, got {dist}");
    }

    #[test]
    fn test_euclidean_distance_known() {
        let a = Embedding::new(vec![0.0, 0.0]);
        let b = Embedding::new(vec![3.0, 4.0]);
        let dist = a.euclidean_distance(&b);
        assert!((dist - 5.0).abs() < 1e-6, "expected 5.0, got {dist}");
    }

    #[test]
    fn test_magnitude() {
        let e = Embedding::new(vec![3.0, 4.0]);
        let mag = e.magnitude();
        assert!((mag - 5.0).abs() < 1e-6, "expected magnitude 5.0, got {mag}");
    }

    #[test]
    fn test_normalize_magnitude_approx_one() {
        let e = Embedding::new(vec![3.0, 4.0]);
        let n = e.normalize();
        let mag = n.magnitude();
        assert!((mag - 1.0).abs() < 1e-6, "normalized magnitude should be ~1.0, got {mag}");
    }

    #[test]
    fn test_normalize_zero_vector() {
        let e = Embedding::zeros(3);
        let n = e.normalize();
        assert_eq!(n.values, vec![0.0, 0.0, 0.0], "normalizing zero vector should return zero vector");
    }

    #[test]
    fn test_zeros() {
        let e = Embedding::zeros(5);
        assert_eq!(e.values, vec![0.0; 5]);
        assert_eq!(e.dimensions(), 5);
    }

    #[test]
    fn test_from_slice() {
        let slice = &[1.0f32, 2.0, 3.0];
        let e = Embedding::from_slice(slice);
        assert_eq!(e.values, slice);
    }

    #[test]
    fn test_vector_document_new() {
        let emb = Embedding::zeros(4);
        let doc = VectorDocument::new("id1", "hello world", emb.clone());
        assert_eq!(doc.id, "id1");
        assert_eq!(doc.content, "hello world");
        assert_eq!(doc.embedding, emb);
        assert_eq!(doc.metadata, serde_json::Value::Null);
    }

    #[test]
    fn test_vector_document_with_metadata() {
        let emb = Embedding::zeros(4);
        let meta = serde_json::json!({ "tag": "test" });
        let doc = VectorDocument::new("id2", "content", emb)
            .with_metadata(meta.clone());
        assert_eq!(doc.metadata, meta);
    }
}
