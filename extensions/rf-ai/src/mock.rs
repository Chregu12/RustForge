//! Deterministic, network-free providers for tests and offline demos.

use std::sync::Mutex;

use async_trait::async_trait;

use crate::error::AiResult;
use crate::message::{ContentBlock, Role};
use crate::provider::{ChatProvider, EmbeddingProvider};
use crate::request::ChatRequest;
use crate::response::{ChatResponse, Usage};

/// A [`ChatProvider`] that replays a queue of canned responses.
///
/// Each call to [`ChatProvider::chat`] returns the next queued response. When
/// only one response is queued it is cloned and returned for every call;
/// otherwise responses are popped in order and the last one is repeated once the
/// queue is exhausted.
pub struct MockChatProvider {
    responses: Mutex<Vec<ChatResponse>>,
    cursor: Mutex<usize>,
}

impl MockChatProvider {
    /// Build a provider from a list of responses returned in order.
    pub fn new(responses: Vec<ChatResponse>) -> Self {
        MockChatProvider {
            responses: Mutex::new(responses),
            cursor: Mutex::new(0),
        }
    }

    /// A provider that always returns a single `end_turn` text response.
    pub fn text(reply: &str) -> Self {
        MockChatProvider::new(vec![text_response(reply)])
    }
}

/// Build an `end_turn` [`ChatResponse`] with a single text block.
pub fn text_response(reply: &str) -> ChatResponse {
    ChatResponse {
        id: "msg_mock".to_string(),
        model: "mock".to_string(),
        role: Role::Assistant,
        stop_reason: Some("end_turn".to_string()),
        content: vec![ContentBlock::text(reply)],
        usage: Usage::default(),
    }
}

#[async_trait]
impl ChatProvider for MockChatProvider {
    async fn chat(&self, _request: &ChatRequest) -> AiResult<ChatResponse> {
        let responses = self.responses.lock().expect("mock mutex poisoned");
        let mut cursor = self.cursor.lock().expect("mock mutex poisoned");

        if responses.is_empty() {
            return Ok(text_response(""));
        }

        let idx = (*cursor).min(responses.len() - 1);
        *cursor += 1;
        Ok(responses[idx].clone())
    }
}

/// A deterministic [`EmbeddingProvider`] that hashes text into fixed-size vectors.
///
/// The same input always yields the same embedding, so semantic-search demos are
/// reproducible offline.
pub struct MockEmbeddingProvider {
    dim: usize,
}

impl MockEmbeddingProvider {
    /// Build a provider that produces `dim`-dimensional embeddings.
    pub fn new(dim: usize) -> Self {
        MockEmbeddingProvider { dim }
    }

    fn embed_one(&self, text: &str) -> Vec<f32> {
        // A tiny deterministic hash-to-vector: each dimension folds the bytes
        // with a per-dimension salt, then maps into [-1, 1].
        let bytes = text.as_bytes();
        let mut out = Vec::with_capacity(self.dim);
        for d in 0..self.dim {
            let mut hash: u64 = 0xcbf2_9ce4_8422_2325 ^ (d as u64).wrapping_mul(0x100_0000_01b3);
            for &b in bytes {
                hash ^= b as u64;
                hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
            }
            // Map the high bits to a float in [-1, 1].
            let frac = (hash >> 40) as f32 / (1u64 << 24) as f32;
            out.push(frac * 2.0 - 1.0);
        }
        out
    }
}

#[async_trait]
impl EmbeddingProvider for MockEmbeddingProvider {
    async fn embed(&self, texts: &[String]) -> AiResult<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| self.embed_one(t)).collect())
    }
}
