//! Email Sending for Authentication
//!
//! Handles sending authentication-related emails

use crate::AuthConfig;
use lettre::{
    message::header::ContentType, transport::smtp::authentication::Credentials, Message,
    SmtpTransport, Transport,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum EmailError {
    #[error("Failed to build email: {0}")]
    BuildError(#[from] lettre::error::Error),

    #[error("Failed to send email: {0}")]
    SendError(#[from] lettre::transport::smtp::Error),
}

pub type EmailResult<T> = Result<T, EmailError>;

/// Email Service
pub struct EmailService {
    config: AuthConfig,
    smtp_host: String,
    smtp_username: String,
    smtp_password: String,
}

impl EmailService {
    pub fn new(
        config: AuthConfig,
        smtp_host: String,
        smtp_username: String,
        smtp_password: String,
    ) -> Self {
        Self {
            config,
            smtp_host,
            smtp_username,
            smtp_password,
        }
    }

    /// Send password reset email
    pub fn send_password_reset(&self, to: &str, token: &str) -> EmailResult<()> {
        let reset_url = format!("{}/password/reset?token={}&email={}",
            self.config.app_url, token, urlencoding::encode(to));

        let body = format!(
            r#"Hello,

You are receiving this email because we received a password reset request for your account.

Click here to reset your password: {}

This password reset link will expire in {} hour(s).

If you did not request a password reset, no further action is required.

Regards,
{}"#,
            reset_url,
            self.config.password_reset_lifetime / 3600,
            self.config.app_name
        );

        self.send_email(to, "Reset Password", &body)
    }

    /// Send email verification
    pub fn send_email_verification(&self, to: &str, token: &str) -> EmailResult<()> {
        let verification_url = format!("{}/email/verify/{}", self.config.app_url, token);

        let body = format!(
            r#"Hello,

Please click the link below to verify your email address:

{}

This link will expire in {} hour(s).

If you did not create an account, no further action is required.

Regards,
{}"#,
            verification_url,
            self.config.email_verification_lifetime / 3600,
            self.config.app_name
        );

        self.send_email(to, "Verify Email Address", &body)
    }

    /// Send welcome email
    pub fn send_welcome(&self, to: &str, name: &str) -> EmailResult<()> {
        let body = format!(
            r#"Hello {},

Welcome to {}!

Your account has been created successfully.

Regards,
{}"#,
            name, self.config.app_name, self.config.app_name
        );

        self.send_email(to, &format!("Welcome to {}", self.config.app_name), &body)
    }

    fn send_email(&self, to: &str, subject: &str, body: &str) -> EmailResult<()> {
        let email = Message::builder()
            .from(self.config.from_email.parse().unwrap())
            .to(to.parse().unwrap())
            .subject(subject)
            .header(ContentType::TEXT_PLAIN)
            .body(body.to_string())?;

        let creds = Credentials::new(
            self.smtp_username.clone(),
            self.smtp_password.clone(),
        );

        let mailer = SmtpTransport::relay(&self.smtp_host)?
            .credentials(creds)
            .build();

        mailer.send(&email)?;

        Ok(())
    }
}
