//! Comprehensive tests for rf-notifications

#[cfg(test)]
mod notification_tests {
    use crate::{
        channels::NotificationChannel, messages::DatabaseNotification, messages::MailMessage,
        Channel, Notifiable, NotifiableExt, Notification, NotificationResult, Notifier,
        NotifierBuilder,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    // ─── Test entities ─────────────────────────────────────────────────────────

    struct User {
        id: i64,
        email: String,
        phone: String,
    }

    impl Notifiable for User {
        fn route_notification_for_mail(&self) -> Option<String> {
            Some(self.email.clone())
        }
        fn route_notification_for_database(&self) -> Option<i64> {
            Some(self.id)
        }
        fn route_notification_for_sms(&self) -> Option<String> {
            Some(self.phone.clone())
        }
    }

    // ─── Mock channel helper ───────────────────────────────────────────────────

    #[derive(Clone)]
    struct RecordingChannel {
        calls: Arc<Mutex<Vec<String>>>,
        label: String,
    }

    impl RecordingChannel {
        fn new(label: &str) -> Self {
            Self {
                calls: Arc::new(Mutex::new(vec![])),
                label: label.to_string(),
            }
        }
        fn call_count(&self) -> usize {
            self.calls.lock().unwrap().len()
        }
    }

    #[async_trait]
    impl NotificationChannel for RecordingChannel {
        async fn send(
            &self,
            _notification: &dyn Notification,
            _notifiable: &dyn Notifiable,
        ) -> NotificationResult<()> {
            self.calls.lock().unwrap().push(self.label.clone());
            Ok(())
        }
    }

    // ─── Notification definitions ──────────────────────────────────────────────

    struct SimpleMailNotification {
        subject: String,
        body: String,
    }

    #[async_trait]
    impl Notification for SimpleMailNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Mail]
        }
        async fn to_mail(&self) -> Option<MailMessage> {
            Some(
                MailMessage::new()
                    .subject(self.subject.clone())
                    .line(self.body.clone()),
            )
        }
    }

    struct DbNotification {
        title: String,
    }

    #[async_trait]
    impl Notification for DbNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Database]
        }
        async fn to_database(&self) -> Option<DatabaseNotification> {
            Some(DatabaseNotification::new().title(self.title.clone()))
        }
    }

    struct MultiChannelNotification;

    #[async_trait]
    impl Notification for MultiChannelNotification {
        fn via(&self) -> Vec<Channel> {
            vec![Channel::Mail, Channel::Database]
        }
        async fn to_mail(&self) -> Option<MailMessage> {
            Some(MailMessage::new().subject("Multi").line("Hello"))
        }
        async fn to_database(&self) -> Option<DatabaseNotification> {
            Some(DatabaseNotification::new().title("Multi"))
        }
    }

    // ─── Notifier tests ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn send_mail_notification_calls_mail_channel() {
        let channel = RecordingChannel::new("mail");
        let mut notifier = Notifier::new();
        notifier.register_channel(Channel::Mail, Arc::new(channel.clone()));

        let user = User {
            id: 1,
            email: "user@example.com".into(),
            phone: "+1234567890".into(),
        };
        let notif = SimpleMailNotification {
            subject: "Test".into(),
            body: "Hello".into(),
        };

        notifier.send(&notif, &user).await.unwrap();
        assert_eq!(channel.call_count(), 1);
    }

    #[tokio::test]
    async fn send_db_notification_calls_database_channel() {
        let channel = RecordingChannel::new("database");
        let mut notifier = Notifier::new();
        notifier.register_channel(Channel::Database, Arc::new(channel.clone()));

        let user = User {
            id: 2,
            email: "u@e.com".into(),
            phone: "+0".into(),
        };
        let notif = DbNotification { title: "New DB Notif".into() };
        notifier.send(&notif, &user).await.unwrap();
        assert_eq!(channel.call_count(), 1);
    }

    #[tokio::test]
    async fn send_multi_channel_calls_each_channel() {
        let mail_ch = RecordingChannel::new("mail");
        let db_ch = RecordingChannel::new("db");

        let mut notifier = Notifier::new();
        notifier.register_channel(Channel::Mail, Arc::new(mail_ch.clone()));
        notifier.register_channel(Channel::Database, Arc::new(db_ch.clone()));

        let user = User {
            id: 3,
            email: "x@y.com".into(),
            phone: "+0".into(),
        };
        notifier.send(&MultiChannelNotification, &user).await.unwrap();

        assert_eq!(mail_ch.call_count(), 1);
        assert_eq!(db_ch.call_count(), 1);
    }

    #[tokio::test]
    async fn missing_channel_returns_error() {
        let notifier = Notifier::new(); // no channels registered
        let user = User { id: 4, email: "e@m.com".into(), phone: "+0".into() };
        let notif = SimpleMailNotification {
            subject: "S".into(),
            body: "B".into(),
        };
        let result = notifier.send(&notif, &user).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn notifiable_ext_notify_method() {
        let channel = RecordingChannel::new("mail");
        let mut notifier = Notifier::new();
        notifier.register_channel(Channel::Mail, Arc::new(channel.clone()));

        let user = User {
            id: 5,
            email: "ext@example.com".into(),
            phone: "+1".into(),
        };
        // Uses the NotifiableExt trait method
        user.notify(
            SimpleMailNotification {
                subject: "Ext".into(),
                body: "Works".into(),
            },
            &notifier,
        )
        .await
        .unwrap();

        assert_eq!(channel.call_count(), 1);
    }

    // ─── Notifier builder ──────────────────────────────────────────────────────

    #[test]
    fn notifier_builder_registers_channels() {
        let n = NotifierBuilder::new()
            .channel(Channel::Mail, Arc::new(RecordingChannel::new("m")))
            .channel(Channel::Database, Arc::new(RecordingChannel::new("d")))
            .build();

        assert!(n.has_channel(&Channel::Mail));
        assert!(n.has_channel(&Channel::Database));
        assert!(!n.has_channel(&Channel::Sms));
    }

    #[test]
    fn notifier_has_channel_returns_false_by_default() {
        let n = Notifier::new();
        assert!(!n.has_channel(&Channel::Mail));
        assert!(!n.has_channel(&Channel::Slack));
    }

    #[test]
    fn notifier_registered_channels_list() {
        let mut n = Notifier::new();
        n.register_channel(Channel::Mail, Arc::new(RecordingChannel::new("m")));
        n.register_channel(Channel::Sms, Arc::new(RecordingChannel::new("s")));
        let channels = n.registered_channels();
        assert_eq!(channels.len(), 2);
        assert!(channels.contains(&Channel::Mail));
        assert!(channels.contains(&Channel::Sms));
    }

    // ─── MailMessage builder ───────────────────────────────────────────────────

    #[test]
    fn mail_message_builder_chain() {
        let msg = MailMessage::new()
            .subject("Invoice Paid")
            .greeting("Hello!")
            .line("Your invoice has been paid.")
            .action("View Invoice", "https://example.com/invoices/1");

        assert_eq!(msg.subject, "Invoice Paid");
        assert_eq!(msg.greeting, Some("Hello!".to_string()));
        assert_eq!(msg.lines.len(), 1);
        assert!(msg.action.is_some());
    }

    #[test]
    fn mail_message_to_text_includes_all_parts() {
        let msg = MailMessage::new()
            .greeting("Hi!")
            .line("Your order has shipped.")
            .action("Track Order", "https://example.com/track");

        let text = msg.to_text();
        assert!(text.contains("Hi!"));
        assert!(text.contains("Your order has shipped."));
        assert!(text.contains("Track Order"));
        assert!(text.contains("https://example.com/track"));
    }

    #[test]
    fn mail_message_to_html_includes_all_parts() {
        let msg = MailMessage::new()
            .greeting("Hello!")
            .line("Welcome aboard.");

        let html = msg.to_html();
        assert!(html.contains("<h1>Hello!</h1>"));
        assert!(html.contains("<p>Welcome aboard.</p>"));
    }

    // ─── DatabaseNotification builder ──────────────────────────────────────────

    #[test]
    fn database_notification_builder() {
        let notif = DatabaseNotification::new()
            .title("Order Shipped")
            .message("Your order #42 has shipped.")
            .data(serde_json::json!({ "order_id": 42 }));

        assert_eq!(notif.title, "Order Shipped");
        assert_eq!(notif.message, "Your order #42 has shipped.");
        assert_eq!(notif.data["order_id"], 42);
    }

    // ─── Custom channel ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn custom_channel_is_dispatched_to() {
        let channel = RecordingChannel::new("custom");
        let mut notifier = Notifier::new();
        notifier.register_channel(
            Channel::Custom("push".into()),
            Arc::new(channel.clone()),
        );

        struct PushNotification;
        #[async_trait]
        impl Notification for PushNotification {
            fn via(&self) -> Vec<Channel> {
                vec![Channel::Custom("push".into())]
            }
        }

        let user = User { id: 6, email: "c@c.com".into(), phone: "+0".into() };
        notifier.send(&PushNotification, &user).await.unwrap();
        assert_eq!(channel.call_count(), 1);
    }

    // ─── should_queue default ──────────────────────────────────────────────────

    #[test]
    fn notification_should_queue_defaults_to_false() {
        let notif = SimpleMailNotification {
            subject: "q".into(),
            body: "b".into(),
        };
        assert!(!notif.should_queue());
    }

    // ─── Notifiable routing defaults ──────────────────────────────────────────

    #[test]
    fn notifiable_routing_returns_configured_values() {
        let user = User {
            id: 7,
            email: "route@test.com".into(),
            phone: "+44123456".into(),
        };
        assert_eq!(
            user.route_notification_for_mail(),
            Some("route@test.com".to_string())
        );
        assert_eq!(user.route_notification_for_database(), Some(7));
        assert_eq!(
            user.route_notification_for_sms(),
            Some("+44123456".to_string())
        );
    }
}
