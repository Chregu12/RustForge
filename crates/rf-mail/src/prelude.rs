//! # rf-mail Prelude
//!
//! This prelude module re-exports the most commonly used types and traits from rf-mail.
//!
//! ## Usage
//!
//! ```rust
//! use rf_mail::prelude::*;
//! ```

// Re-export commonly used items
pub use crate:: address::Address;
pub use crate:: attachment::Attachment;
pub use crate:: backends::{LogMailer, MemoryMailer, MockMailer, SendmailMailer, SmtpConfig, SmtpMailer};
pub use crate:: backends::{PostmarkConfig, PostmarkMailer};
pub use crate:: backends::{MailgunConfig, MailgunMailer, MailgunRegion};
pub use crate:: backends::{SendGridConfig, SendGridMailer};
pub use crate:: backends::{SesConfig, SesMailer};
pub use crate:: builder::MessageBuilder;
pub use crate::config::{Encryption, MailConfig, MailDriver, SendmailConfig, SmtpEnvConfig};
// Deprecated alias kept for backward compatibility — prefer SmtpEnvConfig.
#[allow(deprecated)]
pub use crate::SmtpMailConfig;
pub use crate:: error::{MailError, MailResult};
