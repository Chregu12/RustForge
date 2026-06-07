use crate::{
    embedding::{EmbeddingRequest, EmbeddingResponse},
    error::AiResult,
    text::{TextRequest, TextResponse},
    tool::{Tool, ToolCallResponse},
};
use async_trait::async_trait;

/// Core abstraction over any AI provider.
///
/// Implement this trait to add a new backend (e.g. Ollama, Gemini, …).
#[async_trait]
pub trait AiDriver: Send + Sync {
    /// Generate a text completion from a conversation.
    async fn generate_text(&self, req: TextRequest) -> AiResult<TextResponse>;

    /// Generate dense embedding vectors for the given inputs.
    async fn embed(&self, req: EmbeddingRequest) -> AiResult<EmbeddingResponse>;

    /// Generate a response with the ability to call tools / functions.
    async fn generate_with_tools(
        &self,
        req: TextRequest,
        tools: Vec<Tool>,
    ) -> AiResult<ToolCallResponse>;
}
