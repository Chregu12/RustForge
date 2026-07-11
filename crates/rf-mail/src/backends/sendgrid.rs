//! SendGrid email backend
//!
//! Popular email delivery service with excellent deliverability and analytics.
//!
//! # Features
//! - Simple REST API
//! - Dynamic templates
//! - Marketing campaigns
//! - Email validation
//! - Detailed analytics
//! - List management
//!
//! # Configuration
//!
//! ```toml
//! [mail.sendgrid]
//! api_key = "your-api-key"
//! sandbox_mode = false
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use rf_mail::{SendGridMailer, SendGridConfig, Mailer, MailBuilder, Address};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = SendGridConfig {
//!     api_key: "your-api-key".to_string(),
//!     sandbox_mode: false,
//!     ..Default::default()
//! };
//!
//! let mailer = SendGridMailer::new(config);
//!
//! let mail = MailBuilder::new()
//!     .from(Address::new("sender@example.com"))
//!     .to(Address::new("recipient@example.com"))
//!     .subject("Test Email")
//!     .text("Hello from SendGrid!")
//!     .build()?;
//!
//! mailer.send(mail).await?;
//! # Ok(())
//! # }
//! ```

use crate::{
    error::{MailError, MailResult},
    Mail, Mailer,
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// SendGrid mailer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendGridConfig {
    /// SendGrid API key
    pub api_key: String,

    /// Sandbox mode (mail won't actually be sent)
    #[serde(default)]
    pub sandbox_mode: bool,

    /// IP pool name (optional)
    pub ip_pool_name: Option<String>,

    /// Custom args (metadata)
    #[serde(default)]
    pub custom_args: std::collections::HashMap<String, String>,

    /// Categories/tags
    #[serde(default)]
    pub categories: Vec<String>,

    /// Click tracking enabled
    #[serde(default = "default_true")]
    pub click_tracking: bool,

    /// Open tracking enabled
    #[serde(default = "default_true")]
    pub open_tracking: bool,
}

fn default_true() -> bool {
    true
}

impl Default for SendGridConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            sandbox_mode: false,
            ip_pool_name: None,
            custom_args: std::collections::HashMap::new(),
            categories: Vec::new(),
            click_tracking: true,
            open_tracking: true,
        }
    }
}

/// SendGrid email backend
pub struct SendGridMailer {
    config: SendGridConfig,
    client: reqwest::Client,
}

