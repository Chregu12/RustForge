//! # Mail Driver Manager
//!
//! Provides a unified [`MailManager`] that selects and constructs the appropriate
//! mail backend based on configuration or environment variables.
//!
//! ## Environment Variables
//!
//! | Variable              | Default          | Description                               |
//! |-----------------------|------------------|-------------------------------------------|
//! | `MAIL_DRIVER`         | `memory`         | `smtp`, `sendmail`, `log`, `memory`,      |
//! |                       |                  | `mailgun`, `ses`, `sendgrid`, `postmark`  |
//! | `SMTP_HOST`           | `127.0.0.1`      | SMTP server hostname                      |
//! | `SMTP_PORT`           | `587`            | SMTP server port                          |
//! | `SMTP_USERNAME`       | `""`             | SMTP username                             |
//! | `SMTP_PASSWORD`       | `""`             | SMTP password                             |
//! | `SMTP_FROM_ADDRESS`   | `""`             | Default from address for SMTP             |
//! | `SENDMAIL_PATH`       | (system default) | Path to sendmail binary                   |
//! | `MAIL_LOG_PATH`       | `mail.log`       | File path for log driver                  |
//! | `MAILGUN_API_KEY`     | –                | Mailgun API key                           |
//! | `MAILGUN_DOMAIN`      | –                | Mailgun sending domain                    |
//! | `MAILGUN_REGION`      | `us`             | `us` or `eu`                             |
//! | `AWS_REGION`          | `us-east-1`      | AWS region for SES                        |
//! | `AWS_ACCESS_KEY_ID`   | –                | AWS access key for SES                    |
//! | `AWS_SECRET_ACCESS_KEY` | –              | AWS secret key for SES                    |
//! | `SENDGRID_API_KEY`    | –                | SendGrid API key                          |
//! | `POSTMARK_API_TOKEN`  | –                | Postmark server API token                 |
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_mail::manager::MailManager;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Build from environment variables (async because SMTP setup is async)
//! let manager = MailManager::from_env().await?;
//!
//! // Or build an in-memory manager for testing
//! let manager = MailManager::memory();
//!
//! use rf_mail::Mailer;
//! use rf_mail::{Mail, MailBody};
//! use rf_mail::Address;
//! let mail = Mail {
//!     from: Address::new("noreply@example.com"),
//!     to: vec![Address::new("user@example.com")],
//!     subject: "Hello".into(),
//!     body: MailBody::Text("Hi!".into()),
//!     ..Default::default()
//! };
//! manager.send(mail).await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    backends::{LogMailer, MemoryMailer, MockMailer, SendmailMailer, SmtpConfig, SmtpMailer},
    mail::Mail,
    MailError, Mailer,
};
use async_trait::async_trait;
use std::env;
use std::sync::Arc;

/// Unified mail manager that delegates to the configured backend driver.
///
/// All variant-specific logic is type-erased behind `Arc<dyn Mailer>` so that
/// callers only need to interact with the [`Mailer`] interface.
pub struct MailManager {
    inner: Arc<dyn Mailer>,
}

impl MailManager {
    // ── Direct constructors ──────────────────────────────────────────────────

    /// Create a manager wrapping an already-constructed [`Mailer`].
    pub fn new(mailer: impl Mailer + 'static) -> Self {
        Self {
            inner: Arc::new(mailer),
        }
    }

    /// Create a manager using the in-memory backend (useful for testing).
    pub fn memory() -> Self {
        Self::new(MemoryMailer::new())
    }

    /// Create a manager using the mock backend.
    pub fn mock() -> Self {
        Self::new(MockMailer::new())
    }

    /// Create a manager using the log-to-file backend.
    pub fn log(path: impl Into<String>) -> Self {
        Self::new(LogMailer::new(path.into()))
    }

    /// Create a manager using the sendmail backend with the system default path.
    pub fn sendmail() -> Result<Self, MailError> {
        let m = SendmailMailer::new()
            .map_err(|e| MailError::ConfigError(e.to_string()))?;
        Ok(Self::new(m))
    }

    /// Create a manager using the sendmail backend with a custom binary path.
    pub fn sendmail_with_path(path: impl Into<String>) -> Result<Self, MailError> {
        let m = SendmailMailer::with_path(path)
            .map_err(|e| MailError::ConfigError(e.to_string()))?;
        Ok(Self::new(m))
    }

    /// Create a manager using the SMTP backend (async because of connection setup).
    pub async fn smtp(config: SmtpConfig) -> Result<Self, MailError> {
        let m = SmtpMailer::new(config).await?;
        Ok(Self::new(m))
    }

    /// Create a manager using the Mailgun backend.
    #[cfg(feature = "mailgun")]
    pub fn mailgun(config: crate::backends::MailgunConfig) -> Self {
        Self::new(crate::backends::MailgunMailer::new(config))
    }

    /// Create a manager using the Amazon SES backend.
    #[cfg(feature = "ses")]
    pub fn ses(config: crate::backends::SesConfig) -> Self {
        Self::new(crate::backends::SesMailer::new(config))
    }

    /// Create a manager using the SendGrid backend.
    #[cfg(feature = "sendgrid")]
    pub fn sendgrid(config: crate::backends::SendGridConfig) -> Self {
        Self::new(crate::backends::SendGridMailer::new(config))
    }

