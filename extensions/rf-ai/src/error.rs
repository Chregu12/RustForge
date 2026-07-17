//! Error types for the crate.

use thiserror::Error;

/// Errors produced by providers, agents, and (de)serialization.
#[derive(Debug, Error)]
pub enum AiError {
    /// A transport-level error talking to the provider.
    #[error("HTTP error: {0}")]
    Http(String),
    /// The provider returned a non-2xx status.
    #[error("API error {status}: {body}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// Raw response body.
        body: String,
    },
    /// A request or response failed to (de)serialize.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// An agent received a tool call with no registered handler.
    #[error("no handler registered for tool '{0}'")]
    MissingTool(String),
    /// An agent exceeded its configured turn limit.
    #[error("agent exceeded {0} turns")]
    MaxTurns(usize),
}

/// Convenience result alias.
pub type AiResult<T> = Result<T, AiError>;

impl From<reqwest::Error> for AiError {
    fn from(e: reqwest::Error) -> Self {
        AiError::Http(e.to_string())
    }
}

impl From<serde_json::Error> for AiError {
    fn from(e: serde_json::Error) -> Self {
        AiError::Serialization(e.to_string())
    }
}
