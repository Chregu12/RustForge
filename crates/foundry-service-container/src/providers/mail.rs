use async_trait::async_trait;
use crate::container::Container;
use crate::error::Result;
use crate::provider::ServiceProvider;

/// Mail service provider for email services
pub struct MailServiceProvider;

impl MailServiceProvider {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl ServiceProvider for MailServiceProvider {
    async fn register(&self, container: &Container) -> Result<()> {
        // Register mail driver
        container
            .singleton("mail.driver", || {
                Ok(std::env::var("MAIL_DRIVER").unwrap_or_else(|_| "smtp".to_string()))
            })
            .await?;

        // Register SMTP settings
        container
            .singleton("mail.smtp.host", || {
                Ok(std::env::var("MAIL_HOST").unwrap_or_else(|_| "localhost".to_string()))
            })
            .await?;

        container
            .singleton("mail.smtp.port", || {
                Ok(std::env::var("MAIL_PORT")
                    .unwrap_or_else(|_| "587".to_string())
                    .parse::<u16>()
                    .unwrap_or(587))
            })
            .await?;

        container
            .singleton("mail.smtp.username", || {
                Ok(std::env::var("MAIL_USERNAME").unwrap_or_default())
            })
            .await?;

        container
            .singleton("mail.smtp.password", || {
                Ok(std::env::var("MAIL_PASSWORD").unwrap_or_default())
            })
            .await?;

        container
            .singleton("mail.smtp.encryption", || {
                Ok(std::env::var("MAIL_ENCRYPTION").unwrap_or_else(|_| "tls".to_string()))
            })
            .await?;

        // Register mail from settings
        container
            .singleton("mail.from.address", || {
                Ok(std::env::var("MAIL_FROM_ADDRESS")
                    .unwrap_or_else(|_| "noreply@foundry.dev".to_string()))
            })
            .await?;

        container
            .singleton("mail.from.name", || {
                Ok(std::env::var("MAIL_FROM_NAME")
                    .unwrap_or_else(|_| {
                        std::env::var("APP_NAME")
                            .unwrap_or_else(|_| "Foundry".to_string())
                    }))
            })
            .await?;

        // Register Mailgun settings
        container
            .singleton("mail.mailgun.domain", || {
                Ok(std::env::var("MAILGUN_DOMAIN").ok())
            })
            .await?;

        container
            .singleton("mail.mailgun.secret", || {
                Ok(std::env::var("MAILGUN_SECRET").ok())
            })
            .await?;

        container
            .singleton("mail.mailgun.endpoint", || {
                Ok(std::env::var("MAILGUN_ENDPOINT")
                    .unwrap_or_else(|_| "https://api.mailgun.net".to_string()))
            })
            .await?;

        // Register SES settings
        container
            .singleton("mail.ses.key", || {
                Ok(std::env::var("AWS_ACCESS_KEY_ID").ok())
            })
            .await?;

        container
            .singleton("mail.ses.secret", || {
                Ok(std::env::var("AWS_SECRET_ACCESS_KEY").ok())
            })
            .await?;

        container
            .singleton("mail.ses.region", || {
                Ok(std::env::var("AWS_DEFAULT_REGION")
                    .unwrap_or_else(|_| "us-east-1".to_string()))
            })
            .await?;

        Ok(())
    }

    async fn boot(&self, _container: &Container) -> Result<()> {
        // Could initialize mail connections here
        Ok(())
    }

    fn name(&self) -> &str {
        "MailServiceProvider"
    }

    fn defer(&self) -> Vec<String> {
        // Mail services can be lazy loaded
        vec![
            "mail.driver".to_string(),
            "mail.smtp.host".to_string(),
            "mail.smtp.port".to_string(),
            "mail.smtp.username".to_string(),
            "mail.smtp.password".to_string(),
            "mail.smtp.encryption".to_string(),
            "mail.from.address".to_string(),
            "mail.from.name".to_string(),
            "mail.mailgun.domain".to_string(),
            "mail.mailgun.secret".to_string(),
            "mail.mailgun.endpoint".to_string(),
            "mail.ses.key".to_string(),
            "mail.ses.secret".to_string(),
            "mail.ses.region".to_string(),
        ]
    }
}

impl Default for MailServiceProvider {
    fn default() -> Self {
        Self::new()
    }
}
