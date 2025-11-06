use super::{MailTransport, SmtpConfig, TransportError, TransportResponse, TransportResult};
use crate::domain::Message;
use async_trait::async_trait;
use lettre::message::{header, header::Header, Mailbox, MultiPart, SinglePart};
use lettre::transport::smtp::authentication::Credentials;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message as LettreMessage, Tokio1Executor};

/// SMTP transport implementation
pub struct SmtpTransport {
    config: SmtpConfig,
    transport: AsyncSmtpTransport<Tokio1Executor>,
}

impl SmtpTransport {
    pub fn new(config: SmtpConfig) -> Result<Self, TransportError> {
        let transport = Self::build_transport(&config)?;

        Ok(Self { config, transport })
    }

    fn build_transport(config: &SmtpConfig) -> Result<AsyncSmtpTransport<Tokio1Executor>, TransportError> {
        // Create builder based on TLS setting
        let builder = if config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
                .map_err(|e| TransportError::Connection(e.to_string()))?
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
        };

        let mut builder = builder.port(config.port);

        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            let credentials = Credentials::new(username.clone(), password.clone());
            builder = builder.credentials(credentials);
        }

        if let Some(timeout) = config.timeout {
            builder = builder.timeout(Some(timeout));
        }

        Ok(builder.build())
    }

    fn convert_message(&self, message: &Message) -> Result<LettreMessage, TransportError> {
        // Parse sender
        let from: Mailbox = message
            .envelope
            .from
            .to_string()
            .parse()
            .map_err(|e| TransportError::Other(format!("Invalid from address: {}", e)))?;

        // Parse recipients
        let mut builder = LettreMessage::builder().from(from);

        for addr in message.envelope.to.iter() {
            let mailbox: Mailbox = addr
                .to_string()
                .parse()
                .map_err(|e: lettre::address::AddressError| TransportError::InvalidRecipient(e.to_string()))?;
            builder = builder.to(mailbox);
        }

        for addr in message.envelope.cc.iter() {
            let mailbox: Mailbox = addr
                .to_string()
                .parse()
                .map_err(|e: lettre::address::AddressError| TransportError::InvalidRecipient(e.to_string()))?;
            builder = builder.cc(mailbox);
        }

        for addr in message.envelope.bcc.iter() {
            let mailbox: Mailbox = addr
                .to_string()
                .parse()
                .map_err(|e: lettre::address::AddressError| TransportError::InvalidRecipient(e.to_string()))?;
            builder = builder.bcc(mailbox);
        }

        // Reply-To
        if let Some(reply_to) = &message.envelope.reply_to {
            let mailbox: Mailbox = reply_to
                .to_string()
                .parse()
                .map_err(|e| TransportError::Other(format!("Invalid reply-to: {}", e)))?;
            builder = builder.reply_to(mailbox);
        }

        // Subject
        builder = builder.subject(&message.envelope.subject);

        // Custom headers - skip for now as API has changed
        // TODO: Implement proper header handling for lettre 0.11
        // for (key, value) in &message.envelope.headers {
        //     builder = builder.header(...);
        // }

        // Build multipart message
        let mut multipart = if let (Some(text), Some(html)) = (&message.content.text, &message.content.html) {
            // Both text and HTML
            MultiPart::alternative_plain_html(text.clone(), html.clone())
        } else if let Some(text) = &message.content.text {
            // Text only
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(header::ContentType::TEXT_PLAIN)
                    .body(text.clone()),
            )
        } else if let Some(html) = &message.content.html {
            // HTML only
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(header::ContentType::TEXT_HTML)
                    .body(html.clone()),
            )
        } else {
            // No content - create an empty alternative multipart with a plain text part
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(header::ContentType::TEXT_PLAIN)
                    .body(String::new()),
            )
        };

        // Add attachments
        for attachment in &message.attachments {
            let content_type: mime::Mime = attachment
                .content_type
                .parse()
                .unwrap_or(mime::APPLICATION_OCTET_STREAM);

            let mut part_builder = SinglePart::builder()
                .header(header::ContentType::parse(content_type.as_ref()).unwrap_or(header::ContentType::TEXT_PLAIN));

            // Handle inline vs attachment
            if attachment.inline {
                if let Some(content_id) = &attachment.content_id {
                    // ContentId expects the value to already include angle brackets
                    let cid_value = if content_id.starts_with('<') {
                        content_id.clone()
                    } else {
                        format!("<{}>", content_id)
                    };
                    if let Ok(cid) = header::ContentId::parse(&cid_value) {
                        part_builder = part_builder.header(cid);
                    }
                }
                part_builder = part_builder.header(header::ContentDisposition::inline());
            } else {
                part_builder = part_builder.header(header::ContentDisposition::attachment(&attachment.filename));
            }

            let part = part_builder.body(attachment.data.clone());
            multipart = multipart.singlepart(part);
        }

        builder
            .multipart(multipart)
            .map_err(|e| TransportError::Other(e.to_string()))
    }
}

#[async_trait]
impl MailTransport for SmtpTransport {
    async fn send(&self, message: &Message) -> TransportResult {
        let lettre_message = self.convert_message(message)?;

        let _response = self
            .transport
            .send(lettre_message)
            .await
            .map_err(|e| TransportError::Smtp(e.to_string()))?;

        Ok(TransportResponse {
            message_id: message.id.clone(),
            accepted: message
                .envelope
                .recipients()
                .map(|addr| addr.email.clone())
                .collect(),
            rejected: Vec::new(),
        })
    }

    async fn test_connection(&self) -> Result<(), TransportError> {
        self.transport
            .test_connection()
            .await
            .map_err(|e| TransportError::Connection(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Address, Content};

    #[test]
    fn test_smtp_transport_creation() {
        let config = SmtpConfig {
            host: "localhost".to_string(),
            port: 1025,
            username: None,
            password: None,
            from_address: "test@example.com".to_string(),
            from_name: None,
            timeout: None,
            use_tls: false,
            use_starttls: false,
        };

        let transport = SmtpTransport::new(config);
        assert!(transport.is_ok());
    }
}
