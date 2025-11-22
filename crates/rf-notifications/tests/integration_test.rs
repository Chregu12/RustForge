//! Comprehensive integration tests for notification system

use async_trait::async_trait;
use rf_notifications::*;
use std::sync::Arc;

#[cfg(feature = "mail")]
use rf_mail::MemoryMailer;

#[cfg(feature = "sms")]
use rf_notifications::channels::{MockSmsProvider, SmsProvider};

#[cfg(feature = "slack")]
use rf_notifications::channels::MockSlackChannel;

// Test user implementation
struct TestUser {
    id: i64,
    email: String,
    phone: String,
}

impl Notifiable for TestUser {
    fn route_notification_for_mail(&self) -> Option<String> {
        Some(self.email.clone())
    }

    fn route_notification_for_database(&self) -> Option<i64> {
        Some(self.id)
    }

    fn route_notification_for_sms(&self) -> Option<String> {
        Some(self.phone.clone())
    }

    fn route_notification_for_slack(&self) -> Option<String> {
        Some("https://hooks.slack.com/test".to_string())
    }
}

// Test notification that uses all channels
struct MultiChannelNotification {
    message: String,
}

#[async_trait]
impl Notification for MultiChannelNotification {
    fn via(&self) -> Vec<Channel> {
        vec![
            Channel::Mail,
            Channel::Database,
            Channel::Sms,
            Channel::Slack,
        ]
    }

    async fn to_mail(&self) -> Option<messages::MailMessage> {
        Some(
            messages::MailMessage::new()
                .subject("Test Notification")
                .greeting("Hello!")
                .line(&self.message)
                .action("Click Here", "https://example.com"),
        )
    }

    async fn to_database(&self) -> Option<messages::DatabaseNotification> {
        Some(messages::DatabaseNotification {
            title: "Test".to_string(),
            message: self.message.clone(),
            data: serde_json::json!({"test": true}),
        })
    }

    async fn to_sms(&self) -> Option<messages::SmsMessage> {
        Some(messages::SmsMessage::new(&self.message))
    }

    async fn to_slack(&self) -> Option<messages::SlackMessage> {
        Some(messages::SlackMessage::new(&self.message))
    }
}

#[cfg(feature = "mail")]
#[tokio::test]
async fn test_mail_channel_integration() {
    use rf_notifications::channels::MailChannel;

    let mailer = Arc::new(MemoryMailer::new());
    let mut notifier = Notifier::new();
    notifier.register_channel(
        Channel::Mail,
        Arc::new(MailChannel::new(mailer.clone(), "noreply@example.com")),
    );

    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Mail]
        }

        async fn to_mail(&self) -> Option<messages::MailMessage> {
            Some(
                messages::MailMessage::new()
                    .subject("Test Subject")
                    .greeting("Hello World!")
                    .line("This is a test line.")
                    .action("Click Me", "https://example.com"),
            )
        }
    }

    user.notify(TestNotification, &notifier).await.unwrap();

    let sent = mailer.sent_messages();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].subject, "Test Subject");

    // Verify HTML contains greeting and action button
    assert!(sent[0].html.as_ref().unwrap().contains("Hello World!"));
    assert!(sent[0].html.as_ref().unwrap().contains("Click Me"));
}

#[cfg(feature = "database")]
#[tokio::test]
async fn test_database_channel_integration() {
    use rf_notifications::channels::DatabaseChannel;
    use sea_orm::{Database, DbBackend, MockDatabase, MockExecResult};

    let db = MockDatabase::new(DbBackend::Postgres)
        .append_exec_results([MockExecResult {
            last_insert_id: 1,
            rows_affected: 1,
        }])
        .into_connection();

    let mut notifier = Notifier::new();
    notifier.register_channel(Channel::Database, Arc::new(DatabaseChannel::new(db)));

    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Database]
        }

        async fn to_database(&self) -> Option<messages::DatabaseNotification> {
            Some(messages::DatabaseNotification {
                title: "Test Title".to_string(),
                message: "Test Message".to_string(),
                data: serde_json::json!({"key": "value"}),
            })
        }
    }

    let result = user.notify(TestNotification, &notifier).await;
    assert!(result.is_ok());
}

#[cfg(feature = "sms")]
#[tokio::test]
async fn test_sms_channel_integration() {
    use rf_notifications::channels::SmsChannel;

    let provider = Arc::new(MockSmsProvider::new());
    let mut notifier = Notifier::new();
    notifier.register_channel(Channel::Sms, Arc::new(SmsChannel::new(provider.clone())));

    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Sms]
        }

        async fn to_sms(&self) -> Option<messages::SmsMessage> {
            Some(messages::SmsMessage::new("Test SMS message"))
        }
    }

    user.notify(TestNotification, &notifier).await.unwrap();

    let sent = provider.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].0, "+1234567890");
    assert_eq!(sent[0].1, "Test SMS message");
}

#[cfg(feature = "slack")]
#[tokio::test]
async fn test_slack_channel_integration() {
    let mock_channel = Arc::new(MockSlackChannel::new());
    let mut notifier = Notifier::new();
    notifier.register_channel(Channel::Slack, mock_channel.clone());

    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Slack]
        }

        async fn to_slack(&self) -> Option<messages::SlackMessage> {
            use messages::{SlackAttachment, SlackMessage};
            Some(
                SlackMessage::new("Test Slack message").attachment(
                    SlackAttachment::new("Attachment text")
                        .title("Title")
                        .color("good")
                        .field("Field 1", "Value 1", true),
                ),
            )
        }
    }

    user.notify(TestNotification, &notifier).await.unwrap();

    let sent = mock_channel.sent_messages().await;
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].text, "Test Slack message");
    assert_eq!(sent[0].attachments.len(), 1);
}

