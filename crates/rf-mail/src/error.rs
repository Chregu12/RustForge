//! Error types for email operations

use thiserror::Error;

/// Result type for email operations
pub type MailResult<T> = Result<T, MailError>;

/// Error types for email operations
#[derive(Debug, Error)]
pub enum MailError {
    /// Invalid email message
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    /// Failed to send email
    #[error("Send failed: {0}")]
    SendFailed(String),

    /// Template rendering error (render)
    #[error("Template error: {0}")]
    TemplateRenderError(#[from] handlebars::RenderError),

    /// Template registration error
    #[error("Template registration error: {0}")]
    TemplateError(#[from] handlebars::TemplateError),

    /// SMTP transport error
    #[error("SMTP error: {0}")]
    SmtpError(#[from] lettre::error::Error),

    /// SMTP transport error (specific)
    #[error("SMTP transport error: {0}")]
    SmtpTransportError(#[from] lettre::transport::smtp::Error),

    /// Email address parse error
    #[error("Address parse error: {0}")]
    AddressError(#[from] lettre::address::AddressError),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

// Implement Send + Sync for compatibility with async traits
unsafe impl Send for MailError {}
unsafe impl Sync for MailError {}
