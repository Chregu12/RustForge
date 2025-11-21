//! Core Mail type with body variants

use crate::{Address, Attachment};
use serde::{Deserialize, Serialize};

/// A complete mail representation with typed body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mail {
    /// Unique mail ID
    pub id: String,

    /// Recipient addresses
    pub to: Vec<Address>,

    /// CC addresses
    pub cc: Vec<Address>,

    /// BCC addresses
    pub bcc: Vec<Address>,

    /// From address
    pub from: Address,

    /// Reply-to address
    pub reply_to: Option<Address>,

    /// Subject line
    pub subject: String,

    /// Mail body (HTML, Text, or both)
    pub body: MailBody,

    /// File attachments
    pub attachments: Vec<Attachment>,
}

/// Mail body variants
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MailBody {
    /// HTML only
    Html(String),

    /// Plain text only
    Text(String),

    /// Both HTML and plain text (multipart)
    Both {
        /// HTML version
        html: String,
        /// Plain text version
        text: String,
    },
}

impl Mail {
    /// Create a new mail with defaults
    pub fn new() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            from: Address::new(""),
            reply_to: None,
            subject: String::new(),
            body: MailBody::Text(String::new()),
            attachments: Vec::new(),
        }
    }

    /// Validate the mail
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

        // Check body is not empty
        match &self.body {
            MailBody::Html(html) if html.is_empty() => {
                return Err("HTML body cannot be empty".into());
            }
            MailBody::Text(text) if text.is_empty() => {
                return Err("Text body cannot be empty".into());
            }
            MailBody::Both { html, text } if html.is_empty() && text.is_empty() => {
                return Err("At least one body part must be non-empty".into());
            }
            _ => {}
        }

        Ok(())
    }

    /// Get total recipient count
    pub fn recipient_count(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }

    /// Get total attachment size in bytes
    pub fn attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.size()).sum()
    }

    /// Check if mail has HTML body
    pub fn has_html(&self) -> bool {
        matches!(self.body, MailBody::Html(_) | MailBody::Both { .. })
    }

    /// Check if mail has text body
    pub fn has_text(&self) -> bool {
        matches!(self.body, MailBody::Text(_) | MailBody::Both { .. })
    }

    /// Get HTML body if present
    pub fn html(&self) -> Option<&str> {
        match &self.body {
            MailBody::Html(html) => Some(html),
            MailBody::Both { html, .. } => Some(html),
            _ => None,
        }
    }

    /// Get text body if present
    pub fn text(&self) -> Option<&str> {
        match &self.body {
            MailBody::Text(text) => Some(text),
            MailBody::Both { text, .. } => Some(text),
            _ => None,
        }
    }
}

impl Default for Mail {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_new() {
        let mail = Mail::new();
        assert!(!mail.id.is_empty());
        assert!(mail.to.is_empty());
    }

    #[test]
    fn test_mail_validate() {
        let mut mail = Mail::new();

        // Missing from
        assert!(mail.validate().is_err());

        mail.from = Address::new("sender@example.com");

        // Missing to
        assert!(mail.validate().is_err());

        mail.to.push(Address::new("recipient@example.com"));

        // Missing subject
        assert!(mail.validate().is_err());

        mail.subject = "Test".into();

        // Empty body
        assert!(mail.validate().is_err());

        mail.body = MailBody::Text("Hello".into());

        // Valid
        assert!(mail.validate().is_ok());
    }

    #[test]
    fn test_mail_body_variants() {
        let html_mail = Mail {
            body: MailBody::Html("<h1>Hello</h1>".into()),
            ..Mail::new()
        };
        assert!(html_mail.has_html());
        assert!(!html_mail.has_text());

        let text_mail = Mail {
            body: MailBody::Text("Hello".into()),
            ..Mail::new()
        };
        assert!(!text_mail.has_html());
        assert!(text_mail.has_text());

        let both_mail = Mail {
            body: MailBody::Both {
                html: "<h1>Hello</h1>".into(),
                text: "Hello".into(),
            },
            ..Mail::new()
        };
        assert!(both_mail.has_html());
        assert!(both_mail.has_text());
    }

    #[test]
    fn test_mail_getters() {
        let mail = Mail {
            body: MailBody::Both {
                html: "<h1>Hello</h1>".into(),
                text: "Hello".into(),
            },
            ..Mail::new()
        };

        assert_eq!(mail.html(), Some("<h1>Hello</h1>"));
        assert_eq!(mail.text(), Some("Hello"));
    }
}
