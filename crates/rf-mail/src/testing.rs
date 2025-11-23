//! Testing utilities for email

use crate::{Mail, MailError, Mailer, Message};
use async_trait::async_trait;
use parking_lot::Mutex;
use std::sync::Arc;

/// Global mail fake instance
static MAIL_FAKE: Mutex<Option<Arc<MailFake>>> = Mutex::new(None);

/// Enable mail faking for testing
///
/// After calling this, all emails will be captured instead of sent.
///
/// # Example
///
/// ```
/// use rf_mail::testing::{fake, restore, assert_sent};
///
/// fake();
///
/// // Send emails...
///
/// assert_sent(|mail| mail.subject.contains("Welcome"));
///
/// restore();
/// ```
pub fn fake() -> Arc<MailFake> {
    let fake = Arc::new(MailFake::new());
    *MAIL_FAKE.lock() = Some(fake.clone());
    fake
}

/// Restore normal mail sending (disable faking)
pub fn restore() {
    *MAIL_FAKE.lock() = None;
}

/// Get the current mail fake instance
pub fn get_fake() -> Option<Arc<MailFake>> {
    MAIL_FAKE.lock().clone()
}

/// Assert that an email was sent matching the predicate
///
/// # Panics
///
/// Panics if no matching email was found.
pub fn assert_sent<F>(predicate: F)
where
    F: Fn(&Mail) -> bool,
{
    let fake = get_fake().expect("Mail fake not enabled. Call fake() first.");
    fake.assert_sent(predicate);
}

/// Assert that no email matching the predicate was sent
///
/// # Panics
///
/// Panics if a matching email was found.
pub fn assert_not_sent<F>(predicate: F)
where
    F: Fn(&Mail) -> bool,
{
    let fake = get_fake().expect("Mail fake not enabled. Call fake() first.");
    fake.assert_not_sent(predicate);
}

/// Assert the number of emails sent
///
/// # Panics
///
/// Panics if the count doesn't match.
pub fn assert_sent_count(count: usize) {
    let fake = get_fake().expect("Mail fake not enabled. Call fake() first.");
    fake.assert_sent_count(count);
}

/// Mail fake that captures sent emails
pub struct MailFake {
    sent: Arc<Mutex<Vec<Mail>>>,
}

impl MailFake {
    /// Create a new mail fake
    pub fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all sent messages
    pub fn sent_messages(&self) -> Vec<Mail> {
        self.sent.lock().clone()
    }

    /// Clear all sent messages
    pub fn clear(&self) {
        self.sent.lock().clear();
    }

    /// Assert that an email was sent matching the predicate
    ///
    /// # Panics
    ///
    /// Panics if no matching email was found.
    pub fn assert_sent<F>(&self, predicate: F)
    where
        F: Fn(&Mail) -> bool,
    {
        let sent = self.sent.lock();
        let found = sent.iter().any(|msg| predicate(msg));

        if !found {
            panic!(
                "Expected to find email matching predicate, but none found. Sent {} emails.",
                sent.len()
            );
        }
    }

    /// Assert that no email matching the predicate was sent
    ///
    /// # Panics
    ///
    /// Panics if a matching email was found.
    pub fn assert_not_sent<F>(&self, predicate: F)
    where
        F: Fn(&Mail) -> bool,
    {
        let sent = self.sent.lock();
        let found = sent.iter().any(|msg| predicate(msg));

        if found {
            panic!("Expected not to find email matching predicate, but one was found.");
        }
    }

    /// Assert the number of emails sent
    ///
    /// # Panics
    ///
    /// Panics if the count doesn't match.
    pub fn assert_sent_count(&self, count: usize) {
        let sent = self.sent.lock();
        assert_eq!(
            sent.len(),
            count,
            "Expected {} emails to be sent, but {} were sent",
            count,
            sent.len()
        );
    }

    /// Assert that at least one email was sent
    ///
    /// # Panics
    ///
    /// Panics if no emails were sent.
    pub fn assert_sent_any(&self) {
        let sent = self.sent.lock();
        assert!(
            !sent.is_empty(),
            "Expected at least one email to be sent, but none were sent"
        );
    }

    /// Assert that no emails were sent
    ///
    /// # Panics
    ///
    /// Panics if any emails were sent.
    pub fn assert_nothing_sent(&self) {
        self.assert_sent_count(0);
    }

    /// Get emails sent to a specific address
    pub fn sent_to(&self, email: &str) -> Vec<Mail> {
        self.sent
            .lock()
            .iter()
            .filter(|msg| msg.to.iter().any(|addr| addr.email == email))
            .cloned()
            .collect()
    }

    /// Get emails with a specific subject
    pub fn with_subject(&self, subject: &str) -> Vec<Mail> {
        self.sent
            .lock()
            .iter()
            .filter(|msg| msg.subject.contains(subject))
            .cloned()
            .collect()
    }
}

impl Default for MailFake {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Mailer for MailFake {
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        self.sent.lock().push(mail.clone());
        Ok(())
    }
}

/// Fake mailer for use in Mail type conversion
pub struct FakeMailer {
    fake: Arc<MailFake>,
}

impl FakeMailer {
    /// Create a new fake mailer
    pub fn new(fake: Arc<MailFake>) -> Self {
        Self { fake }
    }

    /// Get the underlying fake
    pub fn fake(&self) -> &MailFake {
        &self.fake
    }
}

#[async_trait]
impl Mailer for FakeMailer {
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        self.fake.send(mail).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MessageBuilder};

    #[tokio::test]
    async fn test_mail_fake() {
        let fake = MailFake::new();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        fake.send(&mail).await.unwrap();

        assert_eq!(fake.sent_messages().len(), 1);
        fake.assert_sent_count(1);
        fake.assert_sent(|msg| msg.subject == "Test");
    }

    #[tokio::test]
    async fn test_assert_sent() {
        let fake = MailFake::new();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Welcome Email")
            .text("Hello")
            .build()
            .unwrap();

        fake.send(&mail).await.unwrap();

        fake.assert_sent(|msg| msg.subject.contains("Welcome"));
        fake.assert_not_sent(|msg| msg.subject.contains("Password"));
    }

    #[tokio::test]
    async fn test_sent_to() {
        let fake = MailFake::new();

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        fake.send(&mail).await.unwrap();

        let sent = fake.sent_to("user@example.com");
        assert_eq!(sent.len(), 1);

        let not_sent = fake.sent_to("other@example.com");
        assert_eq!(not_sent.len(), 0);
    }

    #[test]
    fn test_global_fake() {
        // Enable faking
        let fake = fake();

        // Check it's enabled
        assert!(get_fake().is_some());

        // Restore
        restore();

        // Check it's disabled
        assert!(get_fake().is_none());
    }
}
