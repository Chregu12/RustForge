//! Notification channels for task completion

use crate::{EnvoyError, EnvoyResult, TaskResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Notification trait
#[async_trait]
pub trait Notifier: Send + Sync {
    /// Called when a task starts
    async fn on_start(&self, task: &str) -> EnvoyResult<()>;

    /// Called when a task succeeds
    async fn on_success(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()>;

    /// Called when a task fails
    async fn on_failure(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()>;
}

/// Notification channel types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationChannel {
    Slack { webhook_url: String },
    Discord { webhook_url: String },
    Telegram { bot_token: String, chat_id: String },
    Email { to: String, from: String },
}

/// Slack notifier
pub struct SlackNotifier {
    webhook_url: String,
    channel: Option<String>,
    username: Option<String>,
}

impl SlackNotifier {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
            channel: None,
            username: None,
        }
    }

    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self
    }

    async fn send(&self, message: &SlackMessage) -> EnvoyResult<()> {
        let client = reqwest::Client::new();

        let _response = client
            .post(&self.webhook_url)
            .json(message)
            .send()
            .await
            .map_err(|e: reqwest::Error| EnvoyError::NotificationError(e.to_string()))?;

        Ok(())
    }
}

#[derive(Serialize)]
struct SlackMessage {
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachments: Option<Vec<SlackAttachment>>,
}

#[derive(Serialize)]
struct SlackAttachment {
    color: String,
    title: String,
    text: String,
    fields: Vec<SlackField>,
}

#[derive(Serialize)]
struct SlackField {
    title: String,
    value: String,
    short: bool,
}

#[async_trait]
impl Notifier for SlackNotifier {
    async fn on_start(&self, task: &str) -> EnvoyResult<()> {
        let message = SlackMessage {
            text: format!("🚀 Starting task: {}", task),
            channel: self.channel.clone(),
            username: self.username.clone(),
            attachments: None,
        };

        self.send(&message).await
    }

    async fn on_success(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()> {
        let fields: Vec<SlackField> = results
            .iter()
            .map(|r| SlackField {
                title: r.server.clone(),
                value: format!("✅ Completed in {:.2}s", r.duration.as_secs_f64()),
                short: true,
            })
            .collect();

        let message = SlackMessage {
            text: format!("✅ Task completed successfully: {}", task),
            channel: self.channel.clone(),
            username: self.username.clone(),
            attachments: Some(vec![SlackAttachment {
                color: "good".to_string(),
                title: format!("Task: {}", task),
                text: "All servers completed successfully".to_string(),
                fields,
            }]),
        };

        self.send(&message).await
    }

    async fn on_failure(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()> {
        let failed: Vec<&TaskResult> = results.iter().filter(|r| !r.success).collect();

        let fields: Vec<SlackField> = failed
            .iter()
            .map(|r| SlackField {
                title: r.server.clone(),
                value: format!("❌ Exit code: {}\n{}", r.exit_code, r.stderr.chars().take(200).collect::<String>()),
                short: false,
            })
            .collect();

        let message = SlackMessage {
            text: format!("❌ Task failed: {}", task),
            channel: self.channel.clone(),
            username: self.username.clone(),
            attachments: Some(vec![SlackAttachment {
                color: "danger".to_string(),
                title: format!("Task: {}", task),
                text: format!("{} server(s) failed", failed.len()),
                fields,
            }]),
        };

        self.send(&message).await
    }
}

/// Discord notifier
pub struct DiscordNotifier {
    webhook_url: String,
}

impl DiscordNotifier {
    pub fn new(webhook_url: impl Into<String>) -> Self {
        Self {
            webhook_url: webhook_url.into(),
        }
    }

    async fn send(&self, content: &str, embeds: Option<Vec<DiscordEmbed>>) -> EnvoyResult<()> {
        let client = reqwest::Client::new();

        let body = serde_json::json!({
            "content": content,
            "embeds": embeds
        });

        let _response = client
            .post(&self.webhook_url)
            .json(&body)
            .send()
            .await
            .map_err(|e: reqwest::Error| EnvoyError::NotificationError(e.to_string()))?;

        Ok(())
    }
}

#[derive(Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<DiscordField>,
}

#[derive(Serialize)]
struct DiscordField {
    name: String,
    value: String,
    inline: bool,
}

#[async_trait]
impl Notifier for DiscordNotifier {
    async fn on_start(&self, task: &str) -> EnvoyResult<()> {
        self.send(&format!("🚀 Starting task: {}", task), None).await
    }

    async fn on_success(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()> {
        let fields: Vec<DiscordField> = results
            .iter()
            .map(|r| DiscordField {
                name: r.server.clone(),
                value: format!("✅ {:.2}s", r.duration.as_secs_f64()),
                inline: true,
            })
            .collect();

        let embeds = vec![DiscordEmbed {
            title: format!("Task: {}", task),
            description: "Completed successfully".to_string(),
            color: 0x00FF00, // Green
            fields,
        }];

        self.send(&format!("✅ Task completed: {}", task), Some(embeds)).await
    }

    async fn on_failure(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()> {
        let failed: Vec<&TaskResult> = results.iter().filter(|r| !r.success).collect();

        let fields: Vec<DiscordField> = failed
            .iter()
            .map(|r| DiscordField {
                name: r.server.clone(),
                value: format!("Exit: {}", r.exit_code),
                inline: true,
            })
            .collect();

        let embeds = vec![DiscordEmbed {
            title: format!("Task: {}", task),
            description: format!("{} server(s) failed", failed.len()),
            color: 0xFF0000, // Red
            fields,
        }];

        self.send(&format!("❌ Task failed: {}", task), Some(embeds)).await
    }
}

/// Console notifier (for local output)
pub struct ConsoleNotifier;

#[async_trait]
impl Notifier for ConsoleNotifier {
    async fn on_start(&self, task: &str) -> EnvoyResult<()> {
        println!("🚀 Starting task: {}", task);
        Ok(())
    }

    async fn on_success(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()> {
        println!("✅ Task '{}' completed successfully", task);
        for result in results {
            println!(
                "   {} - {:.2}s",
                result.server,
                result.duration.as_secs_f64()
            );
        }
        Ok(())
    }

    async fn on_failure(&self, task: &str, results: &[TaskResult]) -> EnvoyResult<()> {
        println!("❌ Task '{}' failed", task);
        for result in results.iter().filter(|r| !r.success) {
            println!("   {} - Exit code: {}", result.server, result.exit_code);
            if !result.stderr.is_empty() {
                println!("   Error: {}", result.stderr.lines().next().unwrap_or(""));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_console_notifier() {
        let notifier = ConsoleNotifier;
        assert!(notifier.on_start("test").await.is_ok());
    }
}
