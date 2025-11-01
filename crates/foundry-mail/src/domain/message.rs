use super::{Address, AddressList, Attachment, Content, Envelope};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Complete email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: String,
    pub envelope: Envelope,
    pub content: Content,
    pub attachments: Vec<Attachment>,
}

impl Message {
    pub fn builder() -> MessageBuilder {
        MessageBuilder::default()
    }

    pub fn new(
        from: Address,
        to: impl Into<AddressList>,
        subject: impl Into<String>,
        content: Content,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            envelope: Envelope::new(from, to, subject),
            content,
            attachments: Vec::new(),
        }
    }

    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }

    pub fn total_size(&self) -> usize {
        let content_size = self.content.text.as_ref().map(|s| s.len()).unwrap_or(0)
            + self.content.html.as_ref().map(|s| s.len()).unwrap_or(0);
        let attachments_size: usize = self.attachments.iter().map(|a| a.size()).sum();
        content_size + attachments_size
    }
}

#[derive(Debug, Default)]
pub struct MessageBuilder {
    from: Option<Address>,
    reply_to: Option<Address>,
    to: AddressList,
    cc: AddressList,
    bcc: AddressList,
    subject: Option<String>,
    content: Content,
    attachments: Vec<Attachment>,
    headers: Vec<(String, String)>,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from(mut self, from: impl Into<Address>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn reply_to(mut self, reply_to: impl Into<Address>) -> Self {
        self.reply_to = Some(reply_to.into());
        self
    }

    pub fn to(mut self, to: impl Into<Address>) -> Self {
        self.to.add(to.into());
        self
    }

    pub fn to_many(mut self, to: impl Into<AddressList>) -> Self {
        self.to = to.into();
        self
    }

    pub fn cc(mut self, cc: impl Into<Address>) -> Self {
        self.cc.add(cc.into());
        self
    }

    pub fn bcc(mut self, bcc: impl Into<Address>) -> Self {
        self.bcc.add(bcc.into());
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.content = self.content.with_text(text);
        self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.content = self.content.with_html(html);
        self
    }

    pub fn content(mut self, content: Content) -> Self {
        self.content = content;
        self
    }

    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.attachments.push(attachment);
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn build(self) -> Result<Message, MessageBuilderError> {
        let from = self.from.ok_or(MessageBuilderError::MissingFrom)?;
        let subject = self.subject.ok_or(MessageBuilderError::MissingSubject)?;

        if self.to.is_empty() {
            return Err(MessageBuilderError::MissingRecipients);
        }

        if self.content.is_empty() {
            return Err(MessageBuilderError::MissingContent);
        }

        let mut envelope = Envelope::new(from, self.to, subject);

        if let Some(reply_to) = self.reply_to {
            envelope = envelope.reply_to(reply_to);
        }

        if !self.cc.is_empty() {
            envelope = envelope.cc(self.cc);
        }

        if !self.bcc.is_empty() {
            envelope = envelope.bcc(self.bcc);
        }

        for (key, value) in self.headers {
            envelope = envelope.add_header(key, value);
        }

        Ok(Message {
            id: Uuid::new_v4().to_string(),
            envelope,
            content: self.content,
            attachments: self.attachments,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum MessageBuilderError {
    #[error("Missing sender address")]
    MissingFrom,

    #[error("Missing subject")]
    MissingSubject,

    #[error("Missing recipients")]
    MissingRecipients,

    #[error("Missing content")]
    MissingContent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder() {
        let message = Message::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text("Hello World")
            .build()
            .unwrap();

        assert_eq!(message.envelope.from.email, "sender@example.com");
        assert_eq!(message.envelope.subject, "Test");
        assert!(message.content.has_text());
    }

    #[test]
    fn test_message_builder_missing_from() {
        let result = Message::builder()
            .to("recipient@example.com")
            .subject("Test")
            .text("Hello")
            .build();

        assert!(matches!(result, Err(MessageBuilderError::MissingFrom)));
    }

    #[test]
    fn test_message_total_size() {
        let message = Message::builder()
            .from("sender@example.com")
            .to("recipient@example.com")
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert_eq!(message.total_size(), 5); // "Hello".len()
    }
}
