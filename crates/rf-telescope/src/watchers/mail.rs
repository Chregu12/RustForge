//! Mail watcher for email preview and monitoring

use crate::{Entry, EntryType, Storage};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;

/// Mail information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailInfo {
    pub from: String,
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub html: Option<String>,
    pub text: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
    pub headers: HashMap<String, String>,
    pub sent_at: DateTime<Utc>,
}

/// Attachment information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub filename: String,
    pub content_type: String,
    pub size_bytes: usize,
}

impl MailInfo {
    /// Create a new mail info
    pub fn new(from: impl Into<String>, subject: impl Into<String>) -> Self {
        Self {
            from: from.into(),
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            subject: subject.into(),
            html: None,
            text: None,
            attachments: Vec::new(),
            headers: HashMap::new(),
            sent_at: Utc::now(),
        }
    }

    /// Add a recipient
    pub fn to(mut self, email: impl Into<String>) -> Self {
        self.to.push(email.into());
        self
    }

    /// Add a CC recipient
    pub fn cc(mut self, email: impl Into<String>) -> Self {
        self.cc.push(email.into());
        self
    }

    /// Add a BCC recipient
    pub fn bcc(mut self, email: impl Into<String>) -> Self {
        self.bcc.push(email.into());
        self
    }

    /// Set HTML content
    pub fn with_html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    /// Set plain text content
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Add an attachment
    pub fn with_attachment(
        mut self,
        filename: impl Into<String>,
        content_type: impl Into<String>,
        size_bytes: usize,
    ) -> Self {
        self.attachments.push(AttachmentInfo {
            filename: filename.into(),
            content_type: content_type.into(),
            size_bytes,
        });
        self
    }

    /// Add a header
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Mail watcher for monitoring sent emails
#[derive(Clone)]
pub struct MailWatcher {
    storage: Storage,
}

impl MailWatcher {
    /// Create a new mail watcher
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    /// Record a sent email
    pub async fn record(&self, info: MailInfo) {
        let entry = Entry::new(
            EntryType::Mail,
            json!({
                "from": info.from,
                "to": info.to,
                "cc": info.cc,
                "bcc": info.bcc,
                "subject": info.subject,
                "html": info.html,
                "text": info.text,
                "attachments": info.attachments,
                "headers": info.headers,
                "sent_at": info.sent_at,
            }),
        )
        .with_tag(format!("from:{}", info.from));

        self.storage.store(entry).await;
    }

    /// Get all recorded emails
    pub async fn all(&self) -> Vec<Entry> {
        self.storage.by_type(EntryType::Mail).await
    }

    /// Get emails from a specific sender
    pub async fn from(&self, sender: &str) -> Vec<Entry> {
        let all = self.all().await;
        let tag = format!("from:{}", sender);
        all.into_iter()
            .filter(|entry| entry.tags.contains(&tag))
            .collect()
    }

    /// Get emails with attachments
    pub async fn with_attachments(&self) -> Vec<Entry> {
        let all = self.all().await;
        all.into_iter()
            .filter(|entry| {
                entry
                    .content
                    .get("attachments")
                    .and_then(|a| a.as_array())
                    .map(|arr| !arr.is_empty())
                    .unwrap_or(false)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mail_info_creation() {
        let info = MailInfo::new("noreply@example.com", "Welcome!")
            .to("user@example.com")
            .with_html("<h1>Welcome</h1>")
            .with_text("Welcome");

        assert_eq!(info.from, "noreply@example.com");
        assert_eq!(info.subject, "Welcome!");
        assert_eq!(info.to, vec!["user@example.com"]);
        assert!(info.html.is_some());
        assert!(info.text.is_some());
    }

    #[tokio::test]
    async fn test_mail_watcher_record() {
        let storage = Storage::new();
        let watcher = MailWatcher::new(storage);

        let info = MailInfo::new("admin@example.com", "Test Email")
            .to("user@example.com")
            .with_html("<p>Test</p>");

        watcher.record(info).await;

        let emails = watcher.all().await;
        assert_eq!(emails.len(), 1);
        assert_eq!(emails[0].content["subject"], "Test Email");
    }

    #[tokio::test]
    async fn test_mail_from_sender() {
        let storage = Storage::new();
        let watcher = MailWatcher::new(storage);

        watcher.record(MailInfo::new("admin@example.com", "Admin Email")).await;
        watcher.record(MailInfo::new("noreply@example.com", "Automated Email")).await;
        watcher.record(MailInfo::new("admin@example.com", "Another Admin Email")).await;

        let admin_emails = watcher.from("admin@example.com").await;
        assert_eq!(admin_emails.len(), 2);
    }

    #[tokio::test]
    async fn test_mail_with_attachments() {
        let storage = Storage::new();
        let watcher = MailWatcher::new(storage);

        watcher.record(MailInfo::new("test@example.com", "Plain Email")).await;
        watcher.record(
            MailInfo::new("test@example.com", "Email with Attachment")
                .with_attachment("document.pdf", "application/pdf", 1024)
        ).await;
        watcher.record(
            MailInfo::new("test@example.com", "Email with Image")
                .with_attachment("photo.jpg", "image/jpeg", 2048)
        ).await;

        let with_attachments = watcher.with_attachments().await;
        assert_eq!(with_attachments.len(), 2);
    }
}
