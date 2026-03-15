//! Mailgun email backend
//!
//! Powerful email API with advanced features like tagging, tracking, and webhooks.
//!
//! # Features
//! - HTTP API or SMTP
//! - Tracking (opens, clicks)
//! - Tags and custom variables
//! - Scheduled sending
//! - Email validation API
//! - Route filtering
//!
//! # Configuration
//!
//! ```toml
//! [mail.mailgun]
//! api_key = "your-api-key"
//! domain = "mg.example.com"
//! region = "us"  # or "eu"
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use rf_mail::{MailgunMailer, MailgunConfig, Mailer, MailBuilder, Address};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = MailgunConfig {
//!     api_key: "your-api-key".to_string(),
//!     domain: "mg.example.com".to_string(),
//!     region: MailgunRegion::US,
//!     tags: vec!["welcome".to_string()],
//!     ..Default::default()
//! };
//!
//! let mailer = MailgunMailer::new(config);
//!
//! let mail = MailBuilder::new()
//!     .from(Address::new("sender@example.com"))
//!     .to(Address::new("recipient@example.com"))
//!     .subject("Test Email")
//!     .text("Hello from Mailgun!")
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
use std::collections::HashMap;

/// Mailgun API region
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MailgunRegion {
    /// US region (api.mailgun.net)
    #[serde(rename = "us")]
    US,

    /// EU region (api.eu.mailgun.net)
    #[serde(rename = "eu")]
    EU,
}

impl MailgunRegion {
    /// Get API base URL for region
    pub fn api_base(&self) -> &str {
        match self {
            Self::US => "https://api.mailgun.net/v3",
            Self::EU => "https://api.eu.mailgun.net/v3",
        }
    }
}

/// Mailgun mailer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailgunConfig {
    /// Mailgun API key
    pub api_key: String,

    /// Sending domain
    pub domain: String,

    /// API region
    #[serde(default = "default_region")]
    pub region: MailgunRegion,

    /// Tags for categorizing emails
    #[serde(default)]
    pub tags: Vec<String>,

    /// Enable click tracking
    #[serde(default = "default_true")]
    pub track_clicks: bool,

    /// Enable open tracking
    #[serde(default = "default_true")]
    pub track_opens: bool,

    /// Test mode (doesn't actually send)
    #[serde(default)]
    pub test_mode: bool,
}

fn default_region() -> MailgunRegion {
    MailgunRegion::US
}

fn default_true() -> bool {
    true
}

impl Default for MailgunConfig {
    fn default() -> Self {
        Self {
            api_key: String::new(),
            domain: String::new(),
            region: MailgunRegion::US,
            tags: Vec::new(),
            track_clicks: true,
            track_opens: true,
            test_mode: false,
        }
    }
}

/// Mailgun email backend
pub struct MailgunMailer {
    config: MailgunConfig,
    client: reqwest::Client,
}

