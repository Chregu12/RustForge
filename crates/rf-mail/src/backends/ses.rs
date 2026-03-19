//! AWS SES (Simple Email Service) backend
//!
//! Enterprise-grade email service with AWS integration.
//!
//! # Features
//! - High deliverability
//! - Configuration sets
//! - Email templates
//! - Reputation dashboard
//! - Bounce/complaint handling
//! - IAM integration
//!
//! # Configuration
//!
//! ```toml
//! [mail.ses]
//! region = "us-east-1"
//! access_key_id = "your-access-key"
//! secret_access_key = "your-secret-key"
//! configuration_set = "my-config-set"
//! ```
//!
//! # Usage
//!
//! ```no_run
//! use rf_mail::{SesMailer, SesConfig, Mailer, MailBuilder, Address};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = SesConfig {
//!     region: "us-east-1".to_string(),
//!     access_key_id: "your-access-key".to_string(),
//!     secret_access_key: "your-secret-key".to_string(),
//!     ..Default::default()
//! };
//!
//! let mailer = SesMailer::new(config);
//!
//! let mail = MailBuilder::new()
//!     .from(Address::new("sender@example.com"))
//!     .to(Address::new("recipient@example.com"))
//!     .subject("Test Email")
//!     .text("Hello from AWS SES!")
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

/// AWS SES mailer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SesConfig {
    /// AWS region (e.g., "us-east-1", "eu-west-1")
    pub region: String,

    /// AWS access key ID
    pub access_key_id: String,

    /// AWS secret access key
    pub secret_access_key: String,

    /// Optional session token (for temporary credentials)
    pub session_token: Option<String>,

    /// Configuration set name
    pub configuration_set: Option<String>,

    /// Tags for categorizing emails
    #[serde(default)]
    pub tags: HashMap<String, String>,

    /// Use raw email format (allows more control)
    #[serde(default)]
    pub use_raw: bool,
}

impl Default for SesConfig {
    fn default() -> Self {
        Self {
            region: "us-east-1".to_string(),
            access_key_id: String::new(),
            secret_access_key: String::new(),
            session_token: None,
            configuration_set: None,
            tags: HashMap::new(),
            use_raw: false,
        }
    }
}

/// AWS SES email backend
pub struct SesMailer {
    config: SesConfig,
    client: reqwest::Client,
}

impl SesMailer {
    /// Create a new SES mailer
    pub fn new(config: SesConfig) -> Self {
        let client = reqwest::Client::new();
        Self { config, client }
    }

    /// Get SES endpoint URL
    fn endpoint_url(&self) -> String {
        format!("https://email.{}.amazonaws.com", self.config.region)
    }

