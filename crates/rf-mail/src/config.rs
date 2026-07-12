//! Mail configuration and transport settings

use crate::{Address, MailError};
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Main mail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
    /// Mail transport driver
    pub driver: MailDriver,

    /// Default from address
    pub from: Address,

    /// SMTP environment/connection configuration (if using SMTP driver).
    ///
    /// This is the application-level config read from environment variables or a
    /// config file (optional auth, encryption, timeout, STARTTLS). It is **not**
    /// the struct you pass to [`crate::MailFacade::smtp`] — for that use
    /// [`crate::SmtpConfig`] from `rf_mail::backends::smtp`.
    pub smtp: Option<SmtpEnvConfig>,

    /// Sendmail configuration (if using Sendmail driver)
    pub sendmail: Option<SendmailConfig>,

    /// Log file path (if using Log driver)
    pub log_path: Option<String>,
}

impl MailConfig {
    /// Create a new configuration with defaults
    pub fn new(driver: MailDriver, from: Address) -> Self {
        Self {
            driver,
            from,
            smtp: None,
            sendmail: None,
            log_path: None,
        }
    }

    /// Create SMTP configuration
    pub fn smtp(from: Address, smtp: SmtpEnvConfig) -> Self {
        Self {
            driver: MailDriver::Smtp,
            from,
            smtp: Some(smtp),
            sendmail: None,
            log_path: None,
        }
    }

    /// Create Sendmail configuration
    pub fn sendmail(from: Address, sendmail: SendmailConfig) -> Self {
        Self {
            driver: MailDriver::Sendmail,
            from,
            smtp: None,
            sendmail: Some(sendmail),
            log_path: None,
        }
    }

    /// Create Log configuration
    pub fn log(from: Address, log_path: String) -> Self {
        Self {
            driver: MailDriver::Log,
            from,
            smtp: None,
            sendmail: None,
            log_path: Some(log_path),
        }
    }

    /// Create Memory configuration (for testing)
    pub fn memory(from: Address) -> Self {
        Self {
            driver: MailDriver::Memory,
            from,
            smtp: None,
            sendmail: None,
            log_path: None,
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), MailError> {
        match self.driver {
            MailDriver::Smtp if self.smtp.is_none() => Err(MailError::ConfigError(
                "SMTP config is required for SMTP driver".into(),
            )),
            MailDriver::Sendmail if self.sendmail.is_none() => Err(MailError::ConfigError(
                "Sendmail config is required for Sendmail driver".into(),
            )),
            MailDriver::Log if self.log_path.is_none() => Err(MailError::ConfigError(
                "Log path is required for Log driver".into(),
            )),
            _ => Ok(()),
        }
    }
}

/// Mail driver/transport type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailDriver {
    /// SMTP transport
    Smtp,

    /// Sendmail transport
    Sendmail,

    /// Log to file (no actual sending)
    Log,

    /// In-memory storage (for testing)
    Memory,
}

/// Application-level SMTP connection configuration, typically loaded from
/// environment variables or a config file.
///
/// This struct is used to configure `MailConfig::smtp()`; it carries optional
/// auth credentials, encryption settings, timeout, and STARTTLS flags.
///
/// **Do not confuse this with [`crate::SmtpConfig`]** (re-exported from
/// `rf_mail::backends::smtp`), which is the lower-level config that
/// [`crate::facade::Mail::smtp`] and [`crate::SmtpMailer::new`] consume.
///
/// | Struct | Required by | Non-optional fields |
/// |--------|-------------|---------------------|
/// | `SmtpEnvConfig` (this) | `MailConfig::smtp()` | `host`, `port` only |
/// | `SmtpConfig` (backends) | `Mail::smtp()` / `SmtpMailer::new()` | `username`, `password`, `from_address` |
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpEnvConfig {
    /// SMTP server host
    pub host: String,

    /// SMTP server port
    pub port: u16,

    /// Username for authentication
    pub username: Option<String>,

    /// Password for authentication
    pub password: Option<String>,

    /// Encryption type
    pub encryption: Encryption,

    /// Connection timeout
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Enable STARTTLS
    #[serde(default)]
    pub starttls: bool,
}

/// Deprecated alias for [`SmtpEnvConfig`].
///
/// Use `SmtpEnvConfig` for application-level SMTP connection settings (loaded
/// from env vars / config files). For the backend-facing config used with
/// [`crate::facade::Mail::smtp`] and [`crate::SmtpMailer`], use
/// [`crate::SmtpConfig`] instead.
#[deprecated(since = "1.0.0-rc.2", note = "Use SmtpEnvConfig for application-level SMTP config or SmtpConfig (from rf_mail::backends) for Mail::smtp() / SmtpMailer::new()")]
pub type SmtpMailConfig = SmtpEnvConfig;

