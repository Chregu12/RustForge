//! Sendmail transport backend

use crate::{Mail, MailError, Mailer};
use async_trait::async_trait;
use lettre::transport::sendmail::SendmailTransport as LettreTransport;
use lettre::{Message as LettreMessage, Transport};

/// Sendmail mailer that uses the system's sendmail command
pub struct SendmailMailer {
    transport: LettreTransport,
}

impl SendmailMailer {
    /// Create a new sendmail mailer with default path
    pub fn new() -> Result<Self, MailError> {
        let transport = LettreTransport::new();
        Ok(Self { transport })
    }

    /// Create a sendmail mailer with custom path
    pub fn with_path(path: impl Into<String>) -> Result<Self, MailError> {
        let transport = LettreTransport::new_with_command(path.into());
        Ok(Self { transport })
    }

    /// Convert our Message to lettre's Message
    fn convert_message(message: &Message) -> Result<LettreMessage, MailError> {
        use lettre::message::{header, Mailbox, MultiPart, SinglePart};

        let from: Mailbox = format!(
            "{} <{}>",
            mail.from.name.as_deref().unwrap_or(""),
            mail.from.email
        )
        .parse()?;

        let mut builder = LettreMessage::builder().from(from);

        // Add recipients
        for addr in &mail.to {
            let mailbox: Mailbox = if let Some(name) = &addr.name {
                format!("{} <{}>", name, addr.email).parse()?
            } else {
                addr.email.parse()?
            };
            builder = builder.to(mailbox);
        }

        // Add CC
        for addr in &mail.cc {
            let mailbox: Mailbox = addr.email.parse()?;
            builder = builder.cc(mailbox);
        }

        // Add BCC
        for addr in &mail.bcc {
            let mailbox: Mailbox = addr.email.parse()?;
            builder = builder.bcc(mailbox);
        }

        // Add Reply-To
        if let Some(reply_to) = &mail.reply_to {
            let mailbox: Mailbox = reply_to.email.parse()?;
            builder = builder.reply_to(mailbox);
        }

        // Subject
        builder = builder.subject(&mail.subject);

        // Build body
        let lettre_msg = match (&mail.html, &mail.text) {
            (Some(html), Some(text)) => {
                // Multipart message
                builder.multipart(
                    MultiPart::alternative()
                        .singlepart(
                            SinglePart::builder()
                                .header(header::ContentType::TEXT_PLAIN)
                                .body(text.clone()),
                        )
                        .singlepart(
                            SinglePart::builder()
                                .header(header::ContentType::TEXT_HTML)
                                .body(html.clone()),
                        ),
                )?
            }
            (Some(html), None) => {
                // HTML only
                builder.singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_HTML)
                        .body(html.clone()),
                )?
            }
            (None, Some(text)) => {
                // Text only
                builder.singlepart(
                    SinglePart::builder()
                        .header(header::ContentType::TEXT_PLAIN)
                        .body(text.clone()),
                )?
            }
            (None, None) => {
                return Err(MailError::InvalidMessage(
                    "Message must have either HTML or text body".into(),
                ))
            }
        };

        Ok(lettre_msg)
    }
}

impl Default for SendmailMailer {
    fn default() -> Self {
        Self::new().expect("Failed to create default sendmail mailer")
    }
}

#[async_trait]
impl Mailer for SendmailMailer {
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        let lettre_msg = Self::convert_message(mail)?;

        // Sendmail transport doesn't support async, so we use blocking send
        let envelope = lettre_msg.envelope();
        let formatted = lettre_msg.formatted();

        self.transport
            .send_raw(&envelope, &formatted)
            .map_err(|e| MailError::SendFailed(e.to_string()))?;

        tracing::info!(
            "Email sent via sendmail: {} -> {}",
            mail.from.email,
            message
                .to
                .iter()
                .map(|a| a.email.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MessageBuilder};

    #[test]
    fn test_convert_message() {
        let message = MessageBuilder::new()
            .from(Address::with_name("sender@example.com", "Sender"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        let lettre_msg = SendmailMailer::convert_message(&mail);
        assert!(lettre_msg.is_ok());
    }
}
