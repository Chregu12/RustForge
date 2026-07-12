//! SMTP mailer backend

use crate::{Mail, MailError, Mailer};
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor,
};
use serde::{Deserialize, Serialize};

/// Backend-level SMTP transport configuration — the struct to pass to
/// [`crate::facade::Mail::smtp`] and [`crate::SmtpMailer::new`].
///
/// All credential fields (`username`, `password`, `from_address`) are
/// **required** and non-optional here, because the SMTP transport needs them
/// before the first connection attempt.
///
/// **Do not confuse this with [`crate::SmtpEnvConfig`]** (from
/// `rf_mail::config`), which is the application-level, environment-variable
/// friendly config used with `MailConfig::smtp()` and has optional auth fields.
///
/// | Struct | Required by | Auth fields |
/// |--------|-------------|-------------|
/// | `SmtpConfig` (this) | `Mail::smtp()` / `SmtpMailer::new()` | non-optional |
/// | `SmtpEnvConfig` | `MailConfig::smtp()` | `Option<String>` |
///
/// # Example
///
/// ```rust,no_run
/// use rf_mail::{facade::Mail, SmtpConfig};
///
/// # fn example() -> rf_mail::MailResult<()> {
/// Mail::smtp(SmtpConfig {
///     host: "smtp.example.com".into(),
///     port: 587,
///     username: "user@example.com".into(),
///     password: "secret".into(),
///     from_address: "noreply@example.com".into(),
///     from_name: Some("MyApp".into()),
/// })?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    /// SMTP server host
    pub host: String,

    /// SMTP server port
    pub port: u16,

    /// Username for authentication
    pub username: String,

    /// Password for authentication
    pub password: String,

    /// Default from address
    pub from_address: String,

    /// Default from name
    pub from_name: Option<String>,
}

/// SMTP mailer backend
///
/// # Example
///
/// ```no_run
/// use rf_mail::{SmtpMailer, SmtpConfig, Mailer, MessageBuilder, Address};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = SmtpConfig {
///     host: "smtp.gmail.com".into(),
///     port: 587,
///     username: "user@gmail.com".into(),
///     password: "app_password".into(),
///     from_address: "noreply@example.com".into(),
///     from_name: Some("MyApp".into()),
/// };
///
/// let mailer = SmtpMailer::new(config).await?;
///
/// let message = MessageBuilder::new()
///     .from(Address::with_name("noreply@example.com", "MyApp"))
///     .to(Address::new("recipient@example.com"))
///     .subject("Hello")
///     .text("Hello, World!")
///     .build()?;
///
/// mailer.send(message.into()).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct SmtpMailer {
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpMailer {
    /// Create new SMTP mailer
    pub async fn new(config: SmtpConfig) -> Result<Self, MailError> {
        let credentials = Credentials::new(config.username.clone(), config.password.clone());

        let transport = AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)?
            .port(config.port)
            .credentials(credentials)
            .build();

        Ok(Self { transport })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        let lettre_message = convert_to_lettre(&mail)?;

        self.transport.send(lettre_message).await?;

        tracing::info!(
            to = ?mail.to,
            subject = %mail.subject,
            "Email sent via SMTP"
        );

        Ok(())
    }
}