impl SendGridMailer {
    /// Create a new SendGrid mailer
    pub fn new(config: SendGridConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    /// Send using SendGrid API v3
    async fn send_via_api(&self, mail: &Mail) -> MailResult<()> {
        let from = &mail.from;

        // Build personalizations (recipients)
        let mut personalizations = Vec::new();

        // Build to addresses
        let to_addresses: Vec<serde_json::Value> = mail
            .to
            .iter()
            .map(|addr| {
                let mut obj = json!({ "email": addr.email });
                if let Some(name) = &addr.name {
                    obj["name"] = json!(name);
                }
                obj
            })
            .collect();

        let mut personalization = json!({
            "to": to_addresses,
        });

        // Add CC
        if !mail.cc.is_empty() {
            let cc_addresses: Vec<serde_json::Value> = mail
                .cc
                .iter()
                .map(|addr| {
                    let mut obj = json!({ "email": addr.email });
                    if let Some(name) = &addr.name {
                        obj["name"] = json!(name);
                    }
                    obj
                })
                .collect();
            personalization["cc"] = json!(cc_addresses);
        }

        // Add BCC
        if !mail.bcc.is_empty() {
            let bcc_addresses: Vec<serde_json::Value> = mail
                .bcc
                .iter()
                .map(|addr| {
                    let mut obj = json!({ "email": addr.email });
                    if let Some(name) = &addr.name {
                        obj["name"] = json!(name);
                    }
                    obj
                })
                .collect();
            personalization["bcc"] = json!(bcc_addresses);
        }

        // Add custom args
        if !self.config.custom_args.is_empty() {
            personalization["custom_args"] = json!(self.config.custom_args);
        }

        personalizations.push(personalization);

        // Build from address
        let mut from_obj = json!({ "email": from.email });
        if let Some(name) = &from.name {
            from_obj["name"] = json!(name);
        }

        // Build request body
        let mut body = json!({
            "personalizations": personalizations,
            "from": from_obj,
            "subject": mail.subject,
        });

        // Add reply-to
        if let Some(reply_to) = &mail.reply_to {
            let mut reply_obj = json!({ "email": reply_to.email });
            if let Some(name) = &reply_to.name {
                reply_obj["name"] = json!(name);
            }
            body["reply_to"] = reply_obj;
        }

        // Add content
        let mut content = Vec::new();
        match &mail.body {
            crate::MailBody::Text(text) => {
                content.push(json!({
                    "type": "text/plain",
                    "value": text,
                }));
            }
            crate::MailBody::Html(html) => {
                content.push(json!({
                    "type": "text/html",
                    "value": html,
                }));
            }
            crate::MailBody::Both { text, html } => {
                content.push(json!({
                    "type": "text/plain",
                    "value": text,
                }));
                content.push(json!({
                    "type": "text/html",
                    "value": html,
                }));
            }
        }
        body["content"] = json!(content);

        // Add attachments
        if !mail.attachments.is_empty() {
            let attachments: Vec<serde_json::Value> = mail
                .attachments
                .iter()
                .map(|att| {
                    json!({
                        "content": base64::encode(&att.data),
                        "type": att.content_type,
                        "filename": att.filename,
                    })
                })
                .collect();
            body["attachments"] = json!(attachments);
        }

        // Add categories
        if !self.config.categories.is_empty() {
            body["categories"] = json!(self.config.categories);
        }

        // Add tracking settings
        body["tracking_settings"] = json!({
            "click_tracking": {
                "enable": self.config.click_tracking,
            },
            "open_tracking": {
                "enable": self.config.open_tracking,
            },
        });

        // Add mail settings
        let mut mail_settings = json!({});

        if self.config.sandbox_mode {
            mail_settings["sandbox_mode"] = json!({
                "enable": true,
            });
        }

        if !mail_settings.as_object().unwrap().is_empty() {
            body["mail_settings"] = mail_settings;
        }

        // Add IP pool
        if let Some(pool) = &self.config.ip_pool_name {
            body["ip_pool_name"] = json!(pool);
        }

        // Send request
        let response = self
            .client
            .post("https://api.sendgrid.com/v3/mail/send")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| MailError::SendFailed(e.to_string()))?;

        let status = response.status();

        if !status.is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Failed to read error response".to_string());

            return Err(MailError::SendFailed(format!(
                "SendGrid API error ({}): {}",
                status, error_text
            )));
        }

        // SendGrid returns 202 Accepted with X-Message-Id header
        tracing::info!("Email sent via SendGrid successfully");

        Ok(())
    }
}

#[async_trait]
impl Mailer for SendGridMailer {
    async fn send(&self, mail: Mail) -> MailResult<()> {
        self.send_via_api(&mail).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MailBuilder};

    #[test]
    fn test_sendgrid_config() {
        let mut custom_args = std::collections::HashMap::new();
        custom_args.insert("campaign".to_string(), "welcome".to_string());

        let config = SendGridConfig {
            api_key: "test-key".to_string(),
            sandbox_mode: true,
            ip_pool_name: Some("transactional".to_string()),
            custom_args,
            categories: vec!["welcome".to_string()],
            click_tracking: true,
            open_tracking: false,
        };

        assert_eq!(config.api_key, "test-key");
        assert!(config.sandbox_mode);
        assert_eq!(config.ip_pool_name, Some("transactional".to_string()));
        assert_eq!(config.categories, vec!["welcome"]);
        assert!(config.click_tracking);
        assert!(!config.open_tracking);
    }

    #[test]
    fn test_sendgrid_mailer_creation() {
        let config = SendGridConfig {
            api_key: "test-key".to_string(),
            ..Default::default()
        };

        let mailer = SendGridMailer::new(config);
        assert_eq!(mailer.config.api_key, "test-key");
    }

    #[tokio::test]
    async fn test_build_mail() {
        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Body")
            .build()
            .unwrap();

        assert_eq!(mail.subject, "Test");
        assert_eq!(mail.to.len(), 1);
    }
}
