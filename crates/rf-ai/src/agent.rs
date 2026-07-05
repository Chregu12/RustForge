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

    /// Begin a fluent, single-prompt run: `agent.prompt("..").run().await`.
    ///
    /// Returns a [`PromptRun`] borrowing this agent. The returned builder can
    /// carry a per-call [system prompt](PromptRun::system) override before being
    /// awaited via [`PromptRun::run`]. This is the ergonomic entry point for the
    /// vision surface; it drives the exact same tool-calling loop as [`Agent::run`].
    pub fn prompt(&self, user_message: impl Into<String>) -> PromptRun<'_, P> {
        PromptRun {
            agent: self,
            user_message: user_message.into(),
            system: None,
            attachments: Vec::new(),
        }
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
        self.run_inner(Message::user(user_message.into()), self.system.clone())
            .await
    }

    /// The shared tool-calling loop backing both [`Agent::run`] and
    /// [`PromptRun::run`]. `first_message` is the initial user turn (which may
    /// carry attachment blocks) and `system` is the effective system prompt.
    async fn run_inner(&self, first_message: Message, system: Option<String>) -> AiResult<String> {
        let mut messages = vec![first_message];

        for _ in 0..self.max_turns {
            let mut request = ChatRequest::new(&self.model)
                .messages(messages.clone())
                .tools(self.tools.clone());
            if let Some(system) = &system {
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

/// A fluent, single-prompt run produced by [`Agent::prompt`].
///
/// Holds the user message (and an optional per-call system override) and borrows
/// the originating [`Agent`]. Await [`PromptRun::run`] to execute the full
/// tool-calling loop and obtain the final text answer.
///
/// ```rust
/// use rf_ai::prelude::*;
/// use rf_ai::mock::MockChatProvider;
///
/// # fn main() -> AiResult<()> {
/// let agent = Agent::new(MockChatProvider::text("Paris."));
/// let answer = futures::executor::block_on(
///     agent.prompt("Capital of France?").run(),
/// )?;
/// assert_eq!(answer, "Paris.");
/// # Ok(())
/// # }
/// ```
pub struct PromptRun<'a, P: ChatProvider> {
    agent: &'a Agent<P>,
    user_message: String,
    system: Option<String>,
    attachments: Vec<ContentBlock>,
}

impl<'a, P: ChatProvider> PromptRun<'a, P> {
    /// Override the system prompt for just this run (does not mutate the agent).
    pub fn system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(system.into());
        self
    }

    /// Attach an image from raw bytes (base64-encoded into the message).
    ///
    /// `media_type` is the image MIME type, e.g. `image/png`. This is the vision
    /// surface: `agent.prompt("..").image("image/png", bytes).run()`.
    pub fn image(mut self, media_type: impl Into<String>, bytes: impl AsRef<[u8]>) -> Self {
        self.attachments
            .push(ContentBlock::image(media_type, bytes));
        self
    }

    /// Attach a base64 document such as a PDF, from raw bytes.
    ///
    /// `media_type` is the document MIME type, typically `application/pdf`.
    pub fn document(mut self, media_type: impl Into<String>, bytes: impl AsRef<[u8]>) -> Self {
        self.attachments
            .push(ContentBlock::document(media_type, bytes));
        self
    }

    /// Attach an inline plain-text document (`text/plain`).
    pub fn text_document(mut self, text: impl Into<String>) -> Self {
        self.attachments.push(ContentBlock::text_document(text));
        self
    }

    /// Append an already-built attachment [`ContentBlock`] (e.g. one produced by
    /// [`ContentBlock::image_base64`] or [`ContentBlock::document_base64`]).
    pub fn attachment(mut self, block: ContentBlock) -> Self {
        self.attachments.push(block);
        self
    }

    /// Execute the tool-calling loop and return the final text answer.
    ///
    /// The initial user turn carries any attachments first, followed by the text
    /// prompt — the order Anthropic recommends for image/document inputs.
    pub async fn run(self) -> AiResult<String> {
        let system = self.system.or_else(|| self.agent.system.clone());
        let mut content = self.attachments;
        content.push(ContentBlock::text(self.user_message));
        let first_message = Message::with_blocks(Role::User, content);
        self.agent.run_inner(first_message, system).await
    }
}
