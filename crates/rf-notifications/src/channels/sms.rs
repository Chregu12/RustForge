//! SMS notification channel with provider support (Twilio, etc.)

use crate::channels::NotificationChannel;
use crate::{Notifiable, Notification, NotificationError, NotificationResult};
use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;

/// SMS channel that uses a provider
pub struct SmsChannel {
    provider: Arc<dyn SmsProvider>,
}

impl SmsChannel {
    /// Create a new SMS channel with a provider
    pub fn new(provider: Arc<dyn SmsProvider>) -> Self {
        Self { provider }
    }
}

#[async_trait]
impl NotificationChannel for SmsChannel {
    async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        // Get SMS message from notification
        let sms_message = notification
            .to_sms()
            .await
            .ok_or_else(|| NotificationError::ChannelError("No SMS message provided".to_string()))?;

        // Get recipient phone number
        let to_phone = notifiable.route_notification_for_sms().ok_or_else(|| {
            NotificationError::RoutingError("No phone number found".to_string())
        })?;

        // Send via provider
        self.provider.send(&to_phone, &sms_message.content).await?;

        Ok(())
    }
}

/// SMS provider trait - implement this for different SMS services
#[async_trait]
pub trait SmsProvider: Send + Sync {
    /// Send an SMS message
    async fn send(&self, to: &str, message: &str) -> NotificationResult<()>;
}

/// Twilio SMS provider
pub struct TwilioProvider {
    account_sid: String,
    auth_token: String,
    from_number: String,
    client: reqwest::Client,
}

impl TwilioProvider {
    /// Create a new Twilio provider
    pub fn new(
        account_sid: impl Into<String>,
        auth_token: impl Into<String>,
        from_number: impl Into<String>,
    ) -> Self {
        Self {
            account_sid: account_sid.into(),
            auth_token: auth_token.into(),
            from_number: from_number.into(),
            client: reqwest::Client::new(),
        }
    }
}

#[derive(Serialize)]
struct TwilioRequest {
    #[serde(rename = "To")]
    to: String,
    #[serde(rename = "From")]
    from: String,
    #[serde(rename = "Body")]
    body: String,
}

#[async_trait]
impl SmsProvider for TwilioProvider {
    async fn send(&self, to: &str, message: &str) -> NotificationResult<()> {
        let url = format!(
            "https://api.twilio.com/2010-04-01/Accounts/{}/Messages.json",
            self.account_sid
        );

        let params = TwilioRequest {
            to: to.to_string(),
            from: self.from_number.clone(),
            body: message.to_string(),
        };

        let response = self
            .client
            .post(&url)
            .basic_auth(&self.account_sid, Some(&self.auth_token))
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            return Err(NotificationError::SendError(format!(
                "Twilio API error: {}",
                error_text
            )));
        }

        Ok(())
    }
}

/// Mock SMS provider for testing
pub struct MockSmsProvider {
    sent: Arc<tokio::sync::RwLock<Vec<(String, String)>>>,
}

impl MockSmsProvider {
    pub fn new() -> Self {
        Self {
            sent: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    pub async fn sent_messages(&self) -> Vec<(String, String)> {
        self.sent.read().await.clone()
    }
}

impl Default for MockSmsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SmsProvider for MockSmsProvider {
    async fn send(&self, to: &str, message: &str) -> NotificationResult<()> {
        self.sent.write().await.push((to.to_string(), message.to_string()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::SmsMessage;

    struct TestUser {
        phone: String,
    }

    impl Notifiable for TestUser {
        fn route_notification_for_sms(&self) -> Option<String> {
            Some(self.phone.clone())
        }
    }

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<crate::Channel> {
            vec![crate::Channel::Sms]
        }

        async fn to_sms(&self) -> Option<SmsMessage> {
            Some(SmsMessage::new("Test SMS message"))
        }
    }

    #[tokio::test]
    async fn test_sms_channel_with_mock_provider() {
        let provider = Arc::new(MockSmsProvider::new());
        let channel = SmsChannel::new(provider.clone());

        let user = TestUser {
            phone: "+1234567890".to_string(),
        };

        let notification = TestNotification;
        channel.send(&notification, &user).await.unwrap();

        // Verify SMS was sent
        let sent = provider.sent_messages().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].0, "+1234567890");
        assert_eq!(sent[0].1, "Test SMS message");
    }
}
