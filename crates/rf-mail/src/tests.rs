//! Comprehensive tests for rf-mail

#[cfg(test)]
mod mail_tests {
    use crate::{
        Address, MailBody, MailBuilder, Mailable, MemoryMailer, MessageBuilder, TemplateEngine,
    };
    use crate::testing::MailFake;
    use crate::Mailer;

    // ───────── MemoryMailer ─────────

    #[tokio::test]
    async fn memory_mailer_stores_sent_mail() {
        let mailer = MemoryMailer::new();
        assert_eq!(mailer.sent_count(), 0);

        let msg = MessageBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("alice@example.com"))
            .subject("Hello Alice")
            .text("Hi there")
            .build()
            .unwrap();

        mailer.send(msg.into()).await.unwrap();

        assert_eq!(mailer.sent_count(), 1);
    }

    #[tokio::test]
    async fn memory_mailer_subject_is_stored() {
        let mailer = MemoryMailer::new();
        let msg = MessageBuilder::new()
            .from(Address::new("a@b.com"))
            .to(Address::new("c@d.com"))
            .subject("Unique Subject 123")
            .text("body")
            .build()
            .unwrap();

        mailer.send(msg.into()).await.unwrap();

        assert!(mailer.was_sent_with_subject("Unique Subject 123"));
    }

    #[tokio::test]
    async fn memory_mailer_was_sent_to_returns_true_for_recipient() {
        let mailer = MemoryMailer::new();
        let msg = MessageBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("target@example.com"))
            .subject("Test")
            .text("body")
            .build()
            .unwrap();

        mailer.send(msg.into()).await.unwrap();

        assert!(mailer.was_sent_to("target@example.com"));
        assert!(!mailer.was_sent_to("other@example.com"));
    }

    #[tokio::test]
    async fn memory_mailer_last_message_returns_most_recent() {
        let mailer = MemoryMailer::new();
        for subject in ["First", "Second", "Third"] {
            let msg = MessageBuilder::new()
                .from(Address::new("a@b.com"))
                .to(Address::new("c@d.com"))
                .subject(subject)
                .text("body")
                .build()
                .unwrap();
            mailer.send(msg.into()).await.unwrap();
        }

        let last = mailer.last_message().unwrap();
        assert_eq!(last.subject, "Third");
        assert_eq!(mailer.sent_count(), 3);
    }

    #[tokio::test]
    async fn memory_mailer_clear_empties_inbox() {
        let mailer = MemoryMailer::new();
        let msg = MessageBuilder::new()
            .from(Address::new("a@b.com"))
            .to(Address::new("c@d.com"))
            .subject("S")
            .text("body")
            .build()
            .unwrap();
        mailer.send(msg.into()).await.unwrap();

        mailer.clear();
        assert_eq!(mailer.sent_count(), 0);
        assert!(mailer.last_message().is_none());
    }

    // ───────── Mail / MailBody ─────────

    #[test]
    fn mail_with_html_body() {
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .subject("HTML Mail")
            .html("<h1>Hello</h1>")
            .build()
            .unwrap();

        assert!(mail.has_html());
        assert!(!mail.has_text());
        assert_eq!(mail.html(), Some("<h1>Hello</h1>"));
    }

    #[test]
    fn mail_with_text_body() {
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .subject("Text Mail")
            .text("Plain text")
            .build()
            .unwrap();

        assert!(!mail.has_html());
        assert!(mail.has_text());
        assert_eq!(mail.text(), Some("Plain text"));
    }

    #[test]
    fn mail_with_both_html_and_text_body() {
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .subject("Both")
            .html("<p>Hi</p>")
            .text("Hi")
            .build()
            .unwrap();

        assert!(mail.has_html());
        assert!(mail.has_text());
        assert!(matches!(mail.body, MailBody::Both { .. }));
    }

    #[test]
    fn mail_cc_and_bcc_are_stored() {
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .cc(Address::new("cc@example.com"))
            .bcc(Address::new("bcc@example.com"))
            .subject("Copies")
            .text("body")
            .build()
            .unwrap();

        assert_eq!(mail.cc.len(), 1);
        assert_eq!(mail.cc[0].email, "cc@example.com");
        assert_eq!(mail.bcc.len(), 1);
        assert_eq!(mail.bcc[0].email, "bcc@example.com");
    }

    #[test]
    fn mail_multiple_recipients() {
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("r1@example.com"))
            .to(Address::new("r2@example.com"))
            .to(Address::new("r3@example.com"))
            .subject("Bulk")
            .text("body")
            .build()
            .unwrap();

        assert_eq!(mail.to.len(), 3);
        assert_eq!(mail.recipient_count(), 3);
    }

    #[test]
    fn mail_to_many_helper() {
        let addresses = vec![
            Address::new("a@example.com"),
            Address::new("b@example.com"),
        ];
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to_many(addresses)
            .subject("To Many")
            .text("body")
            .build()
            .unwrap();

        assert_eq!(mail.to.len(), 2);
    }

    #[test]
    fn mail_with_attachment() {
        let data = b"pdf content".to_vec();
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .subject("With Attachment")
            .text("See attached")
            .attach_data(data.clone(), "report.pdf", "application/pdf")
            .build()
            .unwrap();

        assert_eq!(mail.attachments.len(), 1);
        assert_eq!(mail.attachments[0].filename, "report.pdf");
        assert_eq!(mail.attachments[0].content_type, "application/pdf");
        assert_eq!(mail.attachments[0].size(), data.len());
    }

    #[test]
    fn mail_from_address_with_display_name() {
        let mail = MailBuilder::new()
            .from(Address::with_name("from@example.com", "Acme Corp"))
            .to(Address::new("to@example.com"))
            .subject("Named Sender")
            .text("body")
            .build()
            .unwrap();

        assert_eq!(mail.from.name, Some("Acme Corp".to_string()));
        assert_eq!(mail.from.format(), "Acme Corp <from@example.com>");
    }

    // ───────── MailBuilder validation ─────────

    #[test]
    fn mail_builder_requires_from() {
        let result = MailBuilder::new()
            .to(Address::new("to@example.com"))
            .subject("No From")
            .text("body")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn mail_builder_requires_body() {
        let result = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .subject("No Body")
            .build();
        assert!(result.is_err());
    }

    // ───────── MailFake / testing ─────────

    #[tokio::test]
    async fn mail_fake_captures_sent_mails() {
        let fake = MailFake::new();
        let msg = MessageBuilder::new()
            .from(Address::new("a@b.com"))
            .to(Address::new("c@d.com"))
            .subject("Captured")
            .text("body")
            .build()
            .unwrap();

        fake.send(msg.into()).await.unwrap();

        assert_eq!(fake.sent_messages().len(), 1);
        fake.assert_sent_count(1);
    }

    #[tokio::test]
    async fn mail_fake_assert_sent_predicate() {
        let fake = MailFake::new();
        let msg = MessageBuilder::new()
            .from(Address::new("a@b.com"))
            .to(Address::new("c@d.com"))
            .subject("Welcome to RustForge")
            .text("body")
            .build()
            .unwrap();

        fake.send(msg.into()).await.unwrap();

        fake.assert_sent(|m| m.subject.contains("Welcome"));
        fake.assert_not_sent(|m| m.subject.contains("Invoice"));
    }

    #[tokio::test]
    async fn mail_fake_sent_to_filters_by_address() {
        let fake = MailFake::new();
        let msg = MessageBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Filter")
            .text("body")
            .build()
            .unwrap();

        fake.send(msg.into()).await.unwrap();

        assert_eq!(fake.sent_to("user@example.com").len(), 1);
        assert_eq!(fake.sent_to("nobody@example.com").len(), 0);
    }

    #[tokio::test]
    async fn mail_fake_with_subject_filters_by_subject() {
        let fake = MailFake::new();
        for subject in ["Invoice Paid", "Order Shipped", "Invoice Overdue"] {
            let msg = MessageBuilder::new()
                .from(Address::new("a@b.com"))
                .to(Address::new("c@d.com"))
                .subject(subject)
                .text("body")
                .build()
                .unwrap();
            fake.send(msg.into()).await.unwrap();
        }
        let invoices = fake.with_subject("Invoice");
        assert_eq!(invoices.len(), 2);
    }

    #[tokio::test]
    async fn mail_fake_nothing_sent_passes_when_empty() {
        let fake = MailFake::new();
        fake.assert_nothing_sent(); // should not panic
    }

    #[tokio::test]
    async fn mail_fake_assert_sent_any() {
        let fake = MailFake::new();
        let msg = MessageBuilder::new()
            .from(Address::new("a@b.com"))
            .to(Address::new("c@d.com"))
            .subject("Any")
            .text("body")
            .build()
            .unwrap();
        fake.send(msg.into()).await.unwrap();
        fake.assert_sent_any();
    }

    // ───────── Template rendering (Handlebars) ─────────

    #[test]
    fn template_engine_renders_simple_template() {
        let mut engine = TemplateEngine::new();
        engine.register_template("hello", "Hello, {{name}}!").unwrap();
        let data = serde_json::json!({ "name": "Bob" });
        let output = engine.render("hello", &data).unwrap();
        assert_eq!(output, "Hello, Bob!");
    }

    #[test]
    fn template_engine_renders_complex_template() {
        let mut engine = TemplateEngine::new();
        engine
            .register_template(
                "invoice",
                "Invoice #{{id}} for {{customer}} - Total: {{total}}",
            )
            .unwrap();
        let data = serde_json::json!({
            "id": 42,
            "customer": "Alice",
            "total": "$99.00"
        });
        let output = engine.render("invoice", &data).unwrap();
        assert!(output.contains("42"));
        assert!(output.contains("Alice"));
        assert!(output.contains("$99.00"));
    }

    #[test]
    fn template_engine_unknown_template_returns_error() {
        let engine = TemplateEngine::new();
        let data = serde_json::json!({});
        assert!(engine.render("nonexistent", &data).is_err());
    }

    // ───────── Mailable trait ─────────

    #[tokio::test]
    async fn mailable_send_via_memory_mailer() {
        struct WelcomeMail {
            to: String,
        }
        impl Mailable for WelcomeMail {
            fn build(&self) -> MailBuilder {
                MailBuilder::new()
                    .from(Address::new("noreply@rustforge.rs"))
                    .to(Address::new(self.to.as_str()))
                    .subject("Welcome!")
                    .text("Thanks for joining!")
            }
        }

        let mailer = MemoryMailer::new();
        let mail = WelcomeMail { to: "new@user.com".into() };
        mail.send(&mailer).await.unwrap();

        assert_eq!(mailer.sent_count(), 1);
        assert!(mailer.was_sent_to("new@user.com"));
        assert!(mailer.was_sent_with_subject("Welcome!"));
    }

    // ───────── MailManager memory backend ─────────

    #[tokio::test]
    async fn mail_manager_memory_sends_mail() {
        use crate::manager::MailManager;
        use crate::Mailer;
        use crate::{Address, Mail, MailBody};

        let manager = MailManager::memory();
        let mail = Mail {
            from: Address::new("noreply@example.com"),
            to: vec![Address::new("user@example.com")],
            cc: vec![],
            bcc: vec![],
            reply_to: None,
            subject: "Test via Manager".into(),
            body: MailBody::Text("Hello from manager".into()),
            attachments: vec![],
            id: uuid::Uuid::new_v4().to_string(),
        };

        // MailManager::memory() wraps a MemoryMailer; send should succeed
        manager.send(mail).await.unwrap();
    }
}
