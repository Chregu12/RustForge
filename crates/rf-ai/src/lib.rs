//! # rf-ai — Laravel 13 AI SDK equivalent for RustForge
//!
//! Provides a unified interface to multiple AI providers (Anthropic, OpenAI,
//! and more) following the same manager/driver pattern as `rf-mail`.
//!
//! ## Quick Start
//!
//! ```no_run
//! use rf_ai::{AiManager, TextRequest, Message};
//!
//! # async fn example() -> Result<(), rf_ai::AiError> {
//! // Use environment variables AI_DRIVER and AI_API_KEY:
//! let ai = AiManager::from_env()?;
//!
//! let req = TextRequest::builder("claude-3-5-sonnet-20241022")
//!     .message(Message::user("What is the capital of France?"))
//!     .build();
//!
//! let response = ai.generate_text(req).await?;
//! println!("{}", response.content);
//! # Ok(())
//! # }
//! ```

pub mod driver;
pub mod drivers;
pub mod embedding;
pub mod error;
pub mod manager;
pub mod text;
pub mod tool;

// Convenience re-exports
pub use driver::AiDriver;
pub use drivers::{AnthropicDriver, OpenAiDriver};
pub use embedding::{Embedding, EmbeddingRequest, EmbeddingResponse};
pub use error::{AiError, AiResult};
pub use manager::AiManager;
pub use text::{Message, TextRequest, TextRequestBuilder, TextResponse};
pub use tool::{Tool, ToolCall, ToolCallResponse};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ── TextRequest / Message tests ──────────────────────────────────────────

    #[test]
    fn text_request_builder_sets_model() {
        let req = TextRequest::builder("claude-3-5-sonnet-20241022").build();
        assert_eq!(req.model, "claude-3-5-sonnet-20241022");
    }

    #[test]
    fn text_request_builder_adds_messages() {
        let req = TextRequest::builder("gpt-4o")
            .message(Message::user("hello"))
            .message(Message::assistant("hi!"))
            .build();
        assert_eq!(req.messages.len(), 2);
    }

    #[test]
    fn text_request_builder_sets_temperature() {
        let req = TextRequest::builder("gpt-4o").temperature(0.7).build();
        assert_eq!(req.temperature, Some(0.7));
    }

    #[test]
    fn text_request_builder_sets_max_tokens() {
        let req = TextRequest::builder("gpt-4o").max_tokens(512).build();
        assert_eq!(req.max_tokens, Some(512));
    }

    #[test]
    fn text_request_builder_sets_system_prompt() {
        let req = TextRequest::builder("gpt-4o").system("You are helpful.").build();
        assert_eq!(req.system.as_deref(), Some("You are helpful."));
    }

    // ── Message type tests ───────────────────────────────────────────────────

    #[test]
    fn message_user_has_correct_role() {
        let m = Message::user("test");
        assert!(matches!(m, Message::User { .. }));
        assert_eq!(m.content(), "test");
    }

    #[test]
    fn message_assistant_has_correct_role() {
        let m = Message::assistant("reply");
        assert!(matches!(m, Message::Assistant { .. }));
        assert_eq!(m.content(), "reply");
    }

    #[test]
    fn message_system_has_correct_role() {
        let m = Message::system("You are a helpful assistant.");
        assert!(matches!(m, Message::System { .. }));
        assert_eq!(m.content(), "You are a helpful assistant.");
    }

    #[test]
    fn message_serialises_with_role_tag() {
        let m = Message::user("hello");
        let json = serde_json::to_string(&m).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("hello"));
    }

    // ── Tool / ToolCall tests ────────────────────────────────────────────────

    #[test]
    fn tool_new_stores_fields() {
        let tool = Tool::new(
            "get_weather",
            "Returns current weather",
            json!({ "type": "object", "properties": { "city": { "type": "string" } }, "required": ["city"] }),
        );
        assert_eq!(tool.name, "get_weather");
        assert_eq!(tool.description, "Returns current weather");
        assert_eq!(tool.parameters["properties"]["city"]["type"], "string");
    }

    #[test]
    fn tool_no_params_creates_empty_schema() {
        let tool = Tool::no_params("ping", "Pings the server");
        assert_eq!(tool.name, "ping");
        assert_eq!(tool.parameters["type"], "object");
    }

    #[test]
    fn tool_call_response_stores_tool_calls() {
        let resp = ToolCallResponse {
            content: Some("thinking...".to_string()),
            tool_calls: vec![ToolCall {
                id: "call_1".to_string(),
                name: "get_weather".to_string(),
                arguments: json!({ "city": "Berlin" }),
            }],
            model: "gpt-4o".to_string(),
            finish_reason: Some("tool_calls".to_string()),
        };
        assert_eq!(resp.tool_calls.len(), 1);
        assert_eq!(resp.tool_calls[0].name, "get_weather");
    }

    // ── AiError tests ────────────────────────────────────────────────────────

    #[test]
    fn ai_error_unknown_driver_message() {
        let err = AiError::UnknownDriver("cohere".to_string());
        assert!(err.to_string().contains("cohere"));
    }

    #[test]
    fn ai_error_missing_env_var_message() {
        let err = AiError::MissingEnvVar("AI_API_KEY".to_string());
        assert!(err.to_string().contains("AI_API_KEY"));
    }

    #[test]
    fn ai_error_api_error_contains_status() {
        let err = AiError::ApiError { status: 401, message: "Unauthorized".to_string() };
        assert!(err.to_string().contains("401"));
    }

    // ── AiManager::from_env() tests ──────────────────────────────────────────

    #[test]
    fn from_env_returns_error_when_ai_driver_missing() {
        // Ensure neither var is set.
        std::env::remove_var("AI_DRIVER");
        std::env::remove_var("AI_API_KEY");

        let result = AiManager::from_env();
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AiError::MissingEnvVar(_)));
        let msg = err.to_string();
        assert!(msg.contains("AI_DRIVER"));
    }

    #[test]
    fn from_env_returns_error_when_ai_api_key_missing() {
        std::env::set_var("AI_DRIVER", "anthropic");
        std::env::remove_var("AI_API_KEY");

        let result = AiManager::from_env();

        // Clean up before any assertions that could panic.
        std::env::remove_var("AI_DRIVER");

        assert!(result.is_err());
        let err = result.unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("AI_API_KEY"));
    }

    #[test]
    fn from_env_returns_error_for_unknown_driver() {
        std::env::set_var("AI_DRIVER", "unknown_driver_xyz");
        std::env::set_var("AI_API_KEY", "test_key");

        let result = AiManager::from_env();

        std::env::remove_var("AI_DRIVER");
        std::env::remove_var("AI_API_KEY");

        // Without any feature flags compiled in, "unknown_driver_xyz" is always unknown.
        // With feature flags this might succeed for "anthropic"/"openai", but since
        // we used a definitely-invalid name it should always be UnknownDriver.
        assert!(result.is_err());
    }

    // ── EmbeddingRequest tests ───────────────────────────────────────────────

    #[test]
    fn embedding_request_single_has_one_input() {
        let req = EmbeddingRequest::single("text-embedding-3-small", "hello world");
        assert_eq!(req.input.len(), 1);
        assert_eq!(req.input[0], "hello world");
    }

    #[test]
    fn embedding_request_batch_has_multiple_inputs() {
        let req = EmbeddingRequest::batch(
            "text-embedding-3-small",
            vec!["first".to_string(), "second".to_string()],
        );
        assert_eq!(req.input.len(), 2);
    }

    // ── TextResponse tests ───────────────────────────────────────────────────

    #[test]
    fn text_response_new_stores_content_and_model() {
        let resp = TextResponse::new("Hello!", "gpt-4o");
        assert_eq!(resp.content, "Hello!");
        assert_eq!(resp.model, "gpt-4o");
        assert!(resp.prompt_tokens.is_none());
    }
}
