//! Example notification implementations

use crate::messages::{
    DatabaseNotification, MailMessage, SlackAttachment, SlackMessage, SmsMessage,
};
use crate::{Channel, Notification};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// Invoice paid notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoicePaid {
    pub invoice_id: u64,
    pub customer_name: String,
    pub amount: f64,
    pub currency: String,
}

impl InvoicePaid {
    pub fn new(invoice_id: u64, customer_name: impl Into<String>, amount: f64) -> Self {
        Self {
            invoice_id,
            customer_name: customer_name.into(),
            amount,
            currency: "USD".to_string(),
        }
    }

    pub fn currency(mut self, currency: impl Into<String>) -> Self {
        self.currency = currency.into();
        self
    }
}

#[async_trait]
impl Notification for InvoicePaid {
    fn via(&self) -> Vec<Channel> {
        vec![Channel::Mail, Channel::Database]
    }

    async fn to_mail(&self) -> Option<MailMessage> {
        Some(
            MailMessage::new()
                .subject("Invoice Paid")
                .greeting(format!("Hello {}!", self.customer_name))
                .line(format!("Your invoice #{} has been paid.", self.invoice_id))
                .line(format!("Amount: {}{:.2}", self.currency, self.amount))
                .line("Thank you for your business!")
                .action("View Invoice", format!("/invoices/{}", self.invoice_id)),
        )
    }

    async fn to_database(&self) -> Option<DatabaseNotification> {
        Some(DatabaseNotification {
            title: "Invoice Paid".to_string(),
            message: format!("Invoice #{} has been paid", self.invoice_id),
            data: serde_json::json!({
                "invoice_id": self.invoice_id,
                "amount": self.amount,
                "currency": self.currency,
            }),
        })
    }
}

/// Order shipped notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderShipped {
    pub order_id: u64,
    pub tracking_number: String,
    pub carrier: String,
    pub customer_name: String,
}

impl OrderShipped {
    pub fn new(
        order_id: u64,
        tracking_number: impl Into<String>,
        carrier: impl Into<String>,
        customer_name: impl Into<String>,
    ) -> Self {
        Self {
            order_id,
            tracking_number: tracking_number.into(),
            carrier: carrier.into(),
            customer_name: customer_name.into(),
        }
    }
}

#[async_trait]
impl Notification for OrderShipped {
    fn via(&self) -> Vec<Channel> {
        vec![Channel::Mail, Channel::Database, Channel::Sms]
    }

    async fn to_mail(&self) -> Option<MailMessage> {
        Some(
            MailMessage::new()
                .subject("Your Order Has Shipped!")
                .greeting(format!("Hello {}!", self.customer_name))
                .line(format!("Your order #{} has been shipped.", self.order_id))
                .line(format!("Carrier: {}", self.carrier))
                .line(format!("Tracking Number: {}", self.tracking_number))
                .action("Track Package", format!("/orders/{}/track", self.order_id)),
        )
    }

    async fn to_database(&self) -> Option<DatabaseNotification> {
        Some(DatabaseNotification {
            title: "Order Shipped".to_string(),
            message: format!("Order #{} has been shipped", self.order_id),
            data: serde_json::json!({
                "order_id": self.order_id,
                "tracking_number": self.tracking_number,
                "carrier": self.carrier,
            }),
        })
    }

    async fn to_sms(&self) -> Option<SmsMessage> {
        Some(SmsMessage::new(format!(
            "Your order #{} has shipped via {}. Track: {}",
            self.order_id, self.carrier, self.tracking_number
        )))
    }
}

/// Password reset notification
#[derive(Debug, Clone)]
pub struct PasswordReset {
    pub user_name: String,
    pub reset_token: String,
    pub expires_in_minutes: u32,
}

impl PasswordReset {
    pub fn new(user_name: impl Into<String>, reset_token: impl Into<String>) -> Self {
        Self {
            user_name: user_name.into(),
            reset_token: reset_token.into(),
            expires_in_minutes: 60,
        }
    }

    pub fn expires_in(mut self, minutes: u32) -> Self {
        self.expires_in_minutes = minutes;
        self
    }
}

#[async_trait]
impl Notification for PasswordReset {
    fn via(&self) -> Vec<Channel> {
        vec![Channel::Mail]
    }

    async fn to_mail(&self) -> Option<MailMessage> {
        Some(
            MailMessage::new()
                .subject("Password Reset Request")
                .greeting(format!("Hello {}!", self.user_name))
                .line("You are receiving this email because we received a password reset request for your account.")
                .action("Reset Password", format!("/password/reset/{}", self.reset_token))
                .line(format!("This password reset link will expire in {} minutes.", self.expires_in_minutes))
                .line("If you did not request a password reset, no further action is required."),
        )
    }
}

