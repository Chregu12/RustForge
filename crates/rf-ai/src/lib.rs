//! # rf-ai — a provider-agnostic AI SDK for RustForge
//!
//! A Laravel-13-style, provider-agnostic AI toolkit: text generation,
//! embeddings, and tool-calling agents behind small async traits. Code against
//! [`ChatProvider`] / [`EmbeddingProvider`] and swap the live
//! [`AnthropicProvider`] for a [`MockChatProvider`] in tests without touching
//! your application logic.
//!
//! Rust has no official Anthropic SDK, so [`AnthropicProvider`] speaks the
//! Anthropic Messages API directly over `reqwest`. The default model is
//! [`DEFAULT_MODEL`] (`claude-opus-4-8`); sampling parameters are intentionally
//! omitted by default because the default model rejects them.
//!
//! ## Modules
//!
//! - [`message`]: conversation [`Message`]s and [`ContentBlock`]s.
//! - [`request`] / [`response`]: the chat request/response wire types.
//! - [`tool`]: [`Tool`] definitions and [`ToolChoice`].
//! - [`provider`]: the [`ChatProvider`] / [`EmbeddingProvider`] traits and the
//!   `reqwest`-based [`AnthropicProvider`].
//! - [`agent`]: a tool-calling [`Agent`] loop over any [`ChatProvider`].
//! - [`mock`]: deterministic mock providers for tests.
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_ai::prelude::*;
//!
//! # async fn example() -> AiResult<()> {
//! let provider = AnthropicProvider::new(std::env::var("ANTHROPIC_API_KEY").unwrap());
//!
//! let request = ChatRequest::default_model()
//!     .max_tokens(256)
//!     .system("You are a terse assistant.")
//!     .message(Message::user("Name the capital of France."));
//!
//! let response = provider.chat(&request).await?;
//! println!("{}", response.text());
//! # Ok(())
//! # }
//! ```
//!
//! ## Tool-calling agent (offline, no network)
//!
//! ```rust
//! use rf_ai::prelude::*;
//! use rf_ai::mock::{text_response, MockChatProvider};
//! use rf_ai::response::{ChatResponse, Usage};
//! use serde_json::json;
//!
//! # fn main() -> AiResult<()> {
//! // Script the model to call a tool, then answer.
//! let call = ChatResponse {
//!     id: "1".into(),
//!     model: "mock".into(),
//!     role: Role::Assistant,
//!     stop_reason: Some("tool_use".into()),
//!     content: vec![ContentBlock::ToolUse {
//!         id: "t1".into(),
//!         name: "add".into(),
//!         input: json!({ "a": 2, "b": 3 }),
//!     }],
//!     usage: Usage::default(),
//! };
//! let provider = MockChatProvider::new(vec![call, text_response("The answer is 5.")]);
//!
//! let agent = Agent::new(provider).tool(
//!     Tool::new("add", "Add two numbers", json!({"type": "object"})),
//!     |input| {
//!         let a = input["a"].as_i64().unwrap_or(0);
//!         let b = input["b"].as_i64().unwrap_or(0);
//!         Ok((a + b).to_string())
//!     },
//! );
//!
//! let answer = futures::executor::block_on(agent.run("What is 2 + 3?"))?;
//! assert!(answer.contains("5"));
//! # Ok(())
//! # }
//! ```

pub mod agent;
pub mod error;
pub mod message;
pub mod mock;
pub mod provider;
pub mod request;
pub mod response;
pub mod tool;

pub use agent::{Agent, PromptRun, ToolHandler};
pub use error::{AiError, AiResult};
pub use message::{ContentBlock, Message, Role, Source};
pub use mock::{MockChatProvider, MockEmbeddingProvider};
pub use provider::{AnthropicProvider, ChatProvider, EmbeddingProvider};
pub use request::{ChatRequest, DEFAULT_MODEL};
pub use response::{ChatResponse, Usage};
pub use tool::{Tool, ToolChoice};

/// Common imports for downstream crates.
pub mod prelude {
    pub use crate::{
        Agent, AiError, AiResult, AnthropicProvider, ChatProvider, ChatRequest, ChatResponse,
        ContentBlock, EmbeddingProvider, Message, Role, Source, Tool, ToolChoice,
    };
}