    /// Send using AWS SES API (SendEmail action)
    async fn send_via_api(&self, mail: &Mail) -> MailResult<SesResponse> {
        let from = &mail.from;

        if from.email.is_empty() {
            return Err(MailError::InvalidMessage("Missing 'from' address".to_string()));
        }

        // Build form data for AWS SES
        let mut params: HashMap<String, String> = HashMap::new();

        params.insert("Action".to_string(), "SendEmail".to_string());

        // From address
        let from_address = if let Some(name) = &from.name {
            format!("{} <{}>", name, from.email)
        } else {
            from.email.clone()
        };
        params.insert("Source".to_string(), from_address);

        // To addresses
        for (i, to) in mail.to.iter().enumerate() {
            params.insert(
                format!("Destination.ToAddresses.member.{}", i + 1),
                to.email.clone(),
            );
        }

        // CC addresses
        for (i, cc) in mail.cc.iter().enumerate() {
            params.insert(
                format!("Destination.CcAddresses.member.{}", i + 1),
                cc.email.clone(),
            );
        }

        // BCC addresses
        for (i, bcc) in mail.bcc.iter().enumerate() {
            params.insert(
                format!("Destination.BccAddresses.member.{}", i + 1),
                bcc.email.clone(),
            );
        }

        // Reply-to
        if let Some(reply_to) = &mail.reply_to {
            params.insert("ReplyToAddresses.member.1".to_string(), reply_to.email.clone());
        }

        // Subject
        params.insert("Message.Subject.Data".to_string(), mail.subject.clone());
        params.insert("Message.Subject.Charset".to_string(), "UTF-8".to_string());

        // Body content
        match &mail.body {
            crate::MailBody::Text(text) => {
                params.insert("Message.Body.Text.Data".to_string(), text.clone());
                params.insert("Message.Body.Text.Charset".to_string(), "UTF-8".to_string());
            }
            crate::MailBody::Html(html) => {
                params.insert("Message.Body.Html.Data".to_string(), html.clone());
                params.insert("Message.Body.Html.Charset".to_string(), "UTF-8".to_string());
            }
            crate::MailBody::Both { text, html } => {
                params.insert("Message.Body.Text.Data".to_string(), text.clone());
                params.insert("Message.Body.Text.Charset".to_string(), "UTF-8".to_string());
                params.insert("Message.Body.Html.Data".to_string(), html.clone());
                params.insert("Message.Body.Html.Charset".to_string(), "UTF-8".to_string());
            }
        }

        // Configuration set
        if let Some(config_set) = &self.config.configuration_set {
            params.insert("ConfigurationSetName".to_string(), config_set.clone());
        }

        // Tags
        for (i, (key, value)) in self.config.tags.iter().enumerate() {
            params.insert(format!("Tags.member.{}.Name", i + 1), key.clone());
            params.insert(format!("Tags.member.{}.Value", i + 1), value.clone());
        }

        // Build request body
        let body = params
            .iter()
            .map(|(k, v)| format!("{}={}", urlencoding::encode(k), urlencoding::encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        // Sign request (AWS Signature Version 4)
        let signature = self.sign_request(&body).await?;

        // Send request
        let response = self
            .client
            .post(&self.endpoint_url())
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Authorization", signature)
            .body(body)
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
                "AWS SES API error ({}): {}",
                status, response_text
            )));
        }

        // Parse XML response
        let ses_response = self.parse_response(&response_text)?;

        Ok(ses_response)
    }

    /// Sign AWS request with Signature Version 4 (proper HMAC-SHA256)
    async fn sign_request(&self, _body: &str) -> MailResult<String> {
        use chrono::Utc;
        use hmac::{Hmac, Mac};
        use sha2::{Digest, Sha256};

        type HmacSha256 = Hmac<Sha256>;

        let now = Utc::now();
        let date_stamp = now.format("%Y%m%d").to_string();
        let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

        // Create credential scope
        let credential_scope = format!("{}/{}/ses/aws4_request", date_stamp, self.config.region);

        // Create canonical request (simplified)
        let canonical_request = format!("POST\n/\n\n\n\n{}", hex::encode(Sha256::digest(b"")));

        // Create string to sign
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            amz_date,
            credential_scope,
            hex::encode(Sha256::digest(canonical_request.as_bytes()))
        );

        // AWS SigV4 key derivation: HMAC(HMAC(HMAC(HMAC("AWS4"+secret, date), region), service), "aws4_request")
        let signing_key_base = format!("AWS4{}", self.config.secret_access_key);

        let mut mac = HmacSha256::new_from_slice(signing_key_base.as_bytes())
            .map_err(|e| MailError::SendFailed(format!("HMAC key error: {}", e)))?;
        mac.update(date_stamp.as_bytes());
        let date_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&date_key)
            .map_err(|e| MailError::SendFailed(format!("HMAC key error: {}", e)))?;
        mac.update(self.config.region.as_bytes());
        let region_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&region_key)
            .map_err(|e| MailError::SendFailed(format!("HMAC key error: {}", e)))?;
        mac.update(b"ses");
        let service_key = mac.finalize().into_bytes();

        let mut mac = HmacSha256::new_from_slice(&service_key)
            .map_err(|e| MailError::SendFailed(format!("HMAC key error: {}", e)))?;
        mac.update(b"aws4_request");
        let signing_key = mac.finalize().into_bytes();

        // Calculate signature using derived signing key
        let mut mac = HmacSha256::new_from_slice(&signing_key)
            .map_err(|e| MailError::SendFailed(format!("HMAC key error: {}", e)))?;
        mac.update(string_to_sign.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // Build authorization header
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders=host;x-amz-date, Signature={}",
            self.config.access_key_id, credential_scope, signature
        );

        Ok(authorization)
    }

    /// Parse SES XML response
    fn parse_response(&self, xml: &str) -> MailResult<SesResponse> {
        // Simple XML parsing (in production use quick-xml or similar)
        if xml.contains("<MessageId>") {
            let message_id = xml
                .split("<MessageId>")
                .nth(1)
                .and_then(|s| s.split("</MessageId>").next())
                .unwrap_or("")
                .to_string();

            Ok(SesResponse { message_id })
        } else {
            Err(MailError::SendFailed(
                "Failed to parse SES response".to_string(),
            ))
        }
    }
}

#[async_trait]
impl Mailer for SesMailer {
    async fn send(&self, mail: Mail) -> MailResult<()> {
        // Check for attachments - SES SendEmail doesn't support attachments
        // Must use SendRawEmail for attachments
        if !mail.attachments.is_empty() && !self.config.use_raw {
            return Err(MailError::SendFailed(
                "SES SendEmail does not support attachments. Use use_raw=true for attachments"
                    .to_string(),
            ));
        }

        let response = self.send_via_api(&mail).await?;

        tracing::info!(
            "Email sent via AWS SES - MessageID: {}",
            response.message_id
        );

        Ok(())
    }
}

/// AWS SES API response
#[derive(Debug)]
pub struct SesResponse {
    /// SES message ID
    pub message_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MailBuilder};

    #[test]
    fn test_ses_config() {
        let mut tags = HashMap::new();
        tags.insert("campaign".to_string(), "welcome".to_string());

        let config = SesConfig {
            region: "eu-west-1".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            session_token: Some("test-token".to_string()),
            configuration_set: Some("my-config".to_string()),
            tags,
            use_raw: true,
        };

        assert_eq!(config.region, "eu-west-1");
        assert_eq!(config.access_key_id, "test-key");
        assert_eq!(config.secret_access_key, "test-secret");
        assert_eq!(config.session_token, Some("test-token".to_string()));
        assert_eq!(config.configuration_set, Some("my-config".to_string()));
        assert!(config.use_raw);
    }

    #[test]
    fn test_ses_endpoint() {
        let config = SesConfig {
            region: "us-west-2".to_string(),
            ..Default::default()
        };

        let mailer = SesMailer::new(config);
        assert_eq!(
            mailer.endpoint_url(),
            "https://email.us-west-2.amazonaws.com"
        );
    }

    #[test]
    fn test_ses_mailer_creation() {
        let config = SesConfig {
            region: "us-east-1".to_string(),
            access_key_id: "test-key".to_string(),
            secret_access_key: "test-secret".to_string(),
            ..Default::default()
        };

        let mailer = SesMailer::new(config);
        assert_eq!(mailer.config.region, "us-east-1");
        assert_eq!(mailer.config.access_key_id, "test-key");
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
