//! Welcome email mailable

use crate::{Address, MailError, Mailable, Message, MessageBuilder};
use async_trait::async_trait;

/// Welcome email for new users
///
/// # Example
///
/// ```
/// use rf_mail::{WelcomeEmail, Address, Mailable, MemoryMailer, Mailer};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mailer = MemoryMailer::new();
///
/// let welcome = WelcomeEmail {
///     to: Address::with_name("user@example.com", "John Doe"),
///     user_name: "John".into(),
///     app_name: "MyApp".into(),
/// };
///
/// welcome.send(&mailer).await?;
/// # Ok(())
/// # }
/// ```
pub struct WelcomeEmail {
    /// Recipient address
    pub to: Address,

    /// User's name
    pub user_name: String,

    /// Application name
    pub app_name: String,
}

#[async_trait]
impl Mailable for WelcomeEmail {
    async fn build(&self) -> Result<Message, MailError> {
        let html = format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <style>
        body {{ font-family: Arial, sans-serif; line-height: 1.6; color: #333; }}
        .container {{ max-width: 600px; margin: 0 auto; padding: 20px; }}
        h1 {{ color: #4a5568; }}
        .footer {{ margin-top: 30px; padding-top: 20px; border-top: 1px solid #e2e8f0; font-size: 12px; color: #718096; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Welcome, {}!</h1>
        <p>Thank you for joining {}. We're excited to have you on board!</p>
        <p>If you have any questions, feel free to reach out to our support team.</p>
        <div class="footer">
            <p>This email was sent by {}. If you did not sign up, please ignore this email.</p>
        </div>
    </div>
</body>
</html>
            "#,
            self.user_name, self.app_name, self.app_name
        );

        let text = format!(
            r#"Welcome, {}!

Thank you for joining {}. We're excited to have you on board!

If you have any questions, feel free to reach out to our support team.

---
This email was sent by {}. If you did not sign up, please ignore this email.
            "#,
            self.user_name, self.app_name, self.app_name
        );

        Ok(MessageBuilder::new()
            .from(Address::with_name(
                "noreply@example.com",
                &self.app_name,
            ))
            .to(self.to.clone())
            .subject(format!("Welcome to {}!", self.app_name))
            .html(html)
            .text(text)
            .build()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_welcome_email() {
        let email = WelcomeEmail {
            to: Address::with_name("user@example.com", "Test User"),
            user_name: "Test".into(),
            app_name: "TestApp".into(),
        };

        let message = email.build().await.unwrap();

        assert_eq!(message.subject, "Welcome to TestApp!");
        assert_eq!(message.to.len(), 1);
        assert_eq!(message.to[0].email, "user@example.com");
        assert!(message.html.is_some());
        assert!(message.text.is_some());
    }
}
