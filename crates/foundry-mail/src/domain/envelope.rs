use super::{Address, AddressList};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Email envelope containing addressing and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    pub from: Address,
    pub reply_to: Option<Address>,
    pub to: AddressList,
    pub cc: AddressList,
    pub bcc: AddressList,
    pub subject: String,
    pub date: DateTime<Utc>,
    pub headers: Vec<(String, String)>,
}

impl Envelope {
    pub fn new(from: Address, to: impl Into<AddressList>, subject: impl Into<String>) -> Self {
        Self {
            from,
            reply_to: None,
            to: to.into(),
            cc: AddressList::new(),
            bcc: AddressList::new(),
            subject: subject.into(),
            date: Utc::now(),
            headers: Vec::new(),
        }
    }

    pub fn reply_to(mut self, reply_to: Address) -> Self {
        self.reply_to = Some(reply_to);
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

    pub fn add_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((key.into(), value.into()));
        self
    }

    pub fn recipients(&self) -> impl Iterator<Item = &Address> {
        self.to.iter()
            .chain(self.cc.iter())
            .chain(self.bcc.iter())
    }

    pub fn recipient_count(&self) -> usize {
        self.to.len() + self.cc.len() + self.bcc.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_creation() {
        let from = Address::new("sender@example.com");
        let to = Address::new("recipient@example.com");
        let envelope = Envelope::new(from.clone(), to.clone(), "Test Subject");

        assert_eq!(envelope.from, from);
        assert_eq!(envelope.to.len(), 1);
        assert_eq!(envelope.subject, "Test Subject");
    }

    #[test]
    fn test_envelope_recipients() {
        let envelope = Envelope::new(
            Address::new("sender@example.com"),
            Address::new("to@example.com"),
            "Test",
        )
        .cc(Address::new("cc@example.com"))
        .bcc(Address::new("bcc@example.com"));

        assert_eq!(envelope.recipient_count(), 3);
    }
}
