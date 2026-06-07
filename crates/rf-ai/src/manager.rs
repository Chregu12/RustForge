use std::sync::Arc;

use crate::{
    driver::AiDriver,
    embedding::{EmbeddingRequest, EmbeddingResponse},
    error::{AiError, AiResult},
    text::{TextRequest, TextResponse},
    tool::{Tool, ToolCallResponse},
};

/// High-level manager that wraps an [`AiDriver`] implementation.
///
/// Analogous to Laravel's `MailManager` — it owns an `Arc<dyn AiDriver>` and
/// exposes the same API, so callers never need to know which backend is active.
pub struct AiManager {
    inner: Arc<dyn AiDriver>,
}

impl std::fmt::Debug for AiManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AiManager").field("inner", &"<dyn AiDriver>").finish()
    }
}

impl AiManager {
    /// Wrap any type that implements [`AiDriver`].
    pub fn new(driver: impl AiDriver + 'static) -> Self {
        Self { inner: Arc::new(driver) }
    }

    /// Create a manager backed by the Anthropic Claude API.
    ///
    /// Only available when the `anthropic` feature is enabled.
    #[cfg(feature = "anthropic")]
    pub fn anthropic(api_key: impl Into<String>) -> Self {
        use crate::drivers::anthropic::AnthropicDriver;
        Self::new(AnthropicDriver::new(api_key))
    }

    /// Create a manager backed by the OpenAI API.
    ///
    /// Only available when the `openai` feature is enabled.
    #[cfg(feature = "openai")]
    pub fn openai(api_key: impl Into<String>) -> Self {
        use crate::drivers::openai::OpenAiDriver;
        Self::new(OpenAiDriver::new(api_key))
    }

    /// Construct a manager by reading `AI_DRIVER` and `AI_API_KEY` from the
    /// environment.
    ///
    /// `AI_DRIVER` must be one of `"anthropic"` or `"openai"` (case-insensitive).
    ///
    /// # Errors
    ///
    /// Returns [`AiError::MissingEnvVar`] when either variable is absent, and
    /// [`AiError::UnknownDriver`] when the driver name is not recognised.
    #[allow(unused_variables)]
    pub fn from_env() -> AiResult<Self> {
        let driver_name = std::env::var("AI_DRIVER")
            .map_err(|_| AiError::MissingEnvVar("AI_DRIVER".to_string()))?;

        let api_key = std::env::var("AI_API_KEY")
            .map_err(|_| AiError::MissingEnvVar("AI_API_KEY".to_string()))?;

        match driver_name.to_lowercase().as_str() {
            #[cfg(feature = "anthropic")]
            "anthropic" => Ok(Self::anthropic(api_key)),

            #[cfg(feature = "openai")]
            "openai" => Ok(Self::openai(api_key)),

            other => Err(AiError::UnknownDriver(other.to_string())),
        }
    }

    /// Generate a text completion.
    pub async fn generate_text(&self, req: TextRequest) -> AiResult<TextResponse> {
        self.inner.generate_text(req).await
    }

    /// Generate embedding vectors.
    pub async fn embed(&self, req: EmbeddingRequest) -> AiResult<EmbeddingResponse> {
        self.inner.embed(req).await
    }

    /// Generate a response that may include tool calls.
    pub async fn generate_with_tools(
        &self,
        req: TextRequest,
        tools: Vec<Tool>,
    ) -> AiResult<ToolCallResponse> {
        self.inner.generate_with_tools(req, tools).await
    }
}
