/// Notifiable trait - can be applied to user models, etc.
pub trait Notifiable {
    /// Get notification routing info
    fn route_notification_for(&self, channel: &str) -> Option<String>;

    /// Get email address (if available)
    fn email(&self) -> Option<String> {
        self.route_notification_for("mail")
    }

    /// Get phone number (if available)
    fn phone(&self) -> Option<String> {
        self.route_notification_for("sms")
    }

    /// Get Slack user ID (if available)
    fn slack_id(&self) -> Option<String> {
        self.route_notification_for("slack")
    }
}

/// Simple notifiable recipient
#[derive(Debug, Clone)]
pub struct NotifiableRecipient {
    pub email: Option<String>,
    pub phone: Option<String>,
    pub slack_id: Option<String>,
}

impl NotifiableRecipient {
    pub fn new() -> Self {
        Self {
            email: None,
            phone: None,
            slack_id: None,
        }
    }

    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    pub fn with_phone(mut self, phone: impl Into<String>) -> Self {
        self.phone = Some(phone.into());
        self
    }

    pub fn with_slack(mut self, slack_id: impl Into<String>) -> Self {
        self.slack_id = Some(slack_id.into());
        self
    }
}

impl Default for NotifiableRecipient {
    fn default() -> Self {
        Self::new()
    }
}

impl Notifiable for NotifiableRecipient {
    fn route_notification_for(&self, channel: &str) -> Option<String> {
        match channel {
            "mail" | "email" => self.email.clone(),
            "sms" | "phone" => self.phone.clone(),
            "slack" => self.slack_id.clone(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notifiable_recipient() {
        let recipient = NotifiableRecipient::new()
            .with_email("user@example.com")
            .with_phone("+1234567890");

        assert_eq!(recipient.email(), Some("user@example.com".to_string()));
        assert_eq!(recipient.phone(), Some("+1234567890".to_string()));
    }
}
