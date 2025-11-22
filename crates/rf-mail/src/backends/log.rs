//! Log transport - logs emails to file instead of sending

use crate::{MailError, Mailer, Message};
use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;

/// Log mailer that writes emails to a file
///
/// Useful for development and testing.
pub struct LogMailer {
    log_path: PathBuf,
}

impl LogMailer {
    /// Create a new log mailer
    pub fn new(log_path: impl Into<PathBuf>) -> Self {
        Self {
            log_path: log_path.into(),
        }
    }

    /// Format a message for logging
    fn format_message(message: &Message) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "================================================================================\n"
        ));
        output.push_str(&format!("Message ID: {}\n", message.id));
        output.push_str(&format!(
            "From: {} <{}>\n",
            message.from.name.as_deref().unwrap_or(""),
            message.from.email
        ));

        output.push_str("To: ");
        for (i, addr) in message.to.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            if let Some(name) = &addr.name {
                output.push_str(&format!("{} <{}>", name, addr.email));
            } else {
                output.push_str(&addr.email);
            }
        }
        output.push('\n');

        if !message.cc.is_empty() {
            output.push_str("CC: ");
            for (i, addr) in message.cc.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&addr.email);
            }
            output.push('\n');
        }

        if !message.bcc.is_empty() {
            output.push_str("BCC: ");
            for (i, addr) in message.bcc.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&addr.email);
            }
            output.push('\n');
        }

        if let Some(reply_to) = &message.reply_to {
            output.push_str(&format!("Reply-To: {}\n", reply_to.email));
        }

        output.push_str(&format!("Subject: {}\n", message.subject));
        output.push_str(&format!("Timestamp: {}\n", chrono::Utc::now().to_rfc3339()));

        if !message.attachments.is_empty() {
            output.push_str(&format!("Attachments: {}\n", message.attachments.len()));
            for attachment in &message.attachments {
                output.push_str(&format!(
                    "  - {} ({} bytes)\n",
                    attachment.filename,
                    attachment.size()
                ));
            }
        }

        output.push_str("\n--- TEXT BODY ---\n");
        if let Some(text) = &message.text {
            output.push_str(text);
        } else {
            output.push_str("(no text body)");
        }
        output.push_str("\n\n--- HTML BODY ---\n");
        if let Some(html) = &message.html {
            output.push_str(html);
        } else {
            output.push_str("(no html body)");
        }
        output.push_str("\n================================================================================\n\n");

        output
    }
}

#[async_trait]
impl Mailer for LogMailer {
    async fn send(&self, message: &Message) -> Result<(), MailError> {
        let formatted = Self::format_message(message);

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_path)
            .await?;

        file.write_all(formatted.as_bytes()).await?;
        file.flush().await?;

        tracing::info!(
            "Email logged to {:?}: {} -> {}",
            self.log_path,
            message.from.email,
            message
                .to
                .iter()
                .map(|a| a.email.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MessageBuilder};
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_log_mailer() {
        let temp_file = NamedTempFile::new().unwrap();
        let log_path = temp_file.path();

        let mailer = LogMailer::new(log_path);

        let message = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test Email")
            .text("Hello, World!")
            .build()
            .unwrap();

        mailer.send(&message).await.unwrap();

        // Read the log file
        let contents = tokio::fs::read_to_string(log_path).await.unwrap();

        assert!(contents.contains("Test Email"));
        assert!(contents.contains("sender@example.com"));
        assert!(contents.contains("recipient@example.com"));
        assert!(contents.contains("Hello, World!"));
    }
}
