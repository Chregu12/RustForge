//! # rf-mail-facade
//!
//! Laravel-style Mail facade for RustForge

use once_cell::sync::Lazy;
use rf_mail::{MemoryMailer, MailResult, Mailable};
use std::sync::Arc;
use tokio::sync::RwLock;

pub static GLOBAL_MAILER: Lazy<Arc<RwLock<MemoryMailer>>> = Lazy::new(|| {
    Arc::new(RwLock::new(MemoryMailer::new()))
});

pub struct Mail;

impl Mail {
    pub async fn send<M: Mailable>(mailable: M) -> MailResult<()> {
        let mailer = GLOBAL_MAILER.read().await;
        mailable.send(&*mailer).await
    }

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

    pub async fn send<M: Mailable>(self, mailable: M) -> MailResult<()> {
        // Send mail to specific recipient
        let mailer = GLOBAL_MAILER.read().await;
        mailable.send(&*mailer).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mail_to() {
        let mailer = Mail::to("test@example.com");
        assert_eq!(mailer.to, "test@example.com");
    }
}
