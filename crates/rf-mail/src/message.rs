//! Email message types

use crate::{Address, Attachment};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Email message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: String,

    /// From address
    pub from: Address,

    /// To addresses
    pub to: Vec<Address>,

    /// CC addresses
    pub cc: Vec<Address>,

    /// BCC addresses
    pub bcc: Vec<Address>,

    /// Reply-to address
    pub reply_to: Option<Address>,

    /// Subject
    pub subject: String,

    /// HTML body
    pub html: Option<String>,

    /// Plain text body
    pub text: Option<String>,

    /// Attachments
    pub attachments: Vec<Attachment>,

    /// Custom headers
    pub headers: HashMap<String, String>,
}

impl Message {
    /// Create new message with defaults
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            from: Address::new(""),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            reply_to: None,
            subject: String::new(),
            html: None,
            text: None,
            attachments: Vec::new(),
            headers: HashMap::new(),
        }
    }

    /// Validate message
    pub fn validate(&self) -> Result<(), String> {
        if self.from.email.is_empty() {
            return Err("From address is required".into());
        }
        if self.to.is_empty() {
            return Err("At least one To address is required".into());
        }
        if self.subject.is_empty() {
            return Err("Subject is required".into());
        }
        if self.html.is_none() && self.text.is_none() {
            return Err("Either HTML or text body is required".into());
        }

        Ok(())
    }

    /// Get total number of recipients
    pub fn recipient_count(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    /// Get total attachment size in bytes
    pub fn attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.size()).sum()
    }
}

impl Default for Message {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_new() {
        let msg = Message::new();
        assert!(!msg.id.is_empty());
        assert!(msg.to.is_empty());
    }

    #[test]
    fn test_message_validate() {
        let mut msg = Message::new();

        // Missing from
        assert!(msg.validate().is_err());

        msg.from = Address::new("sender@example.com");

        // Missing to
        assert!(msg.validate().is_err());

        msg.to.push(Address::new("recipient@example.com"));

        // Missing subject
        assert!(msg.validate().is_err());

        msg.subject = "Test".into();

        // Missing body
        assert!(msg.validate().is_err());

        msg.text = Some("Hello".into());

        // Valid
        assert!(msg.validate().is_ok());
    }

    #[test]
    fn test_recipient_count() {
        let mut msg = Message::new();
        msg.to.push(Address::new("to@example.com"));
        msg.cc.push(Address::new("cc@example.com"));
        msg.bcc.push(Address::new("bcc@example.com"));

        assert_eq!(msg.recipient_count(), 3);
    }
}
