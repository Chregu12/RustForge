//! Email backend implementations

pub mod log;
pub mod memory;
pub mod mock;
pub mod sendmail;
pub mod smtp;

// Phase 19: Production mail drivers
#[cfg(feature = "postmark")]
pub mod postmark;

#[cfg(feature = "mailgun")]
pub mod mailgun;

#[cfg(feature = "sendgrid")]
pub mod sendgrid;

#[cfg(feature = "ses")]
pub mod ses;

pub use log::LogMailer;
pub use memory::MemoryMailer;
pub use mock::MockMailer;
pub use sendmail::SendmailMailer;
pub use smtp::{SmtpConfig, SmtpMailer};

#[cfg(feature = "postmark")]
pub use postmark::{PostmarkConfig, PostmarkMailer};

#[cfg(feature = "mailgun")]
pub use mailgun::{MailgunConfig, MailgunMailer, MailgunRegion};

#[cfg(feature = "sendgrid")]
pub use sendgrid::{SendGridConfig, SendGridMailer};

#[cfg(feature = "ses")]
pub use ses::{SesConfig, SesMailer};
