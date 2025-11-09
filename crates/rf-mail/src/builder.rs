//! Message builder for fluent email construction

use crate::{Address, Attachment, MailError, Message};

/// Fluent builder for email messages
///
/// # Example
///
/// ```
/// use rf_mail::{MessageBuilder, Address};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let message = MessageBuilder::new()
///     .from(Address::with_name("sender@example.com", "Sender"))
///     .to(Address::new("recipient@example.com"))
///     .subject("Hello!")
///     .text("Hello, World!")
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    /// Create new message builder
    pub fn new() -> Self {
        Self {
            message: Message::new(),
        }
    }

    /// Set from address
    pub fn from(mut self, address: Address) -> Self {
        self.message.from = address;
        self
    }

    /// Add to address
    pub fn to(mut self, address: Address) -> Self {
        self.message.to.push(address);
        self
    }

    /// Add multiple to addresses
    pub fn to_many(mut self, addresses: Vec<Address>) -> Self {
        self.message.to.extend(addresses);
        self
    }

    /// Add cc address
    pub fn cc(mut self, address: Address) -> Self {
        self.message.cc.push(address);
        self
    }

    /// Add bcc address
    pub fn bcc(mut self, address: Address) -> Self {
        self.message.bcc.push(address);
        self
    }

    /// Set reply-to address
    pub fn reply_to(mut self, address: Address) -> Self {
        self.message.reply_to = Some(address);
        self
    }

    /// Set subject
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.message.subject = subject.into();
        self
    }

    /// Set HTML body
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.message.html = Some(html.into());
        self
    }

    /// Set plain text body
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.message.text = Some(text.into());
        self
    }

    /// Add attachment
    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.message.attachments.push(attachment);
        self
    }

    /// Add custom header
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.message.headers.insert(key.into(), value.into());
        self
    }

    /// Build the message (validates before returning)
    pub fn build(self) -> Result<Message, MailError> {
        self.message
            .validate()
            .map_err(|e| MailError::InvalidMessage(e))?;

        Ok(self.message)
    }
}

impl Default for MessageBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert_eq!(message.from.email, "sender@example.com");
        assert_eq!(message.to.len(), 1);
        assert_eq!(message.subject, "Test");
        assert_eq!(message.text, Some("Hello".into()));
    }

    #[test]
    fn test_builder_validation() {
        let result = MessageBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_to_many() {
        let addresses = vec![
            Address::new("user1@example.com"),
            Address::new("user2@example.com"),
        ];

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to_many(addresses)
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert_eq!(message.to.len(), 2);
    }

    #[test]
    fn test_builder_headers() {
        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Hello")
            .header("X-Custom", "value")
            .build()
            .unwrap();

        assert_eq!(message.headers.get("X-Custom"), Some(&"value".to_string()));
    }
}
