//! Postmark email backend
//!
//! High-deliverability email service with excellent reputation management.
//!
//! # Features
//! - Simple HTTP API
//! - Templates support
//! - Message streams
//! - Bounce tracking
//! - DKIM/SPF automatic setup
//!
//! # Configuration
//!
//! ```toml
//! [mail.postmark]
//! server_token = "your-server-token"
//! message_stream = "outbound"  # or "broadcast", "transactional"
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use rf_mail::{PostmarkMailer, PostmarkConfig, Mailer, MailBuilder, Address};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = PostmarkConfig {
//!     server_token: "your-server-token".to_string(),
//!     message_stream: Some("outbound".to_string()),
//! };
//!
//! let mailer = PostmarkMailer::new(config);
//!
//! let mail = MailBuilder::new()
//!     .from(Address::new("sender@example.com"))
//!     .to(Address::new("recipient@example.com"))
//!     .subject("Test Email")
//!     .text("Hello from Postmark!")
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
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;

/// Postmark mailer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmarkConfig {
    /// Postmark server API token
    pub server_token: String,

    /// Message stream (outbound, broadcast, transactional)
    pub message_stream: Option<String>,
}

impl Default for PostmarkConfig {
    fn default() -> Self {
        Self {
            server_token: String::new(),
            message_stream: Some("outbound".to_string()),
        }
    }
}

/// Postmark email backend
pub struct PostmarkMailer {
    config: PostmarkConfig,
    client: reqwest::Client,
}

impl PostmarkMailer {
    /// Create a new Postmark mailer
    pub fn new(config: PostmarkConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    /// Send using Postmark API
    async fn send_via_api(&self, mail: &Mail) -> MailResult<PostmarkResponse> {
        let from = &mail.from;

        // Build recipients
        let to_addresses: Vec<String> = mail
            .to
            .iter()
            .map(|addr| {
                if let Some(name) = &addr.name {
                    format!("{} <{}>", name, addr.email)
                } else {
                    addr.email.clone()
                }
            })
            .collect();

        let cc_addresses: Option<Vec<String>> = if !mail.cc.is_empty() {
            Some(
                mail.cc
                    .iter()
                    .map(|addr| {
                        if let Some(name) = &addr.name {
                            format!("{} <{}>", name, addr.email)
                        } else {
                            addr.email.clone()
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        let bcc_addresses: Option<Vec<String>> = if !mail.bcc.is_empty() {
            Some(
                mail.bcc
                    .iter()
                    .map(|addr| {
                        if let Some(name) = &addr.name {
                            format!("{} <{}>", name, addr.email)
                        } else {
                            addr.email.clone()
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };

        // Build from address
        let from_address = if let Some(name) = &from.name {
            format!("{} <{}>", name, from.email)
        } else {
            from.email.clone()
        };

        // Build reply-to
        let reply_to = mail.reply_to.as_ref().map(|addr| {
            if let Some(name) = &addr.name {
                format!("{} <{}>", name, addr.email)
            } else {
                addr.email.clone()
            }
        });

        // Build attachments
        let attachments: Option<Vec<serde_json::Value>> = if !mail.attachments.is_empty() {
            Some(
                mail.attachments
                    .iter()
                    .map(|att| {
                        json!({
                            "Name": att.filename,
                            "Content": base64::engine::general_purpose::STANDARD.encode(&att.data),
                            "ContentType": att.content_type,
                        })
                    })
                    .collect(),
            )
        } else {
            None
        };

        // Build request body
        let mut body = json!({
            "From": from_address,
            "To": to_addresses.join(","),
            "Subject": mail.subject,
        });

        if let Some(cc) = cc_addresses {
            body["Cc"] = json!(cc.join(","));
        }

        if let Some(bcc) = bcc_addresses {
            body["Bcc"] = json!(bcc.join(","));
        }

        if let Some(reply) = reply_to {
            body["ReplyTo"] = json!(reply);
        }

        // Add message stream
        if let Some(stream) = &self.config.message_stream {
            body["MessageStream"] = json!(stream);
        }

        // Add body content
        match &mail.body {
            crate::MailBody::Text(text) => {
                body["TextBody"] = json!(text);
            }
            crate::MailBody::Html(html) => {
                body["HtmlBody"] = json!(html);
            }
            crate::MailBody::Both { text, html } => {
                body["TextBody"] = json!(text);
                body["HtmlBody"] = json!(html);
            }
        }

        // Add attachments
        if let Some(atts) = attachments {
            body["Attachments"] = json!(atts);
        }

        // Send request
        let response = self
            .client
            .post("https://api.postmarkapp.com/email")
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("X-Postmark-Server-Token", &self.config.server_token)
            .json(&body)
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
                "Postmark API error ({}): {}",
                status, response_text
            )));
        }

        let postmark_response: PostmarkResponse = serde_json::from_str(&response_text)
            .map_err(|e| MailError::SendFailed(format!("Failed to parse response: {}", e)))?;

        Ok(postmark_response)
    }
}

#[async_trait]
impl Mailer for PostmarkMailer {
    async fn send(&self, mail: Mail) -> MailResult<()> {
        let response = self.send_via_api(&mail).await?;

        tracing::info!(
            "Email sent via Postmark - MessageID: {}, SubmittedAt: {}",
            response.message_id,
            response.submitted_at
        );

        Ok(())
    }
}

/// Postmark API response
#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[allow(dead_code)] // deserialize-only: mirrors the Postmark payload; not all fields are read
pub struct PostmarkResponse {
    /// Postmark message ID
    pub message_id: String,

    /// Submission timestamp
    pub submitted_at: String,

    /// Recipient email
    pub to: String,

    /// Error code (0 = success)
    pub error_code: i32,

    /// Error message
    #[serde(default)]
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MailBuilder};

    #[test]
    fn test_postmark_config() {
        let config = PostmarkConfig {
            server_token: "test-token".to_string(),
            message_stream: Some("outbound".to_string()),
        };

        assert_eq!(config.server_token, "test-token");
        assert_eq!(config.message_stream, Some("outbound".to_string()));
    }

    #[test]
    fn test_postmark_mailer_creation() {
        let config = PostmarkConfig {
            server_token: "test-token".to_string(),
            message_stream: Some("outbound".to_string()),
        };

        let mailer = PostmarkMailer::new(config);
        assert_eq!(mailer.config.server_token, "test-token");
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
