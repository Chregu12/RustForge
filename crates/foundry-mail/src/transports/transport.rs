use crate::domain::Message;
use async_trait::async_trait;

/// Mail transport result
pub type TransportResult = Result<TransportResponse, TransportError>;

/// Transport response
#[derive(Debug, Clone)]
pub struct TransportResponse {
    pub message_id: String,
    pub accepted: Vec<String>,
    pub rejected: Vec<String>,
}

/// Mail transport trait
#[async_trait]
pub trait MailTransport: Send + Sync {
    /// Send an email message
    async fn send(&self, message: &Message) -> TransportResult;

    /// Test the transport connection
    async fn test_connection(&self) -> Result<(), TransportError>;
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("SMTP error: {0}")]
    Smtp(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Invalid recipient: {0}")]
    InvalidRecipient(String),

    #[error("Message too large: {0} bytes (limit: {1})")]
    MessageTooLarge(usize, usize),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Transport error: {0}")]
    Other(String),
}