/// Convert our Message to lettre's Message
fn convert_to_lettre(mail: &Mail) -> Result<LettreMessage, MailError> {
    // Parse from address
    let from: Mailbox = if let Some(name) = &mail.from.name {
        format!("{} <{}>", name, mail.from.email).parse()?
    } else {
        mail.from.email.parse()?
    };

    let mut builder = LettreMessage::builder().from(from);

    // Add To addresses
    for to in &mail.to {
        let mailbox: Mailbox = if let Some(name) = &to.name {
            format!("{} <{}>", name, to.email).parse()?
        } else {
            to.email.parse()?
        };
        builder = builder.to(mailbox);
    }

    // Add CC addresses
    for cc in &mail.cc {
        let mailbox: Mailbox = if let Some(name) = &cc.name {
            format!("{} <{}>", name, cc.email).parse()?
        } else {
            cc.email.parse()?
        };
        builder = builder.cc(mailbox);
    }

    // Add BCC addresses
    for bcc in &mail.bcc {
        let mailbox: Mailbox = if let Some(name) = &bcc.name {
            format!("{} <{}>", name, bcc.email).parse()?
        } else {
            bcc.email.parse()?
        };
        builder = builder.bcc(mailbox);
    }

    // Add reply-to
    if let Some(reply_to) = &mail.reply_to {
        let mailbox: Mailbox = if let Some(name) = &reply_to.name {
            format!("{} <{}>", name, reply_to.email).parse()?
        } else {
            reply_to.email.parse()?
        };
        builder = builder.reply_to(mailbox);
    }

    // Add subject
    builder = builder.subject(&mail.subject);

    // Build body (multipart if both HTML and text)
    let lettre_message = match (mail.html(), mail.text()) {
        (Some(html), Some(text)) => {
            // Multipart: both HTML and text
            builder.multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html.to_string()),
                    ),
            )?
        }
        (Some(html), None) => {
            // HTML only
            builder.body(html.to_string())?
        }
        (None, Some(text)) => {
            // Text only
            builder.body(text.to_string())?
        }
        (None, None) => {
            return Err(MailError::InvalidMessage("No body content".into()));
        }
    };

    Ok(lettre_message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, Mail, MessageBuilder};

    /// Prove that `SmtpConfig` (the backend-facing struct) is the correct type to
    /// construct for `Mail::smtp()` / `SmtpMailer::new()`, and that it is distinct
    /// from `SmtpEnvConfig` (the application-level env-var config).
    ///
    /// This test closes the gap flagged in the cycle-6 audit: previously two
    /// structs with confusingly similar names were both re-exported from rf-mail
    /// with no guidance on which to use.
    #[test]
    fn test_smtp_config_is_the_facade_config_not_smtp_env_config() {
        // `SmtpConfig` has non-optional username/password/from_address fields —
        // correct for `SmtpMailer::new()` and `Mail::smtp()`.
        let cfg = SmtpConfig {
            host: "smtp.example.com".into(),
            port: 587,
            username: "user@example.com".into(),
            password: "s3cr3t".into(),
            from_address: "noreply@example.com".into(),
            from_name: Some("MyApp".into()),
        };
        assert_eq!(cfg.host, "smtp.example.com");
        assert_eq!(cfg.username, "user@example.com");
        assert_eq!(cfg.from_address, "noreply@example.com");

        // `SmtpEnvConfig` has optional auth fields and is for `MailConfig::smtp()`.
        let env_cfg = crate::config::SmtpEnvConfig::new("smtp.example.com", 587)
            .with_credentials("user@example.com", "s3cr3t");
        assert_eq!(env_cfg.host, "smtp.example.com");
        assert_eq!(env_cfg.username, Some("user@example.com".into()));
        // SmtpEnvConfig does NOT have `from_address` — it's application config, not transport config.
    }

    #[test]
    fn test_convert_to_lettre() {
        let message = MessageBuilder::new()
            .from(Address::with_name("sender@example.com", "Sender"))
            .to(Address::with_name("recipient@example.com", "Recipient"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        let mail: Mail = message.into();
        let lettre_msg = convert_to_lettre(&mail);
        assert!(lettre_msg.is_ok());
    }

    #[test]
    fn test_convert_multipart() {
        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .html("<h1>Hello</h1>")
            .text("Hello")
            .build()
            .unwrap();

        let mail: Mail = message.into();
        let lettre_msg = convert_to_lettre(&mail);
        assert!(lettre_msg.is_ok());
    }

    /// Helper to check whether a live MailHog SMTP server is reachable.
    ///
    /// Mirrors the graceful-skip TCP probe used by `rf-storage`'s
    /// `s3_available`: the live test SKIPS (prints a skip line and passes) when
    /// MailHog is down, and sends a real message when it is up. Bring the
    /// service up with `scripts/test-env-up.sh` (mailhog SMTP on 1025 / HTTP UI
    /// on 8025 in `docker-compose.test.yml`).
    /// SMTP host:port the live test targets. Defaults to MailHog's 127.0.0.1:1025,
    /// but is overridable via `RF_SMTP_TEST_ADDR` so the test is runnable on a host
    /// where 1025 is squatted by an unrelated process (e.g. macOS FinderSync) or
    /// where the sink listens elsewhere.
    fn smtp_test_addr() -> (String, u16) {
        let raw = std::env::var("RF_SMTP_TEST_ADDR").unwrap_or_else(|_| "127.0.0.1:1025".to_string());
        match raw.rsplit_once(':') {
            Some((h, p)) => (h.to_string(), p.parse().unwrap_or(1025)),
            None => (raw, 1025),
        }
    }

    async fn mailhog_available() -> bool {
        use tokio::io::AsyncReadExt;

        let (host, port) = smtp_test_addr();
        let Ok(mut stream) = tokio::net::TcpStream::connect((host.as_str(), port)).await else {
            return false;
        };
        // A bare TCP probe (as used for S3/MinIO on 9000) can false-positive on
        // 1025: unrelated local processes sometimes squat that port, accept the
        // connection, and never speak SMTP. Confirm we are really talking to an
        // SMTP server by reading its greeting, which must begin with the "220"
        // service-ready reply, before declaring MailHog "available". A squatter
        // that sends nothing simply times out and is treated as absent (skip).
        let mut buf = [0u8; 3];
        match tokio::time::timeout(
            std::time::Duration::from_secs(2),
            stream.read_exact(&mut buf),
        )
        .await
        {
            Ok(Ok(_)) => &buf == b"220",
            _ => false,
        }
    }

    #[tokio::test]
    async fn test_smtp_send_via_mailhog() {
        if !mailhog_available().await {
            eprintln!(
                "⏭️  Skipping test_smtp_send_via_mailhog: MailHog SMTP (127.0.0.1:1025) not available"
            );
            eprintln!("   Start services with: ./scripts/test-env-up.sh");
            return;
        }

        // MailHog speaks plaintext SMTP on 1025 (no TLS, no auth), so the
        // transport is built with `builder_dangerous` rather than the TLS
        // `relay` used by `SmtpMailer::new`. Everything downstream — the real
        // `convert_to_lettre` message construction and `SmtpMailer::send` over a
        // live lettre SMTP connection — is exercised exactly as in production.
        let (host, port) = smtp_test_addr();
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
            .port(port)
            .build();
        let mailer = SmtpMailer { transport };

        let message = MessageBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("RustForge live SMTP test")
            .text("Hello from the RustForge graceful-skip live SMTP test")
            .build()
            .unwrap();

        let result = mailer.send(message.into()).await;
        assert!(
            result.is_ok(),
            "live SMTP send to MailHog must succeed: {result:?}"
        );
    }
}
