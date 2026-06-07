use thiserror::Error;

/// Errors that can occur when interacting with an AI provider.
#[derive(Debug, Error)]
pub enum AiError {
    /// The requested AI driver is not recognised or not compiled in.
    #[error("Unknown AI driver: {0}")]
    UnknownDriver(String),

    /// A required environment variable is missing.
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    /// The request could not be serialised / deserialised.
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// The remote API returned an error response.
    #[error("API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    /// A network / transport error.
    #[error("Network error: {0}")]
    Network(String),

    /// The provider rejected the request due to authentication failure.
    #[error("Authentication failed: {0}")]
    AuthError(String),

    /// The provider enforced a rate limit.
    #[error("Rate limit exceeded: {0}")]
    RateLimited(String),

    /// An AI tool call could not be executed.
    #[error("Tool call error: {0}")]
    ToolCallError(String),

    /// A generic, unclassified error.
    #[error("AI error: {0}")]
    Other(String),
}

/// Convenience alias.
pub type AiResult<T> = Result<T, AiError>;
