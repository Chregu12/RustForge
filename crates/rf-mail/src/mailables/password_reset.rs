//! Password reset email mailable

use crate::{Address, MailError, Mailable, Message, MessageBuilder};
use async_trait::async_trait;

/// Password reset email
///
/// # Example
///
/// ```
/// use rf_mail::{PasswordResetEmail, Address, Mailable, MemoryMailer, Mailer};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mailer = MemoryMailer::new();
///
/// let reset_email = PasswordResetEmail {
///     to: Address::new("user@example.com"),
///     reset_token: "abc123".into(),
///     reset_url: "https://example.com/reset".into(),
///     expires_minutes: 60,
/// };
///
/// reset_email.send(&mailer).await?;
/// # Ok(())
/// # }
/// ```
pub struct PasswordResetEmail {
    /// Recipient address
    pub to: Address,

    /// Password reset token
    pub reset_token: String,

    /// Reset URL (base URL)
    pub reset_url: String,

    /// Expiration time in minutes
    pub expires_minutes: u32,
}

#[async_trait]
impl Mailable for PasswordResetEmail {
    async fn build(&self) -> Result<Message, MailError> {
        let full_url = format!("{}?token={}", self.reset_url, self.reset_token);

        let html = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #4a5568; }}
        .button {{
            display: inline-block;
            padding: 12px 24px;
            margin: 20px 0;
            background-color: #4299e1;
            color: #ffffff;
            text-decoration: none;
            border-radius: 4px;
        }}
        .warning {{
            margin-top: 20px;
            padding: 15px;
            background-color: #fff5f5;
            border-left: 4px solid #fc8181;
            color: #742a2a;
        }}
        .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #e2e8f0; font-size: 12px; color: #718096; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Password Reset Request</h1>
        <p>You requested to reset your password. Click the button below to create a new password:</p>
        <a href="{}" class="button">Reset Password</a>
        <p>Or copy and paste this link into your browser:</p>
        <p>{}</p>
        <div class="warning">
            <strong>⚠️ Security Notice</strong><br>
            This link expires in {} minutes. If you didn't request this reset, please ignore this email.
        </div>
        <div class="footer">
            <p>If you have any questions, please contact our support team.</p>
        </div>
    </div>
</body>
</html>
            "#,
            full_url, full_url, self.expires_minutes
        );

        let text = format!(
            r#"Password Reset Request

You requested to reset your password. Visit the link below to create a new password:

{}

⚠️ SECURITY NOTICE
This link expires in {} minutes. If you didn't request this reset, please ignore this email.

---
If you have any questions, please contact our support team.
            "#,
            full_url, self.expires_minutes
        );

        Ok(MessageBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(self.to.clone())
            .subject("Reset Your Password")
            .html(html)
            .text(text)
            .build()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_password_reset_email() {
        let email = PasswordResetEmail {
            to: Address::new("user@example.com"),
            reset_token: "test-token-123".into(),
            reset_url: "https://example.com/reset".into(),
            expires_minutes: 60,
        };

        let message = email.build().await.unwrap();

        assert_eq!(message.subject, "Reset Your Password");
        assert_eq!(message.to[0].email, "user@example.com");

        let html = message.html.unwrap();
        assert!(html.contains("test-token-123"));
        assert!(html.contains("60 minutes"));

        let text = message.text.unwrap();
        assert!(text.contains("test-token-123"));
    }
}
