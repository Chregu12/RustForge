//! SMTP mailer backend

use crate::{MailError, Mailer, Message};
use async_trait::async_trait;
use lettre::{
    message::{header::ContentType, Mailbox, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor,
};
use serde::{Deserialize, Serialize};

/// SMTP mailer configuration
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
/// mailer.send(&message).await?;
/// # Ok(())
/// # }
/// ```
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
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let lettre_message = convert_to_lettre(message)?;

        self.transport.send(lettre_message).await?;

        tracing::info!(
            to = ?message.to,
            subject = %message.subject,
            "Email sent via SMTP"
        );

        Ok(())
    }
}

/// Convert our Message to lettre's Message
fn convert_to_lettre(message: &Message) -> Result<LettreMessage, MailError> {
    // Parse from address
    let from: Mailbox = if let Some(name) = &message.from.name {
        format!("{} <{}>", name, message.from.email).parse()?
    } else {
        message.from.email.parse()?
    };

    let mut builder = LettreMessage::builder().from(from);

    // Add To addresses
    for to in &message.to {
        let mailbox: Mailbox = if let Some(name) = &to.name {
            format!("{} <{}>", name, to.email).parse()?
        } else {
            to.email.parse()?
        };
        builder = builder.to(mailbox);
    }

    // Add CC addresses
    for cc in &message.cc {
        let mailbox: Mailbox = if let Some(name) = &cc.name {
            format!("{} <{}>", name, cc.email).parse()?
        } else {
            cc.email.parse()?
        };
        builder = builder.cc(mailbox);
    }

    // Add BCC addresses
    for bcc in &message.bcc {
        let mailbox: Mailbox = if let Some(name) = &bcc.name {
            format!("{} <{}>", name, bcc.email).parse()?
        } else {
            bcc.email.parse()?
        };
        builder = builder.bcc(mailbox);
    }

    // Add reply-to
    if let Some(reply_to) = &message.reply_to {
        let mailbox: Mailbox = if let Some(name) = &reply_to.name {
            format!("{} <{}>", name, reply_to.email).parse()?
        } else {
            reply_to.email.parse()?
        };
        builder = builder.reply_to(mailbox);
    }

    // Add subject
    builder = builder.subject(&message.subject);

    // Build body (multipart if both HTML and text)
    let lettre_message = match (&message.html, &message.text) {
        (Some(html), Some(text)) => {
            // Multipart: both HTML and text
            builder.multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(text.clone()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html.clone()),
                    ),
            )?
        }
        (Some(html), None) => {
            // HTML only
            builder.body(html.clone())?
        }
        (None, Some(text)) => {
            // Text only
            builder.body(text.clone())?
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
    use crate::{Address, MessageBuilder};

    #[test]
    fn test_convert_to_lettre() {
        let message = MessageBuilder::new()
            .from(Address::with_name("sender@example.com", "Sender"))
            .to(Address::with_name("recipient@example.com", "Recipient"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        let lettre_msg = convert_to_lettre(&message);
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

        let lettre_msg = convert_to_lettre(&message);
        assert!(lettre_msg.is_ok());
    }
}
