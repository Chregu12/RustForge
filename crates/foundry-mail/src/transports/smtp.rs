use super::{MailTransport, SmtpConfig, TransportError, TransportResponse, TransportResult};
use crate::domain::Message;
use async_trait::async_trait;
use lettre::message::{header, Mailbox, MultiPart, SinglePart};
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
        let mut builder = if config.use_tls {
            AsyncSmtpTransport::<Tokio1Executor>::relay(&config.host)
        } else {
            AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&config.host)
        }
        .map_err(|e| TransportError::Connection(e.to_string()))?;

        builder = builder.port(config.port);

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
                .map_err(|e| TransportError::InvalidRecipient(e.to_string()))?;
            builder = builder.to(mailbox);
        }

        for addr in message.envelope.cc.iter() {
            let mailbox: Mailbox = addr
                .to_string()
                .parse()
                .map_err(|e| TransportError::InvalidRecipient(e.to_string()))?;
            builder = builder.cc(mailbox);
        }

        for addr in message.envelope.bcc.iter() {
            let mailbox: Mailbox = addr
                .to_string()
                .parse()
                .map_err(|e| TransportError::InvalidRecipient(e.to_string()))?;
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

        // Custom headers
        for (key, value) in &message.envelope.headers {
            builder = builder.header((key.as_str(), value.as_str()));
        }

        // Build multipart message
        let mut multipart = if let Some(text) = &message.content.text {
            MultiPart::alternative_plain_html(
                text.clone(),
                message.content.html.clone().unwrap_or_default(),
            )
        } else if let Some(html) = &message.content.html {
            MultiPart::alternative().singlepart(
                SinglePart::builder()
                    .header(header::ContentType::TEXT_HTML)
                    .body(html.clone()),
            )
        } else {
            MultiPart::mixed()
        };

        // Add attachments
        for attachment in &message.attachments {
            let content_type = attachment
                .content_type
                .parse::<mime::Mime>()
                .unwrap_or(mime::APPLICATION_OCTET_STREAM);

            let mut part = SinglePart::builder()
                .header(header::ContentType::from(content_type))
                .body(attachment.data.clone());

            if attachment.inline {
                if let Some(content_id) = &attachment.content_id {
                    part = part.header(header::ContentId::from(format!("<{}>", content_id)));
                    part = part.header(header::ContentDisposition::inline());
                }
            } else {
                part = part.header(header::ContentDisposition::attachment(&attachment.filename));
            }

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

        let response = self
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
            .map_err(|e| TransportError::Connection(e.to_string()))
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