/// Welcome notification for new users
#[derive(Debug, Clone)]
pub struct WelcomeNotification {
    pub user_name: String,
    pub email: String,
}

impl WelcomeNotification {
    pub fn new(user_name: impl Into<String>, email: impl Into<String>) -> Self {
        Self {
            user_name: user_name.into(),
            email: email.into(),
        }
    }
}

#[async_trait]
impl Notification for WelcomeNotification {
    fn via(&self) -> Vec<Channel> {
        vec![Channel::Mail, Channel::Database]
    }

    async fn to_mail(&self) -> Option<MailMessage> {
        Some(
            MailMessage::new()
                .subject("Welcome to RustForge!")
                .greeting(format!("Welcome, {}!", self.user_name))
                .line("Thank you for creating an account with us.")
                .line("We're excited to have you on board!")
                .line("To get started, please verify your email address.")
                .action("Verify Email", "/verify-email"),
        )
    }

    async fn to_database(&self) -> Option<DatabaseNotification> {
        Some(DatabaseNotification {
            title: "Welcome!".to_string(),
            message: "Welcome to RustForge!".to_string(),
            data: serde_json::json!({
                "user_name": self.user_name,
                "email": self.email,
            }),
        })
    }
}

/// Server alert notification for Slack
#[derive(Debug, Clone)]
pub struct ServerAlert {
    pub severity: AlertSeverity,
    pub title: String,
    pub message: String,
    pub details: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

impl ServerAlert {
    pub fn new(
        severity: AlertSeverity,
        title: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity,
            title: title.into(),
            message: message.into(),
            details: Vec::new(),
        }
    }

    pub fn detail(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.details.push((key.into(), value.into()));
        self
    }
}

#[async_trait]
impl Notification for ServerAlert {
    fn via(&self) -> Vec<Channel> {
        vec![Channel::Slack]
    }

    async fn to_slack(&self) -> Option<SlackMessage> {
        let color = match self.severity {
            AlertSeverity::Info => "good",
            AlertSeverity::Warning => "warning",
            AlertSeverity::Critical => "danger",
        };

        let mut attachment = SlackAttachment::new(&self.message)
            .title(&self.title)
            .color(color);

        for (key, value) in &self.details {
            attachment = attachment.field(key, value, true);
        }

        Some(SlackMessage::new("Server Alert").attachment(attachment))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_invoice_paid() {
        let notification = InvoicePaid::new(123, "John Doe", 99.99).currency("EUR");

        let channels = notification.via();
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&Channel::Mail));
        assert!(channels.contains(&Channel::Database));

        let mail = notification.to_mail().await.unwrap();
        assert!(mail.subject.contains("Invoice Paid"));
        assert!(mail.lines.iter().any(|l| l.contains("EUR99.99")));

        let db = notification.to_database().await.unwrap();
        assert_eq!(db.title, "Invoice Paid");
    }

    #[tokio::test]
    async fn test_order_shipped() {
        let notification = OrderShipped::new(456, "1Z999AA10123456784", "UPS", "Jane Smith");

        let channels = notification.via();
        assert_eq!(channels.len(), 3);

        let mail = notification.to_mail().await.unwrap();
        assert!(mail.subject.contains("Shipped"));

        let sms = notification.to_sms().await.unwrap();
        assert!(sms.content.contains("456"));
        assert!(sms.content.contains("UPS"));
    }

    #[tokio::test]
    async fn test_password_reset() {
        let notification = PasswordReset::new("Alice", "reset-token-123").expires_in(30);

        let mail = notification.to_mail().await.unwrap();
        assert!(mail.subject.contains("Password Reset"));
        assert!(mail.lines.iter().any(|l| l.contains("30 minutes")));
    }

    #[tokio::test]
    async fn test_welcome_notification() {
        let notification = WelcomeNotification::new("Bob", "bob@example.com");

        let mail = notification.to_mail().await.unwrap();
        assert!(mail.subject.contains("Welcome"));
        assert!(mail.greeting.as_ref().unwrap().contains("Bob"));
    }

    #[tokio::test]
    async fn test_server_alert() {
        let notification = ServerAlert::new(
            AlertSeverity::Critical,
            "High CPU Usage",
            "CPU usage is above 90%",
        )
        .detail("Server", "prod-web-01")
        .detail("CPU", "95%");

        let slack = notification.to_slack().await.unwrap();
        assert_eq!(slack.text, "Server Alert");
        assert_eq!(slack.attachments.len(), 1);
        assert_eq!(slack.attachments[0].color, "danger");
        assert_eq!(slack.attachments[0].fields.len(), 2);
    }
}
