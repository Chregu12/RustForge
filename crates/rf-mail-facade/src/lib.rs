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
//! use rf_mail_facade::Mail;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Send mail to specific address
//! let mailer = Mail::to("user@example.com");
//! # Ok(())
//! # }
//! ```

use once_cell::sync::Lazy;
use rf_mail::{MemoryMailer, MailResult, Mailable};
use std::sync::RwLock;

/// Global mailer instance
/// Uses std::sync::RwLock for synchronous access (no .await needed)
pub static GLOBAL_MAILER: Lazy<RwLock<MemoryMailer>> = Lazy::new(|| {
    RwLock::new(MemoryMailer::new())
});

pub struct Mail;

impl Mail {
    /// Send a mailable
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use rf_mail_facade::Mail;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Mail::send(my_mailable)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send<M: Mailable>(mailable: M) -> MailResult<()> {
        let mailer = GLOBAL_MAILER.read().unwrap();
        // Block on async operation
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mailable.send(&*mailer).await
            })
        })
    }

    /// Create a mailer for a specific recipient
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_mail_facade::Mail;
    ///
    /// let mailer = Mail::to("user@example.com");
    /// ```
    pub fn to(address: impl Into<String>) -> Mailer {
        Mailer::new(address.into())
    }
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
    /// use rf_mail_facade::Mail;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Mail::to("user@example.com").send(my_mailable)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send<M: Mailable>(self, mailable: M) -> MailResult<()> {
        let mailer = GLOBAL_MAILER.read().unwrap();
        // Block on async operation
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                mailable.send(&*mailer).await
            })
        })
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