#[tokio::test]
async fn test_notifier_builder() {
    let notifier = NotifierBuilder::new().build();

    assert!(!notifier.has_channel(&Channel::Mail));
    assert!(!notifier.has_channel(&Channel::Database));
}

#[tokio::test]
async fn test_example_notifications() {
    use rf_notifications::examples::*;

    // Test InvoicePaid
    let invoice_paid = InvoicePaid::new(123, "John Doe", 99.99).currency("EUR");
    let channels = invoice_paid.via();
    assert_eq!(channels.len(), 2);

    let mail = invoice_paid.to_mail().await.unwrap();
    assert!(mail.subject.contains("Invoice Paid"));

    // Test OrderShipped
    let order_shipped = OrderShipped::new(456, "TRACK123", "UPS", "Jane Doe");
    let channels = order_shipped.via();
    assert_eq!(channels.len(), 3);

    let sms = order_shipped.to_sms().await.unwrap();
    assert!(sms.content.contains("456"));

    // Test PasswordReset
    let password_reset = PasswordReset::new("Alice", "token123").expires_in(30);
    let mail = password_reset.to_mail().await.unwrap();
    assert!(mail.subject.contains("Password Reset"));

    // Test WelcomeNotification
    let welcome = WelcomeNotification::new("Bob", "bob@example.com");
    let db = welcome.to_database().await.unwrap();
    assert_eq!(db.title, "Welcome!");

    // Test ServerAlert
    let alert = ServerAlert::new(AlertSeverity::Critical, "High CPU", "CPU > 90%")
        .detail("Server", "prod-01");
    let slack = alert.to_slack().await.unwrap();
    assert_eq!(slack.attachments[0].color, "danger");
}

#[tokio::test]
async fn test_mail_message_rendering() {
    let msg = messages::MailMessage::new()
        .subject("Test")
        .greeting("Hello!")
        .line("Line 1")
        .line("Line 2")
        .action("Click", "https://example.com");

    let text = msg.to_text();
    assert!(text.contains("Hello!"));
    assert!(text.contains("Line 1"));
    assert!(text.contains("Click"));

    let html = msg.to_html();
    assert!(html.contains("<h1>Hello!</h1>"));
    assert!(html.contains("<p>Line 1</p>"));
    assert!(html.contains("<a href="));
}

#[tokio::test]
async fn test_notifiable_trait() {
    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    assert_eq!(
        user.route_notification_for_mail(),
        Some("user@example.com".to_string())
    );
    assert_eq!(user.route_notification_for_database(), Some(1));
    assert_eq!(
        user.route_notification_for_sms(),
        Some("+1234567890".to_string())
    );
}

#[tokio::test]
async fn test_notification_error_handling() {
    let notifier = Notifier::new();

    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    struct TestNotification;

    #[async_trait]
    impl Notification for TestNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Mail]
        }

        async fn to_mail(&self) -> Option<messages::MailMessage> {
            Some(messages::MailMessage::new().subject("Test"))
        }
    }

    // Should fail because no mail channel is registered
    let result = user.notify(TestNotification, &notifier).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_channel_enum() {
    let mail = Channel::Mail;
    let db = Channel::Database;
    let sms = Channel::Sms;
    let slack = Channel::Slack;
    let custom = Channel::Custom("webhook".to_string());

    assert_eq!(mail, Channel::Mail);
    assert_ne!(mail, db);
    assert_eq!(custom, Channel::Custom("webhook".to_string()));
}

#[cfg(all(feature = "mail", feature = "database"))]
#[tokio::test]
async fn test_multi_channel_notification() {
    use rf_notifications::channels::{DatabaseChannel, MailChannel};
    use sea_orm::{DbBackend, MockDatabase, MockExecResult};

    let mailer = Arc::new(MemoryMailer::new());
    let db = MockDatabase::new(DbBackend::Postgres)
        .append_exec_results([MockExecResult {
            last_insert_id: 1,
            rows_affected: 1,
        }])
        .into_connection();

    let mut notifier = Notifier::new();
    notifier.register_channel(
        Channel::Mail,
        Arc::new(MailChannel::new(mailer.clone(), "noreply@example.com")),
    );
    notifier.register_channel(Channel::Database, Arc::new(DatabaseChannel::new(db)));

    let user = TestUser {
        id: 1,
        email: "user@example.com".to_string(),
        phone: "+1234567890".to_string(),
    };

    struct MultiNotification;

    #[async_trait]
    impl Notification for MultiNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Mail, Channel::Database]
        }

        async fn to_mail(&self) -> Option<messages::MailMessage> {
            Some(messages::MailMessage::new().subject("Multi Test"))
        }

        async fn to_database(&self) -> Option<messages::DatabaseNotification> {
            Some(messages::DatabaseNotification {
                title: "Multi Test".to_string(),
                message: "Test".to_string(),
                data: serde_json::json!({}),
            })
        }
    }

    let result = user.notify(MultiNotification, &notifier).await;
    assert!(result.is_ok());

    // Verify mail was sent
    let sent = mailer.sent_messages();
    assert_eq!(sent.len(), 1);
}
