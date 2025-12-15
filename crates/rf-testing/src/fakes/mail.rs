//! Mail fake implementation for testing
//!
//! Provides a fake Mailer implementation that records all sent mails
//! and allows assertions on what was sent.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

// Re-export types needed for the fake
pub use rf_mail::{Address, Attachment, Mail, MailBody, MailError, Mailer};

/// Record of a sent mail
#[derive(Debug, Clone)]
pub struct MailRecord {
    /// Recipient addresses (to)
    pub to: Vec<Address>,

    /// CC addresses
    pub cc: Vec<Address>,

    /// BCC addresses
    pub bcc: Vec<Address>,

    /// From address
    pub from: Address,

    /// Reply-to address
    pub reply_to: Option<Address>,

    /// Mail subject
    pub subject: String,

    /// Mail body (HTML, Text, or both)
    pub body: MailBody,

    /// File attachments
    pub attachments: Vec<Attachment>,

    /// When the mail was recorded/sent
    pub sent_at: chrono::DateTime<chrono::Utc>,
}

impl From<Mail> for MailRecord {
    fn from(mail: Mail) -> Self {
        Self {
            to: mail.to,
            cc: mail.cc,
            bcc: mail.bcc,
            from: mail.from,
            reply_to: mail.reply_to,
            subject: mail.subject,
            body: mail.body,
            attachments: mail.attachments,
            sent_at: chrono::Utc::now(),
        }
    }
}

/// Mail fake for testing
///
/// Records all mails that are sent and provides
/// assertion methods to verify behavior.
///
/// # Example
///
/// ```ignore
/// use rf_testing::fakes::MailFake;
/// use rf_mail::{MailBuilder, Address};
///
/// #[tokio::test]
/// async fn test_sends_welcome_email() {
///     let fake = MailFake::new();
///
///     // Send some mails
///     let mail = MailBuilder::new()
///         .from(Address::new("noreply@example.com"))
///         .to(Address::new("user@example.com"))
///         .subject("Welcome!")
///         .text("Welcome to our app!")
///         .build()
///         .unwrap();
///
///     fake.send(mail).await.unwrap();
///
///     // Assert
///     fake.assert_sent("Welcome!");
///     fake.assert_sent_to("user@example.com");
///     fake.assert_sent_times("Welcome!", 1);
/// }
/// ```
#[derive(Clone)]
pub struct MailFake {
    records: Arc<Mutex<Vec<MailRecord>>>,
}

impl MailFake {
    /// Create a new mail fake
    pub fn new() -> Self {
        Self {
            records: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all sent mails
    pub fn sent(&self) -> Vec<MailRecord> {
        self.records.lock().unwrap().clone()
    }

    /// Get mails sent to a specific email address (in to, cc, or bcc)
    pub fn sent_to(&self, email: &str) -> Vec<MailRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| {
                r.to.iter().any(|addr| addr.email == email)
                    || r.cc.iter().any(|addr| addr.email == email)
                    || r.bcc.iter().any(|addr| addr.email == email)
            })
            .cloned()
            .collect()
    }

