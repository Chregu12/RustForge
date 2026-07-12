//! Email and notification system for RustForge
//!
//! # Features
//!
//! - Multiple backend support (SMTP, Sendmail, Log, Memory, Mock)
//! - Mailable trait for reusable email types
//! - Template rendering with Handlebars
//! - Markdown support with custom components (@button, @panel, @table)
//! - Queue integration for background sending
//! - Testing utilities with MailFake
//! - Common email types (Welcome, Password Reset, Order Shipped, Invoice)
//!
//! # Quick Start with Mailable
//!
//! ```
//! use rf_mail::{Mailable, MailBuilder, Address, MemoryMailer, Mailer};
//!
//! struct WelcomeMail {
//!     to: String,
//!     name: String,
//! }
//!
//! impl Mailable for WelcomeMail {
//!     fn build(&self) -> MailBuilder {
//!         MailBuilder::new()
//!             .from(Address::new("noreply@example.com"))
//!             .to(Address::new(&self.to))
//!             .subject("Welcome!")
//!             .text(format!("Welcome, {}!", self.name))
//!     }
//! }
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mailer = MemoryMailer::new();
//! let mail = WelcomeMail { to: "user@example.com".into(), name: "Alice".into() };
//!
//! mail.send(&mailer).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Using Markdown
//!
//! ```
//! use rf_mail::{MailBuilder, Address};
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mail = MailBuilder::new()
//!     .from(Address::new("sender@example.com"))
//!     .to(Address::new("recipient@example.com"))
//!     .subject("Markdown Email")
//!     .markdown(r#"
//! # Hello
//!
//! This is **markdown** with custom components:
//!
//! @button(https://example.com)
//! Click Here
//! @endbutton
//!     "#)
//!     .build()?;
//! # Ok(())
//! # }
//! ```
//!
//! # Testing
//!
//! ```
//! use rf_mail::testing::{fake, assert_sent, restore};
//!
//! # async fn example() {
//! fake(); // Enable mail faking
//!
//! // Send emails...
//!
//! assert_sent(|mail| mail.subject.contains("Welcome"));
//!
//! restore(); // Disable faking
//! # }
//! ```

mod address;
mod attachment;
mod backends;
pub mod bridge;
mod builder;
mod config;
mod error;
pub mod facade;
pub mod manager;
mod mail;
mod mail_builder;
pub mod mailables;
mod mailer;
pub mod markdown;
mod message;
pub mod queue;
pub mod templates;
pub mod testing;

// Legacy compatibility (commented out to avoid re-export error)
// pub use builder as message_builder;

// Re-exports
pub use address::Address;
pub use attachment::Attachment;
pub use backends::{
    FileMailer, LogMailer, MemoryMailer, MockMailer, SendmailMailer, SmtpConfig, SmtpMailer,
};

// Phase 19: Production mail drivers
#[cfg(feature = "postmark")]
pub use backends::{PostmarkConfig, PostmarkMailer};

#[cfg(feature = "mailgun")]
pub use backends::{MailgunConfig, MailgunMailer, MailgunRegion};

#[cfg(feature = "sendgrid")]
pub use backends::{SendGridConfig, SendGridMailer};

#[cfg(feature = "ses")]
pub use backends::{SesConfig, SesMailer};

pub use bridge::{AsyncBridge, BridgedMailer, BridgedSmtpMailer};
pub use builder::MessageBuilder;
pub use config::{Encryption, MailConfig, MailDriver, SendmailConfig, SmtpEnvConfig};
// `SmtpMailConfig` is a deprecated alias for `SmtpEnvConfig`. Re-exported here
// for backward compatibility; prefer `SmtpEnvConfig` in new code.
#[allow(deprecated)]
pub use config::SmtpMailConfig;
pub use error::{MailError, MailResult};
pub use mail::{Mail, MailBody};
pub use mail_builder::MailBuilder;
pub use mailable::{Mailable, MailableAsync};
pub use mailables::{InvoiceMail, OrderShippedMail, PasswordResetEmail, WelcomeEmail};
pub use mailer::Mailer;
pub use message::Message;
pub use templates::TemplateEngine;

// Facade re-exports (Laravel-style static API)
pub use facade::{Mail as MailFacade, Mailer as FacadeMailer, GLOBAL_MAILER};

// Queue re-export: available as `rf_mail::MailQueue` regardless of whether the
// `queue` feature is enabled (the non-feature stub is a no-op with helpful errors).
pub use queue::MailQueue;

// Mailable trait module
mod mailable;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Address, Attachment, Mail, MailBody, MailBuilder, MailConfig, MailDriver, MailError,
        MailResult, Mailable, MailableAsync, Mailer, Message, MessageBuilder, TemplateEngine,
    };

    // Queue
    pub use crate::queue::MailQueue;

    // Backends
    pub use crate::{FileMailer, LogMailer, MemoryMailer, MockMailer, SendmailMailer, SmtpMailer};

    // Phase 19: Production mail drivers
    #[cfg(feature = "postmark")]
    pub use crate::{PostmarkConfig, PostmarkMailer};

    #[cfg(feature = "mailgun")]
    pub use crate::{MailgunConfig, MailgunMailer, MailgunRegion};

    #[cfg(feature = "sendgrid")]
    pub use crate::{SendGridConfig, SendGridMailer};

    #[cfg(feature = "ses")]
    pub use crate::{SesConfig, SesMailer};

    // Config
    pub use crate::{Encryption, SendmailConfig, SmtpEnvConfig};
    // Deprecated — use SmtpEnvConfig for application-level config or SmtpConfig for the facade/mailer.
    #[allow(deprecated)]
    pub use crate::SmtpMailConfig;

    // Common mailables
    pub use crate::mailables::{InvoiceMail, OrderShippedMail, PasswordResetEmail, WelcomeEmail};

    // Markdown helpers
    pub use crate::markdown::{button, panel, render_markdown, table};

    // Testing
    pub use crate::testing::{assert_not_sent, assert_sent, assert_sent_count, fake, restore};
}
