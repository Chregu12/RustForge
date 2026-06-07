use serde::{Deserialize, Serialize};

/// A request to convert text into a dense embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    /// The model to use for embedding (e.g. `"text-embedding-3-small"`).
    pub model: String,
    /// The text(s) to embed. Each string becomes one embedding vector.
    pub input: Vec<String>,
}

impl EmbeddingRequest {
    /// Create an embedding request for a single string.
    pub fn single(model: impl Into<String>, text: impl Into<String>) -> Self {
        Self { model: model.into(), input: vec![text.into()] }
    }

    /// Create an embedding request for multiple strings.
    pub fn batch(model: impl Into<String>, texts: Vec<String>) -> Self {
        Self { model: model.into(), input: texts }
    }
}

/// A single embedding vector returned by the provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// The dense float vector.
    pub vector: Vec<f32>,
    /// Index into the original `input` slice that this vector corresponds to.
    pub index: usize,
}

/// The response from an embedding request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    /// The embedding vectors, one per input string.
    pub embeddings: Vec<Embedding>,
    /// The model that produced the embeddings.
    pub model: String,
    /// Total tokens consumed.
    pub total_tokens: Option<u32>,
}