    /// Get mails with a specific subject (exact match)
    pub fn sent_with_subject(&self, subject: &str) -> Vec<MailRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.subject == subject)
            .cloned()
            .collect()
    }

    /// Get mails where subject contains the given text
    pub fn sent_with_subject_containing(&self, text: &str) -> Vec<MailRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.subject.contains(text))
            .cloned()
            .collect()
    }

    /// Check if a mail with the given subject was sent
    pub fn has_sent(&self, subject: &str) -> bool {
        self.records
            .lock()
            .unwrap()
            .iter()
            .any(|r| r.subject == subject)
    }

    /// Check if a mail was sent to the given email address
    pub fn has_sent_to(&self, email: &str) -> bool {
        self.records.lock().unwrap().iter().any(|r| {
            r.to.iter().any(|addr| addr.email == email)
                || r.cc.iter().any(|addr| addr.email == email)
                || r.bcc.iter().any(|addr| addr.email == email)
        })
    }

    /// Assert that a mail with the given subject was sent
    ///
    /// # Panics
    ///
    /// Panics if no mail with the given subject was sent.
    pub fn assert_sent(&self, subject: &str) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| r.subject == subject) {
            panic!(
                "Failed asserting that mail with subject '{}' was sent. Sent subjects: {:?}",
                subject,
                records.iter().map(|r| &r.subject).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that a mail was sent to the given email address
    ///
    /// # Panics
    ///
    /// Panics if no mail was sent to the email address.
    pub fn assert_sent_to(&self, email: &str) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| {
            r.to.iter().any(|addr| addr.email == email)
                || r.cc.iter().any(|addr| addr.email == email)
                || r.bcc.iter().any(|addr| addr.email == email)
        }) {
            panic!("Failed asserting that mail was sent to '{}'", email);
        }
    }

    /// Assert that a mail with the given subject was sent exactly N times
    ///
    /// # Panics
    ///
    /// Panics if the mail was not sent exactly N times.
    pub fn assert_sent_times(&self, subject: &str, times: usize) {
        let records = self.records.lock().unwrap();
        let count = records.iter().filter(|r| r.subject == subject).count();

        if count != times {
            panic!(
                "Failed asserting that mail with subject '{}' was sent {} times. Actually sent {} times.",
                subject, times, count
            );
        }
    }

    /// Assert that a mail with the given subject was NOT sent
    ///
    /// # Panics
    ///
    /// Panics if the mail was sent.
    pub fn assert_not_sent(&self, subject: &str) {
        let records = self.records.lock().unwrap();

        if records.iter().any(|r| r.subject == subject) {
            panic!(
                "Failed asserting that mail with subject '{}' was not sent",
                subject
            );
        }
    }

    /// Assert that no mails were sent at all
    ///
    /// # Panics
    ///
    /// Panics if any mails were sent.
    pub fn assert_nothing_sent(&self) {
        let records = self.records.lock().unwrap();

        if !records.is_empty() {
            panic!(
                "Failed asserting that no mails were sent. {} mails were sent: {:?}",
                records.len(),
                records.iter().map(|r| &r.subject).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that a mail was sent with specific properties
    ///
    /// Uses a closure to inspect the mail record.
    pub fn assert_sent_with<F>(&self, predicate: F)
    where
        F: Fn(&MailRecord) -> bool,
    {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| predicate(r)) {
            panic!("Failed asserting that mail was sent with matching properties");
        }
    }

    /// Assert that a mail was sent from a specific address
    ///
    /// # Panics
    ///
    /// Panics if no mail was sent from the address.
    pub fn assert_sent_from(&self, email: &str) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| r.from.email == email) {
            panic!(
                "Failed asserting that mail was sent from '{}'. Sent from: {:?}",
                email,
                records.iter().map(|r| &r.from.email).collect::<Vec<_>>()
            );
        }
    }

    /// Assert that a mail with attachments was sent
    ///
    /// # Panics
    ///
    /// Panics if no mail with attachments was sent.
    pub fn assert_sent_with_attachments(&self) {
        let records = self.records.lock().unwrap();

        if !records.iter().any(|r| !r.attachments.is_empty()) {
            panic!("Failed asserting that mail with attachments was sent");
        }
    }

    /// Get the first sent mail with a specific subject
    pub fn first_sent(&self, subject: &str) -> Option<MailRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .find(|r| r.subject == subject)
            .cloned()
    }

    /// Get all sent mails with a specific subject
    pub fn all_sent(&self, subject: &str) -> Vec<MailRecord> {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.subject == subject)
            .cloned()
            .collect()
    }

    /// Clear all recorded mails
    pub fn clear(&self) {
        self.records.lock().unwrap().clear();
    }

    /// Get the total number of sent mails
    pub fn count(&self) -> usize {
        self.records.lock().unwrap().len()
    }

    /// Get the number of sent mails with a specific subject
    pub fn count_with_subject(&self, subject: &str) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| r.subject == subject)
            .count()
    }

    /// Get the number of sent mails to a specific address
    pub fn count_sent_to(&self, email: &str) -> usize {
        self.records
            .lock()
            .unwrap()
            .iter()
            .filter(|r| {
                r.to.iter().any(|addr| addr.email == email)
                    || r.cc.iter().any(|addr| addr.email == email)
                    || r.bcc.iter().any(|addr| addr.email == email)
            })
            .count()
    }

    /// Record a mail send manually
    pub fn record_send(&self, record: MailRecord) {
        self.records.lock().unwrap().push(record);
    }
}

