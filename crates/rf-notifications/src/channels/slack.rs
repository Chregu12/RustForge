//! Slack notification channel with webhook support

use crate::channels::NotificationChannel;
use crate::messages::SlackMessage;
use crate::{Notifiable, Notification, NotificationError, NotificationResult};
use async_trait::async_trait;
use serde::Serialize;

/// Slack channel that sends notifications via webhooks
pub struct SlackChannel {
    default_webhook_url: Option<String>,
    client: reqwest::Client,
}

impl SlackChannel {
    /// Create a new Slack channel
    pub fn new() -> Self {
        Self {
            default_webhook_url: None,
            client: reqwest::Client::new(),
        }
    }

    /// Create a new Slack channel with a default webhook URL
    pub fn with_webhook(webhook_url: impl Into<String>) -> Self {
        Self {
            default_webhook_url: Some(webhook_url.into()),
            client: reqwest::Client::new(),
        }
    }
}

impl Default for SlackChannel {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationChannel for SlackChannel {
    async fn send(
        &self,
        notification: &dyn Notification,
        notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        // Get Slack message from notification
        let slack_message = notification.to_slack().await.ok_or_else(|| {
            NotificationError::ChannelError("No Slack message provided".to_string())
        })?;

        // Get webhook URL
        let webhook_url = notifiable
            .route_notification_for_slack()
            .or_else(|| self.default_webhook_url.clone())
            .ok_or_else(|| {
                NotificationError::RoutingError("No Slack webhook URL found".to_string())
            })?;

        // Send to Slack
        let payload = SlackPayload::from_message(slack_message);

        let response = self.client.post(&webhook_url).json(&payload).send().await?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(NotificationError::SendError(format!(
                "Slack API error: {}",
                error_text
            )));
        }

        Ok(())
    }
}

#[derive(Serialize)]
struct SlackPayload {
    text: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    attachments: Vec<SlackAttachmentPayload>,
}

#[derive(Serialize)]
struct SlackAttachmentPayload {
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    text: String,
    color: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    fields: Vec<SlackFieldPayload>,
}

#[derive(Serialize)]
struct SlackFieldPayload {
    title: String,
    value: String,
    short: bool,
}

impl SlackPayload {
    fn from_message(msg: SlackMessage) -> Self {
        Self {
            text: msg.text,
            attachments: msg
                .attachments
                .into_iter()
                .map(|a| SlackAttachmentPayload {
                    title: a.title,
                    text: a.text,
                    color: a.color,
                    fields: a
                        .fields
                        .into_iter()
                        .map(|f| SlackFieldPayload {
                            title: f.title,
                            value: f.value,
                            short: f.short,
                        })
                        .collect(),
                })
                .collect(),
        }
    }
}

/// Mock Slack channel for testing
pub struct MockSlackChannel {
    sent: Arc<tokio::sync::RwLock<Vec<SlackMessage>>>,
}

impl MockSlackChannel {
    pub fn new() -> Self {
        Self {
            sent: Arc::new(tokio::sync::RwLock::new(Vec::new())),
        }
    }

    pub async fn sent_messages(&self) -> Vec<SlackMessage> {
        self.sent.read().await.clone()
    }
}

impl Default for MockSlackChannel {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::Arc;

#[async_trait]
impl NotificationChannel for MockSlackChannel {
    async fn send(
        &self,
        notification: &dyn Notification,
        _notifiable: &dyn Notifiable,
    ) -> NotificationResult<()> {
        if let Some(slack_message) = notification.to_slack().await {
            self.sent.write().await.push(slack_message);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{SlackAttachment, SlackMessage};

    struct TestUser;

    impl Notifiable for TestUser {
        fn route_notification_for_slack(&self) -> Option<String> {
            Some("https://hooks.slack.com/test".to_string())
        }
    }

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<crate::Channel> {
            vec![crate::Channel::Slack]
        }

        async fn to_slack(&self) -> Option<SlackMessage> {
            Some(
                SlackMessage::new("Test Slack message").attachment(
                    SlackAttachment::new("Attachment text")
                        .title("Title")
                        .color("good")
                        .field("Field 1", "Value 1", true),
                ),
            )
        }
    }

    #[tokio::test]
    async fn test_slack_channel_with_mock() {
        let channel = MockSlackChannel::new();
        let user = TestUser;
        let notification = TestNotification;

        channel.send(&notification, &user).await.unwrap();

        // Verify Slack message was sent
        let sent = channel.sent_messages().await;
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].text, "Test Slack message");
        assert_eq!(sent[0].attachments.len(), 1);
    }
}