#[cfg(test)]
mod tests {
    use super::mock::{text_response, MockChatProvider, MockEmbeddingProvider};
    use super::prelude::*;
    use super::response::{ChatResponse, Usage};
    use serde_json::json;

    #[test]
    fn chat_request_serializes_to_anthropic_shape() {
        let request = ChatRequest::default_model()
            .max_tokens(512)
            .message(Message::user("Hello"));

        let value = serde_json::to_value(&request).unwrap();

        assert_eq!(value["model"], "claude-opus-4-8");
        assert_eq!(value["max_tokens"], 512);
        assert_eq!(value["messages"][0]["role"], "user");
        assert_eq!(value["messages"][0]["content"][0]["type"], "text");
        assert_eq!(value["messages"][0]["content"][0]["text"], "Hello");
        // Absent optional/empty fields are omitted.
        assert!(value.get("system").is_none());
        assert!(value.get("tools").is_none());
        assert!(value.get("tool_choice").is_none());
    }

    #[test]
    fn chat_request_includes_system_tools_and_choice() {
        let request = ChatRequest::default_model()
            .system("be brief")
            .tool(Tool::new("t", "a tool", json!({"type": "object"})))
            .tool_choice(ToolChoice::Tool("t".into()));

        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["system"], "be brief");
        assert_eq!(value["tools"][0]["name"], "t");
        assert_eq!(value["tool_choice"]["type"], "tool");
        assert_eq!(value["tool_choice"]["name"], "t");
    }

    #[test]
    fn tool_choice_variants_serialize() {
        assert_eq!(
            serde_json::to_value(ToolChoice::Auto).unwrap(),
            json!({"type": "auto"})
        );
        assert_eq!(
            serde_json::to_value(ToolChoice::Any).unwrap(),
            json!({"type": "any"})
        );
    }

    #[test]
    fn image_block_matches_anthropic_wire_shape() {
        // base64 of the three bytes 0x89 'P' 'N' -> "iVBO" (spot-check encoding).
        let block = ContentBlock::image("image/png", [0x89u8, b'P', b'N', b'G']);
        let value = serde_json::to_value(&block).unwrap();

        assert_eq!(value["type"], "image");
        assert_eq!(value["source"]["type"], "base64");
        assert_eq!(value["source"]["media_type"], "image/png");
        // The data field is a real base64 encoding of the input bytes.
        assert_eq!(value["source"]["data"], "iVBORw==");

        // Exact-shape round-trip through JSON.
        let s = serde_json::to_string(&block).unwrap();
        let back: ContentBlock = serde_json::from_str(&s).unwrap();
        assert_eq!(back, block);
    }

    #[test]
    fn document_blocks_round_trip_base64_and_text() {
        let pdf = ContentBlock::document("application/pdf", b"%PDF-1.7\n");
        let pv = serde_json::to_value(&pdf).unwrap();
        assert_eq!(pv["type"], "document");
        assert_eq!(pv["source"]["type"], "base64");
        assert_eq!(pv["source"]["media_type"], "application/pdf");
        // %PDF-1.7\n base64-encodes to this exact string.
        assert_eq!(pv["source"]["data"], "JVBERi0xLjcK");
        assert_eq!(
            serde_json::from_value::<ContentBlock>(pv).unwrap(),
            pdf,
            "base64 document round-trips",
        );

        let doc = ContentBlock::text_document("Quarterly report: revenue up 10%.");
        let dv = serde_json::to_value(&doc).unwrap();
        assert_eq!(dv["type"], "document");
        assert_eq!(dv["source"]["type"], "text");
        assert_eq!(dv["source"]["media_type"], "text/plain");
        assert_eq!(dv["source"]["data"], "Quarterly report: revenue up 10%.");
        assert_eq!(
            serde_json::from_value::<ContentBlock>(dv).unwrap(),
            doc,
            "plain-text document round-trips",
        );
    }

    #[test]
    fn chat_response_deserializes_and_helpers_work() {
        let payload = json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-8",
            "stop_reason": "tool_use",
            "content": [
                { "type": "text", "text": "Let me check." },
                { "type": "tool_use", "id": "tu_1", "name": "get_weather", "input": { "city": "Paris" } }
            ],
            "usage": { "input_tokens": 10, "output_tokens": 5 }
        });

        let response: ChatResponse = serde_json::from_value(payload).unwrap();

        assert_eq!(response.text(), "Let me check.");
        assert_eq!(response.tool_uses().len(), 1);
        assert!(response.stopped_for_tools());
        assert_eq!(response.usage.input_tokens, 10);
        assert_eq!(response.usage.output_tokens, 5);
        match response.tool_uses()[0] {
            ContentBlock::ToolUse { name, input, .. } => {
                assert_eq!(name, "get_weather");
                assert_eq!(input["city"], "Paris");
            }
            _ => panic!("expected tool use"),
        }
    }

    #[test]
    fn chat_response_tolerates_missing_fields() {
        let payload = json!({ "content": [{ "type": "text", "text": "hi" }] });
        let response: ChatResponse = serde_json::from_value(payload).unwrap();
        assert_eq!(response.text(), "hi");
        assert_eq!(response.role, Role::Assistant);
        assert!(!response.stopped_for_tools());
    }

    #[tokio::test]
    async fn agent_runs_tool_then_returns_final_text() {
        let call = ChatResponse {
            id: "1".into(),
            model: "mock".into(),
            role: Role::Assistant,
            stop_reason: Some("tool_use".into()),
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "add".into(),
                input: json!({ "a": 2, "b": 3 }),
            }],
            usage: Usage::default(),
        };
        let provider = MockChatProvider::new(vec![call, text_response("The total is 5.")]);

        let ran = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ran_clone = ran.clone();

        let agent = Agent::new(provider).tool(
            Tool::new("add", "Add two integers", json!({"type": "object"})),
            move |input| {
                ran_clone.store(true, std::sync::atomic::Ordering::SeqCst);
                let a = input["a"].as_i64().unwrap();
                let b = input["b"].as_i64().unwrap();
                Ok((a + b).to_string())
            },
        );

        let answer = agent.run("add 2 and 3").await.unwrap();
        assert!(ran.load(std::sync::atomic::Ordering::SeqCst), "handler ran");
        assert_eq!(answer, "The total is 5.");
    }

    #[tokio::test]
    async fn agent_errors_on_missing_handler() {
        let call = ChatResponse {
            id: "1".into(),
            model: "mock".into(),
            role: Role::Assistant,
            stop_reason: Some("tool_use".into()),
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "unknown".into(),
                input: json!({}),
            }],
            usage: Usage::default(),
        };
        let provider = MockChatProvider::new(vec![call]);
        let agent = Agent::new(provider);

        let err = agent.run("do it").await.unwrap_err();
        match err {
            AiError::MissingTool(name) => assert_eq!(name, "unknown"),
            other => panic!("expected MissingTool, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn agent_respects_max_turns() {
        // Always asks for a tool — never terminates on its own.
        let looping = ChatResponse {
            id: "1".into(),
            model: "mock".into(),
            role: Role::Assistant,
            stop_reason: Some("tool_use".into()),
            content: vec![ContentBlock::ToolUse {
                id: "t1".into(),
                name: "noop".into(),
                input: json!({}),
            }],
            usage: Usage::default(),
        };
        let provider = MockChatProvider::new(vec![looping]);
        let agent = Agent::new(provider).max_turns(3).tool(
            Tool::new("noop", "does nothing", json!({"type": "object"})),
            |_| Ok("ok".to_string()),
        );

        let err = agent.run("loop").await.unwrap_err();
        match err {
            AiError::MaxTurns(n) => assert_eq!(n, 3),
            other => panic!("expected MaxTurns, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn mock_embedding_is_deterministic_and_sized() {
        let provider = MockEmbeddingProvider::new(8);
        let texts = vec!["hello".to_string(), "world".to_string()];

        let first = provider.embed(&texts).await.unwrap();
        let second = provider.embed(&texts).await.unwrap();

        assert_eq!(first.len(), 2);
        assert_eq!(first[0].len(), 8);
        assert_eq!(first[1].len(), 8);
        assert_eq!(first, second, "embeddings are deterministic");
        assert_ne!(first[0], first[1], "distinct inputs differ");
    }
}
