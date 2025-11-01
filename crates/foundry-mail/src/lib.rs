//! Foundry Mail - Email delivery system with SMTP, templates, and queue integration
//!
//! This crate provides a comprehensive email system for the Foundry framework.
//!
//! # Features
//!
//! - **Domain Models**: Email addresses, messages, attachments, content
//! - **Templates**: Tera and Handlebars template engines
//! - **Transports**: SMTP with TLS/STARTTLS support
//! - **Mailable**: Laravel-style mailable classes
//! - **Queue Integration**: Send emails via job queue
//!
//! # Example
//!
//! ```no_run
//! use foundry_mail::prelude::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create SMTP configuration
//! let config = SmtpConfig::from_env();
//!
//! // Create transport and mailer
//! let transport = SmtpTransport::new(config)?;
//! let mailer = Mailer::new(Arc::new(transport));
//!
//! // Build and send a message
//! let message = Message::builder()
//!     .from("sender@example.com")
//!     .to("recipient@example.com")
//!     .subject("Hello from Foundry")
//!     .text("This is a test email")
//!     .build()?;
//!
//! mailer.send(&message).await?;
//! # Ok(())
//! # }
//! ```

pub mod domain;
pub mod templates;
pub mod transports;
pub mod mailable;
pub mod mailer;

pub use domain::{Address, AddressList, Attachment, Content, Envelope, Message};
pub use templates::{TemplateEngine, TemplateRenderer, TeraRenderer, HandlebarsRenderer, RenderContext};
pub use transports::{MailTransport, SmtpTransport, SmtpConfig, TransportError};
pub use mailable::{Mailable, MailableEnvelope, MailableError, TemplateMailable};
pub use mailer::{Mailer, MailerError, QueuedMailer};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::domain::{Address, AddressList, Attachment, Content, Message};
    pub use crate::templates::{TemplateEngine, TemplateRenderer, TeraRenderer, HandlebarsRenderer, RenderContext};
    pub use crate::transports::{MailTransport, SmtpTransport, SmtpConfig};
    pub use crate::mailable::{Mailable, MailableEnvelope, TemplateMailable};
    pub use crate::mailer::{Mailer, MailerError};
    pub use std::sync::Arc;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_message() {
        let message = Message::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text("Hello World")
            .build()
            .unwrap();

        assert_eq!(message.envelope.from.email, "sender@example.com");
        assert_eq!(message.envelope.subject, "Test");
    }
}
