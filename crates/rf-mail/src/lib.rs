//! Email and notification system for RustForge
//!
//! # Features
//!
//! - Multiple backend support (SMTP, Memory, Mock)
//! - Message builder with fluent API
//! - Mailable trait for reusable email types
//! - Template rendering with Handlebars
//! - Common email types (Welcome, Password Reset)
//! - Testing support with Memory and Mock backends
//!
//! # Quick Start
//!
//! ```
//! use rf_mail::{MemoryMailer, Mailer, MessageBuilder, Address};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mailer = MemoryMailer::new();
//!
//! let message = MessageBuilder::new()
//!     .from(Address::with_name("sender@example.com", "Sender"))
//!     .to(Address::new("recipient@example.com"))
//!     .subject("Hello!")
//!     .html("<h1>Hello, World!</h1>")
//!     .text("Hello, World!")
//!     .build()?;
//!
//! mailer.send(&message).await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Using Mailables
//!
//! ```
//! use rf_mail::{WelcomeEmail, Address, Mailable, MemoryMailer, Mailer};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mailer = MemoryMailer::new();
//!
//! let welcome = WelcomeEmail {
//!     to: Address::with_name("user@example.com", "John Doe"),
//!     user_name: "John".into(),
//!     app_name: "MyApp".into(),
//! };
//!
//! welcome.send(&mailer).await?;
//! # Ok(())
//! # }
//! ```

mod address;
mod attachment;
mod backends;
mod builder;
mod error;
pub mod mailables;
mod mailer;
mod message;
mod templates;

// Re-exports
pub use address::Address;
pub use attachment::Attachment;
pub use backends::{MemoryMailer, MockMailer, SmtpConfig, SmtpMailer};
pub use builder::MessageBuilder;
pub use error::{MailError, MailResult};
pub use mailables::{PasswordResetEmail, WelcomeEmail};
pub use mailer::{Mailable, Mailer};
pub use message::Message;
pub use templates::TemplateEngine;