impl SmtpEnvConfig {
    /// Create a new SMTP configuration
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            username: None,
            password: None,
            encryption: Encryption::None,
            timeout: default_timeout(),
            starttls: false,
        }
    }

    /// Set authentication credentials
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Set encryption type
    pub fn with_encryption(mut self, encryption: Encryption) -> Self {
        self.encryption = encryption;
        self
    }

    /// Set timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Enable STARTTLS
    pub fn with_starttls(mut self) -> Self {
        self.starttls = true;
        self
    }

    /// Common Gmail configuration
    pub fn gmail(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::new("smtp.gmail.com", 587)
            .with_credentials(username, password)
            .with_encryption(Encryption::StartTls)
    }

    /// Common Mailgun configuration
    pub fn mailgun(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::new("smtp.mailgun.org", 587)
            .with_credentials(username, password)
            .with_encryption(Encryption::StartTls)
    }

    /// Common SendGrid configuration
    pub fn sendgrid(api_key: impl Into<String>) -> Self {
        Self::new("smtp.sendgrid.net", 587)
            .with_credentials("apikey", api_key)
            .with_encryption(Encryption::StartTls)
    }

    /// Common AWS SES configuration
    pub fn ses(username: impl Into<String>, password: impl Into<String>, region: &str) -> Self {
        let host = format!("email-smtp.{}.amazonaws.com", region);
        Self::new(host, 587)
            .with_credentials(username, password)
            .with_encryption(Encryption::StartTls)
    }
}

/// Sendmail configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendmailConfig {
    /// Path to sendmail binary
    #[serde(default = "default_sendmail_path")]
    pub path: String,

    /// Additional command-line arguments
    #[serde(default)]
    pub args: Vec<String>,
}

impl SendmailConfig {
    /// Create new sendmail configuration
    pub fn new() -> Self {
        Self {
            path: default_sendmail_path(),
            args: Vec::new(),
        }
    }

    /// Set custom sendmail path
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.path = path.into();
        self
    }

    /// Add command-line argument
    pub fn with_arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }
}

impl Default for SendmailConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Encryption type for SMTP
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Encryption {
    /// No encryption
    None,

    /// SSL/TLS encryption
    Tls,

    /// STARTTLS encryption
    StartTls,
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_sendmail_path() -> String {
    "/usr/sbin/sendmail".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_env_config() {
        let config = SmtpEnvConfig::new("smtp.example.com", 587)
            .with_credentials("user", "pass")
            .with_encryption(Encryption::StartTls);

        assert_eq!(config.host, "smtp.example.com");
        assert_eq!(config.port, 587);
        assert_eq!(config.username, Some("user".into()));
        assert_eq!(config.encryption, Encryption::StartTls);
    }

    #[test]
    fn test_smtp_env_config_presets() {
        let gmail = SmtpEnvConfig::gmail("user@gmail.com", "password");
        assert_eq!(gmail.host, "smtp.gmail.com");
        assert_eq!(gmail.port, 587);

        let sendgrid = SmtpEnvConfig::sendgrid("api_key");
        assert_eq!(sendgrid.host, "smtp.sendgrid.net");
        assert_eq!(sendgrid.username, Some("apikey".into()));
    }

    #[test]
    fn test_mail_config() {
        let smtp = SmtpEnvConfig::new("localhost", 1025);
        let config = MailConfig::smtp(Address::new("test@example.com"), smtp);

        assert_eq!(config.driver, MailDriver::Smtp);
        assert!(config.smtp.is_some());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_sendmail_config() {
        let sendmail = SendmailConfig::new().with_arg("-t");
        let config = MailConfig::sendmail(Address::new("test@example.com"), sendmail);

        assert_eq!(config.driver, MailDriver::Sendmail);
        assert!(config.sendmail.is_some());
    }

    #[test]
    fn test_config_validation() {
        // SMTP without config should fail
        let mut config = MailConfig::new(MailDriver::Smtp, Address::new("test@example.com"));
        assert!(config.validate().is_err());

        // SMTP with config should pass
        config.smtp = Some(SmtpEnvConfig::new("localhost", 1025));
        assert!(config.validate().is_ok());
    }

    /// Verify that the deprecated `SmtpMailConfig` alias still resolves to
    /// `SmtpEnvConfig` so existing callers compiled against the old name continue
    /// to work.
    #[allow(deprecated)]
    #[test]
    fn test_smtp_mail_config_deprecated_alias_resolves() {
        // `SmtpMailConfig` is a deprecated type alias for `SmtpEnvConfig`.
        // Creating one and calling its constructor proves the alias is live.
        let via_alias: SmtpMailConfig = SmtpEnvConfig::new("mail.example.com", 465)
            .with_encryption(Encryption::Tls);
        assert_eq!(via_alias.host, "mail.example.com");
        assert_eq!(via_alias.port, 465);
        assert_eq!(via_alias.encryption, Encryption::Tls);
    }
}
