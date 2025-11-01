use crate::domain::{Address, AddressList, Attachment, Content, Message};
use crate::templates::{RenderContext, TemplateEngine};
use async_trait::async_trait;
use std::sync::Arc;

/// Mailable trait - represents an email that can be built and sent
#[async_trait]
pub trait Mailable: Send + Sync {
    /// Build the email message
    async fn build(&self) -> Result<Message, MailableError>;

    /// Get the envelope information
    fn envelope(&self) -> MailableEnvelope;

    /// Get the content (can be overridden for templates)
    async fn content(&self) -> Result<Content, MailableError> {
        Ok(Content::new())
    }

    /// Get attachments
    fn attachments(&self) -> Vec<Attachment> {
        Vec::new()
    }
}

/// Envelope information for a mailable
pub struct MailableEnvelope {
    pub from: Option<Address>,
    pub reply_to: Option<Address>,
    pub to: AddressList,
    pub cc: AddressList,
    pub bcc: AddressList,
    pub subject: String,
}

impl MailableEnvelope {
    pub fn new(subject: impl Into<String>) -> Self {
        Self {
            from: None,
            reply_to: None,
            to: AddressList::new(),
            cc: AddressList::new(),
            bcc: AddressList::new(),
            subject: subject.into(),
        }
    }

    pub fn from(mut self, from: impl Into<Address>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn reply_to(mut self, reply_to: impl Into<Address>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    pub fn to(mut self, to: impl Into<AddressList>) -> Self {
        self.to = to.into();
        self
    }

    pub fn cc(mut self, cc: impl Into<AddressList>) -> Self {
        self.cc = cc.into();
        self
    }

    pub fn bcc(mut self, bcc: impl Into<AddressList>) -> Self {
        self.bcc = bcc.into();
        self
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MailableError {
    #[error("Missing sender address")]
    MissingFrom,

    #[error("Missing recipients")]
    MissingRecipients,

    #[error("Template error: {0}")]
    Template(String),

    #[error("Content error: {0}")]
    Content(String),
}

/// Base implementation for building a message from a mailable
pub async fn build_message<M: Mailable + ?Sized>(mailable: &M) -> Result<Message, MailableError> {
    let envelope = mailable.envelope();
    let content = mailable.content().await?;
    let attachments = mailable.attachments();

    let from = envelope.from.ok_or(MailableError::MissingFrom)?;

    if envelope.to.is_empty() {
        return Err(MailableError::MissingRecipients);
    }

    let mut builder = Message::builder()
        .from(from)
        .to_many(envelope.to)
        .subject(envelope.subject)
        .content(content);

    if let Some(reply_to) = envelope.reply_to {
        builder = builder.reply_to(reply_to);
    }

    for addr in envelope.cc {
        builder = builder.cc(addr);
    }

    for addr in envelope.bcc {
        builder = builder.bcc(addr);
    }

    for attachment in attachments {
        builder = builder.attach(attachment);
    }

    builder.build().map_err(|e| MailableError::Content(e.to_string()))
}

/// Template-based mailable
pub struct TemplateMailable {
    pub envelope: MailableEnvelope,
    pub template: String,
    pub context: RenderContext,
    pub engine: Arc<dyn TemplateEngine>,
    pub attachments: Vec<Attachment>,
}

#[async_trait]
impl Mailable for TemplateMailable {
    async fn build(&self) -> Result<Message, MailableError> {
        build_message(self).await
    }

    fn envelope(&self) -> MailableEnvelope {
        MailableEnvelope {
            from: self.envelope.from.clone(),
            reply_to: self.envelope.reply_to.clone(),
            to: self.envelope.to.clone(),
            cc: self.envelope.cc.clone(),
            bcc: self.envelope.bcc.clone(),
            subject: self.envelope.subject.clone(),
        }
    }

    async fn content(&self) -> Result<Content, MailableError> {
        let html = self
            .engine
            .render(&self.template, &self.context.data)
            .await
            .map_err(|e| MailableError::Template(e.to_string()))?;

        Ok(Content::html(html))
    }

    fn attachments(&self) -> Vec<Attachment> {
        self.attachments.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct SimpleMailable {
        to: String,
        subject: String,
        body: String,
    }

    #[async_trait]
    impl Mailable for SimpleMailable {
        async fn build(&self) -> Result<Message, MailableError> {
            build_message(self).await
        }

        fn envelope(&self) -> MailableEnvelope {
            MailableEnvelope::new(&self.subject)
                .from("sender@example.com")
                .to(Address::new(&self.to))
        }

        async fn content(&self) -> Result<Content, MailableError> {
            Ok(Content::text(&self.body))
        }
    }

    #[tokio::test]
    async fn test_simple_mailable() {
        let mailable = SimpleMailable {
            to: "recipient@example.com".to_string(),
            subject: "Test Email".to_string(),
            body: "Hello World".to_string(),
        };

        let message = mailable.build().await.unwrap();
        assert_eq!(message.envelope.subject, "Test Email");
        assert!(message.content.has_text());
    }
}
