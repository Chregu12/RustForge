use super::{Channel, ChannelResult};
use crate::notification::{Notification, NotificationError};
use async_trait::async_trait;
use serde_json::json;

/// Slack notification channel
pub struct SlackChannel {
    webhook_url: String,
}

impl SlackChannel {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
        }
    }

    pub fn from_env() -> Result<Self, NotificationError> {
        let webhook_url = std::env::var("SLACK_WEBHOOK_URL")
            .map_err(|_| NotificationError::Channel("SLACK_WEBHOOK_URL not set".to_string()))?;

        Ok(Self::new(webhook_url))
    }
}

#[async_trait]
impl Channel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(&self, notification: &dyn Notification, _recipient: &dyn std::any::Any) -> ChannelResult {
        let payload = json!({
            "text": notification.title(),
            "blocks": [
                {
                    "type": "section",
                    "text": {
                        "type": "mrkdwn",
                        "text": format!("*{}*\n{}", notification.title(), notification.body())
                    }
                }
            ]
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| NotificationError::Send(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(NotificationError::Send(format!(
                "Slack API error: {}",
                response.status()
            )))
        }
    }

    async fn is_ready(&self) -> bool {
        !self.webhook_url.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slack_channel_creation() {
        let channel = SlackChannel::new("https://hooks.slack.com/test");
        assert_eq!(channel.name(), "slack");
    }
}
