//! A tool-calling agent loop over any [`ChatProvider`].

use std::collections::HashMap;

use crate::error::{AiError, AiResult};
use crate::message::{ContentBlock, Message, Role};
use crate::provider::ChatProvider;
use crate::request::{ChatRequest, DEFAULT_MODEL};
use crate::tool::Tool;

/// A function that executes a tool, given its JSON input, returning a string.
pub type ToolHandler = Box<dyn Fn(serde_json::Value) -> AiResult<String> + Send + Sync>;

/// Default maximum number of provider round-trips per [`Agent::run`].
const DEFAULT_MAX_TURNS: usize = 8;

/// An agent that drives a tool-calling loop against a [`ChatProvider`].
///
/// Register tools with [`Agent::tool`]; each registration adds both the schema
/// (sent to the model) and the executor (run when the model calls it). When the
/// model stops to call tools, the agent executes the handlers, feeds the results
/// back, and continues until the model produces a final text answer or the turn
/// limit is reached.
pub struct Agent<P: ChatProvider> {
    provider: P,
    model: String,
    system: Option<String>,
    tools: Vec<Tool>,
    handlers: HashMap<String, ToolHandler>,
    max_turns: usize,
}

impl<P: ChatProvider> Agent<P> {
    /// Create an agent over `provider` using [`DEFAULT_MODEL`].
    pub fn new(provider: P) -> Self {
        Agent {
            provider,
            model: DEFAULT_MODEL.to_string(),
            system: None,
            tools: Vec::new(),
            handlers: HashMap::new(),
            max_turns: DEFAULT_MAX_TURNS,
        }
    }

    /// Set the model id.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Set the system prompt.
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Register a tool: its schema is advertised to the model and `handler`
    /// executes it when called.
    pub fn tool<F>(mut self, tool: Tool, handler: F) -> Self
    where
        F: Fn(serde_json::Value) -> AiResult<String> + Send + Sync + 'static,
    {
        self.handlers.insert(tool.name.clone(), Box::new(handler));
        self.tools.push(tool);
        self
    }

    /// Set the maximum number of provider round-trips.
    pub fn max_turns(mut self, max_turns: usize) -> Self {
        self.max_turns = max_turns;
        self
    }

    /// Run the loop with an initial user message and return the final text.
    ///
    /// On each turn the agent sends the conversation to the provider. If the
    /// response stops for tool use, every [`ContentBlock::ToolUse`] is dispatched
    /// to its registered handler — a missing handler yields [`AiError::MissingTool`]
    /// — and the assistant turn plus the tool results are appended before looping.
    /// If the loop runs `max_turns` times without a final answer, it returns
    /// [`AiError::MaxTurns`].
    pub async fn run(&self, user_message: impl Into<String>) -> AiResult<String> {
        let mut messages = vec![Message::user(user_message)];

        for _ in 0..self.max_turns {
            let mut request = ChatRequest::new(&self.model)
                .messages(messages.clone())
                .tools(self.tools.clone());
            if let Some(system) = &self.system {
                request = request.system(system.clone());
            }

            let response = self.provider.chat(&request).await?;

            if !response.stopped_for_tools() {
                return Ok(response.text());
            }

            // Append the assistant turn verbatim so tool_use ids round-trip.
            messages.push(Message::with_blocks(Role::Assistant, response.content.clone()));

            // Execute each requested tool and collect results.
            let mut results = Vec::new();
            for block in &response.content {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    let handler = self
                        .handlers
                        .get(name)
                        .ok_or_else(|| AiError::MissingTool(name.clone()))?;
                    match handler(input.clone()) {
                        Ok(output) => results.push(ContentBlock::tool_result(id, output)),
                        Err(e) => results.push(ContentBlock::tool_error(id, e.to_string())),
                    }
                }
            }

            messages.push(Message::with_blocks(Role::User, results));
        }

        Err(AiError::MaxTurns(self.max_turns))
    }
}