impl MailgunMailer {
    /// Create a new Mailgun mailer
    pub fn new(config: MailgunConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    /// Send using Mailgun API
    async fn send_via_api(&self, mail: &Mail) -> MailResult<MailgunResponse> {
        let from = mail
            .from
            .as_ref()
            .ok_or_else(|| MailError::InvalidAddress("Missing 'from' address".to_string()))?;

        // Build form data
        let mut form = HashMap::new();

        // From address
        let from_address = if let Some(name) = &from.name {
            format!("{} <{}>", name, from.email)
        } else {
            from.email.clone()
        };
        form.insert("from", from_address);

        // To addresses — Mailgun accepts comma-separated recipients
        let to_list: Vec<String> = mail
            .to
            .iter()
            .map(|to| {
                if let Some(name) = &to.name {
                    format!("{} <{}>", name, to.email)
                } else {
                    to.email.clone()
                }
            })
            .collect();
        if !to_list.is_empty() {
            form.insert("to", to_list.join(", "));
        }

        // CC addresses
        let cc_list: Vec<String> = mail
            .cc
            .iter()
            .map(|cc| {
                if let Some(name) = &cc.name {
                    format!("{} <{}>", name, cc.email)
                } else {
                    cc.email.clone()
                }
            })
            .collect();
        if !cc_list.is_empty() {
            form.insert("cc", cc_list.join(", "));
        }

        // BCC addresses
        let bcc_list: Vec<String> = mail
            .bcc
            .iter()
            .map(|bcc| {
                if let Some(name) = &bcc.name {
                    format!("{} <{}>", name, bcc.email)
                } else {
                    bcc.email.clone()
                }
            })
            .collect();
        if !bcc_list.is_empty() {
            form.insert("bcc", bcc_list.join(", "));
        }

        // Subject
        form.insert("subject", mail.subject.clone());

        // Reply-to
        if let Some(reply_to) = &mail.reply_to {
            let reply_address = if let Some(name) = &reply_to.name {
                format!("{} <{}>", name, reply_to.email)
            } else {
                reply_to.email.clone()
            };
            form.insert("h:Reply-To", reply_address);
        }

        // Body content
        match &mail.body {
            crate::MailBody::Text(text) => {
                form.insert("text", text.clone());
            }
            crate::MailBody::Html(html) => {
                form.insert("html", html.clone());
            }
            crate::MailBody::Both { text, html } => {
                form.insert("text", text.clone());
                form.insert("html", html.clone());
            }
        }

        // Tracking
        form.insert(
            "o:tracking-clicks",
            if self.config.track_clicks {
                "yes"
            } else {
                "no"
            }
            .to_string(),
        );

        form.insert(
            "o:tracking-opens",
            if self.config.track_opens { "yes" } else { "no" }.to_string(),
        );

        // Test mode
        if self.config.test_mode {
            form.insert("o:testmode", "yes".to_string());
        }

        // Tags
        for tag in &self.config.tags {
            form.insert("o:tag", tag.clone());
        }

        // Build multipart form
        let mut multipart = reqwest::multipart::Form::new();

        for (key, value) in form {
            multipart = multipart.text(key, value);
        }

        // Add attachments
        for attachment in &mail.attachments {
            let part = reqwest::multipart::Part::bytes(attachment.content.clone())
                .file_name(attachment.filename.clone())
                .mime_str(&attachment.content_type)
                .map_err(|e| MailError::SendFailed(format!("Invalid mime type: {}", e)))?;

            multipart = multipart.part("attachment", part);
        }

        // Send request
        let url = format!(
            "{}/{}/messages",
            self.config.region.api_base(),
            self.config.domain
        );

        let response = self
            .client
            .post(&url)
            .basic_auth("api", Some(&self.config.api_key))
            .multipart(multipart)
            .send()
            .await
            .map_err(|e| MailError::SendFailed(e.to_string()))?;

        let status = response.status();
        let response_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Failed to read response".to_string());

        if !status.is_success() {
            return Err(MailError::SendFailed(format!(
                "Mailgun API error ({}): {}",
                status, response_text
            )));
        }

        let mailgun_response: MailgunResponse = serde_json::from_str(&response_text)
            .map_err(|e| MailError::SendFailed(format!("Failed to parse response: {}", e)))?;

        Ok(mailgun_response)
    }
}

#[async_trait]
impl Mailer for MailgunMailer {
    async fn send(&self, mail: Mail) -> MailResult<()> {
        let response = self.send_via_api(&mail).await?;

        tracing::info!(
            "Email sent via Mailgun - ID: {}, Message: {}",
            response.id,
            response.message
        );

        Ok(())
    }
}

/// Mailgun API response
#[derive(Debug, Deserialize)]
pub struct MailgunResponse {
    /// Mailgun message ID
    pub id: String,

    /// Response message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MailBuilder};

    #[test]
    fn test_mailgun_config() {
        let config = MailgunConfig {
            api_key: "test-key".to_string(),
            domain: "mg.example.com".to_string(),
            region: MailgunRegion::EU,
            tags: vec!["test".to_string()],
            track_clicks: true,
            track_opens: false,
            test_mode: true,
        };

        assert_eq!(config.api_key, "test-key");
        assert_eq!(config.domain, "mg.example.com");
        assert_eq!(config.region, MailgunRegion::EU);
        assert_eq!(config.tags, vec!["test"]);
        assert!(config.track_clicks);
        assert!(!config.track_opens);
        assert!(config.test_mode);
    }

    #[test]
    fn test_mailgun_region() {
        assert_eq!(MailgunRegion::US.api_base(), "https://api.mailgun.net/v3");
        assert_eq!(
            MailgunRegion::EU.api_base(),
            "https://api.eu.mailgun.net/v3"
        );
    }

    #[test]
    fn test_mailgun_mailer_creation() {
        let config = MailgunConfig {
            api_key: "test-key".to_string(),
            domain: "mg.example.com".to_string(),
            ..Default::default()
        };

        let mailer = MailgunMailer::new(config);
        assert_eq!(mailer.config.api_key, "test-key");
        assert_eq!(mailer.config.domain, "mg.example.com");
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