impl Default for MailFake {
    fn default() -> Self {
        Self::new()
    }
}

/// Implement the Mailer trait for MailFake
#[async_trait]
impl Mailer for MailFake {
    /// Send a mail (records it in memory)
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        let record = MailRecord::from(mail);
        self.record_send(record);
        Ok(())
    }

    /// Send multiple mails
    async fn send_batch(&self, mails: Vec<Mail>) -> Result<(), MailError> {
        for mail in mails {
            self.send(mail).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_fake_creation() {
        let fake = MailFake::new();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_mail_fake_clear() {
        let fake = MailFake::new();
        let record = MailRecord {
            to: vec![Address::new("test@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Test".to_string(),
            body: MailBody::Text("Test body".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        };
        fake.record_send(record);
        assert_eq!(fake.count(), 1);

        fake.clear();
        assert_eq!(fake.count(), 0);
    }

    #[test]
    fn test_assert_nothing_sent() {
        let fake = MailFake::new();
        fake.assert_nothing_sent();
    }

    #[test]
    #[should_panic(expected = "Failed asserting that no mails were sent")]
    fn test_assert_nothing_sent_fails() {
        let fake = MailFake::new();
        let record = MailRecord {
            to: vec![Address::new("test@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Test".to_string(),
            body: MailBody::Text("Test body".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        };
        fake.record_send(record);
        fake.assert_nothing_sent();
    }

    #[test]
    fn test_has_sent() {
        let fake = MailFake::new();
        let record = MailRecord {
            to: vec![Address::new("test@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Welcome".to_string(),
            body: MailBody::Text("Test body".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        };
        fake.record_send(record);

        assert!(fake.has_sent("Welcome"));
        assert!(!fake.has_sent("Goodbye"));
    }

    #[test]
    fn test_has_sent_to() {
        let fake = MailFake::new();
        let record = MailRecord {
            to: vec![Address::new("user@example.com")],
            cc: vec![Address::new("cc@example.com")],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Test".to_string(),
            body: MailBody::Text("Test body".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        };
        fake.record_send(record);

        assert!(fake.has_sent_to("user@example.com"));
        assert!(fake.has_sent_to("cc@example.com"));
        assert!(!fake.has_sent_to("other@example.com"));
    }

    #[test]
    fn test_count_with_subject() {
        let fake = MailFake::new();

        for i in 0..3 {
            let record = MailRecord {
                to: vec![Address::new(&format!("user{}@example.com", i))],
                cc: vec![],
                bcc: vec![],
                from: Address::new("sender@example.com"),
                reply_to: None,
                subject: "Newsletter".to_string(),
                body: MailBody::Text("Test body".to_string()),
                attachments: vec![],
                sent_at: chrono::Utc::now(),
            };
            fake.record_send(record);
        }

        assert_eq!(fake.count_with_subject("Newsletter"), 3);
        assert_eq!(fake.count_with_subject("Other"), 0);
    }

    #[test]
    fn test_sent_with_subject_containing() {
        let fake = MailFake::new();
        let record = MailRecord {
            to: vec![Address::new("user@example.com")],
            cc: vec![],
            bcc: vec![],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Welcome to our platform!".to_string(),
            body: MailBody::Text("Test body".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        };
        fake.record_send(record);

        let results = fake.sent_with_subject_containing("Welcome");
        assert_eq!(results.len(), 1);

        let results = fake.sent_with_subject_containing("Goodbye");
        assert_eq!(results.len(), 0);
    }
}
