//! # rf-mail-facade
//!
//! Laravel-style Mail facade for RustForge
//!
//! ## Features
//!
//! - **Static Mail API**: Use `Mail::send()`, `Mail::to()`, etc. - no `.await` needed!
//! - **Global Mailer**: Thread-safe global mail state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_mail::facade::Mail;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Send mail to specific address
//! let mailer = Mail::to("user@example.com");
//! # Ok(())
//! # }
//! ```

use once_cell::sync::Lazy;
use crate::{FileMailer, MemoryMailer, MailResult, Mailable};
use std::path::PathBuf;
use std::sync::RwLock;

/// Global in-memory mailer instance (kept for backwards-compatible inspection).
///
/// Uses `std::sync::RwLock` for synchronous access (no `.await` needed).
pub static GLOBAL_MAILER: Lazy<RwLock<MemoryMailer>> = Lazy::new(|| {
    RwLock::new(MemoryMailer::new())
});

/// Default mailbox directory used by the file transport when `MAIL_MAILBOX`
/// is not set.
fn default_mailbox() -> PathBuf {
    if let Ok(dir) = std::env::var("MAIL_MAILBOX") {
        PathBuf::from(dir)
    } else {
        std::env::temp_dir().join("rustforge-mailbox")
    }
}

/// Global filesystem transport backing the synchronous `Mail` facade.
///
/// This is the real default transport: `Mail::to(..).send(..)` writes each
/// message to disk as an `.eml` file. Delivery is fully synchronous, so it works
/// with or without a Tokio runtime.
pub static GLOBAL_FILE_MAILER: Lazy<RwLock<FileMailer>> =
    Lazy::new(|| RwLock::new(FileMailer::new(default_mailbox())));

pub struct Mail;

impl Mail {
    /// Send a mailable
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::Mail;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Mail::send(my_mailable)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send<M: Mailable>(mailable: M) -> MailResult<()> {
        deliver(mailable, None)
    }

    /// Create a mailer for a specific recipient
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_mail::facade::Mail;
    ///
    /// let mailer = Mail::to("user@example.com");
    /// ```
    pub fn to(address: impl Into<String>) -> Mailer {
        Mailer::new(address.into())
    }

    /// Configure the mailbox directory used by the default file transport.
    ///
    /// Every subsequent `Mail::to(..).send(..)` writes `.eml` files here.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_mail::facade::Mail;
    ///
    /// Mail::mailbox("/var/mail/rustforge");
    /// ```
    pub fn mailbox(dir: impl Into<PathBuf>) {
        *GLOBAL_FILE_MAILER.write().unwrap() = FileMailer::new(dir.into());
    }

    /// Get the current mailbox directory used by the default file transport.
    pub fn mailbox_path() -> PathBuf {
        GLOBAL_FILE_MAILER.read().unwrap().mailbox().to_path_buf()
    }
}

/// Build a mailable synchronously and deliver it through the real default
/// transport.
///
/// When mail faking is enabled (see [`crate::testing::fake`]), the message is
/// recorded there instead of being written to disk, mirroring Laravel's
/// `Mail::fake()`. Otherwise the message is written as an `.eml` file via the
/// global [`FileMailer`]. Delivery is synchronous and never blocks on an async
/// runtime.
fn deliver<M: Mailable>(mailable: M, to_override: Option<&str>) -> MailResult<()> {
    let mut mail = mailable.build().build()?;

    // `Mail::to(addr)` acts as a recipient override only when the mailable did
    // not specify its own recipients, preserving prior facade behavior.
    if let Some(addr) = to_override {
        if mail.to.is_empty() {
            mail.to.push(crate::Address::new(addr));
        }
    }

    if let Some(fake) = crate::testing::get_fake() {
        fake.record(mail);
        return Ok(());
    }

    let mailer = GLOBAL_FILE_MAILER.read().unwrap();
    mailer.deliver(&mail)?;
    Ok(())
}

pub struct Mailer {
    pub to: String,
}

impl Mailer {
    pub fn new(to: String) -> Self {
        Self { to }
    }

    /// Send a mailable to this recipient
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::Mail;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Mail::to("user@example.com").send(my_mailable)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send<M: Mailable>(self, mailable: M) -> MailResult<()> {
        deliver(mailable, Some(&self.to))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_to() {
        let mailer = Mail::to("test@example.com");
        assert_eq!(mailer.to, "test@example.com");
    }
}