    /// Create a manager using the Postmark backend.
    #[cfg(feature = "postmark")]
    pub fn postmark(config: crate::backends::PostmarkConfig) -> Self {
        Self::new(crate::backends::PostmarkMailer::new(config))
    }

    // ── Environment-based factory ────────────────────────────────────────────

    /// Build a [`MailManager`] by reading `MAIL_DRIVER` and related environment
    /// variables.  Falls back to the **memory** driver when `MAIL_DRIVER` is not
    /// set.
    ///
    /// This is an `async fn` because the SMTP driver requires an async connection.
    pub async fn from_env() -> Result<Self, MailError> {
        let driver_name = env::var("MAIL_DRIVER").unwrap_or_else(|_| "memory".into());

        match driver_name.to_lowercase().as_str() {
            "smtp" => {
                let host = env::var("SMTP_HOST").unwrap_or_else(|_| "127.0.0.1".into());
                let port = env::var("SMTP_PORT")
                    .unwrap_or_else(|_| "587".into())
                    .parse::<u16>()
                    .map_err(|_| MailError::ConfigError("SMTP_PORT must be a number".into()))?;
                let username = env::var("SMTP_USERNAME").unwrap_or_default();
                let password = env::var("SMTP_PASSWORD").unwrap_or_default();
                let from_address = env::var("SMTP_FROM_ADDRESS").unwrap_or_default();

                Self::smtp(SmtpConfig {
                    host,
                    port,
                    username,
                    password,
                    from_address,
                    from_name: env::var("MAIL_FROM_NAME").ok(),
                })
                .await
            }

            "sendmail" => {
                let path = env::var("SENDMAIL_PATH");
                match path {
                    Ok(p) => Self::sendmail_with_path(p),
                    Err(_) => Self::sendmail(),
                }
            }

            "log" => {
                let path = env::var("MAIL_LOG_PATH").unwrap_or_else(|_| "mail.log".into());
                Ok(Self::log(path))
            }

            #[cfg(feature = "mailgun")]
            "mailgun" => {
                let api_key = env::var("MAILGUN_API_KEY").map_err(|_| {
                    MailError::ConfigError("MAILGUN_API_KEY is required for the mailgun driver".into())
                })?;
                let domain = env::var("MAILGUN_DOMAIN").map_err(|_| {
                    MailError::ConfigError("MAILGUN_DOMAIN is required for the mailgun driver".into())
                })?;
                let region = match env::var("MAILGUN_REGION")
                    .unwrap_or_else(|_| "us".into())
                    .to_lowercase()
                    .as_str()
                {
                    "eu" => crate::backends::MailgunRegion::EU,
                    _ => crate::backends::MailgunRegion::US,
                };
                Ok(Self::mailgun(crate::backends::MailgunConfig {
                    api_key,
                    domain,
                    region,
                    ..Default::default()
                }))
            }

            #[cfg(feature = "ses")]
            "ses" => {
                let region = env::var("AWS_REGION").unwrap_or_else(|_| "us-east-1".into());
                let access_key_id = env::var("AWS_ACCESS_KEY_ID").unwrap_or_default();
                let secret_access_key = env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_default();
                Ok(Self::ses(crate::backends::SesConfig {
                    region,
                    access_key_id,
                    secret_access_key,
                    ..Default::default()
                }))
            }

            #[cfg(feature = "sendgrid")]
            "sendgrid" => {
                let api_key = env::var("SENDGRID_API_KEY").map_err(|_| {
                    MailError::ConfigError(
                        "SENDGRID_API_KEY is required for the sendgrid driver".into(),
                    )
                })?;
                Ok(Self::sendgrid(crate::backends::SendGridConfig {
                    api_key,
                    ..Default::default()
                }))
            }

            #[cfg(feature = "postmark")]
            "postmark" => {
                let server_token = env::var("POSTMARK_API_TOKEN").map_err(|_| {
                    MailError::ConfigError(
                        "POSTMARK_API_TOKEN is required for the postmark driver".into(),
                    )
                })?;
                Ok(Self::postmark(crate::backends::PostmarkConfig {
                    server_token,
                    ..Default::default()
                }))
            }

            _ /* "memory" and anything unrecognised */ => Ok(Self::memory()),
        }
    }
}

#[async_trait]
impl Mailer for MailManager {
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        self.inner.send(mail).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_manager_memory() {
        let _m = MailManager::memory();
    }

    #[test]
    fn test_manager_log() {
        let _m = MailManager::log("/tmp/mail.log");
    }

    #[tokio::test]
    async fn test_manager_from_env_defaults_to_memory() {
        unsafe { env::remove_var("MAIL_DRIVER") };
        let _m = MailManager::from_env().await.unwrap();
    }

    #[tokio::test]
    async fn test_manager_from_env_log_driver() {
        unsafe {
            env::set_var("MAIL_DRIVER", "log");
            env::set_var("MAIL_LOG_PATH", "/tmp/test.log");
        }
        let _m = MailManager::from_env().await.unwrap();
        unsafe {
            env::remove_var("MAIL_DRIVER");
            env::remove_var("MAIL_LOG_PATH");
        }
    }
}
