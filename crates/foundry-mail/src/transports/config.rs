use serde::{Deserialize, Serialize};
use std::time::Duration;

/// SMTP configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub from_address: String,
    pub from_name: Option<String>,
    pub timeout: Option<Duration>,
    pub use_tls: bool,
    pub use_starttls: bool,
}

impl SmtpConfig {
    pub fn from_env() -> Self {
        Self {
            host: std::env::var("MAIL_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: std::env::var("MAIL_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(587),
            username: std::env::var("MAIL_USERNAME").ok(),
            password: std::env::var("MAIL_PASSWORD").ok(),
            from_address: std::env::var("MAIL_FROM_ADDRESS")
                .unwrap_or_else(|_| "noreply@localhost".to_string()),
            from_name: std::env::var("MAIL_FROM_NAME").ok(),
            timeout: Some(Duration::from_secs(30)),
            use_tls: std::env::var("MAIL_USE_TLS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(false),
            use_starttls: std::env::var("MAIL_USE_STARTTLS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true),
        }
    }

    pub fn default_sender(&self) -> String {
        match &self.from_name {
            Some(name) => format!("{} <{}>", name, self.from_address),
            None => self.from_address.clone(),
        }
    }
}

impl Default for SmtpConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smtp_config_default_sender() {
        let config = SmtpConfig {
            host: "localhost".to_string(),
            port: 587,
            username: None,
            password: None,
            from_address: "test@example.com".to_string(),
            from_name: Some("Test User".to_string()),
            timeout: None,
            use_tls: false,
            use_starttls: true,
        };

        assert_eq!(config.default_sender(), "Test User <test@example.com>");
    }
}
