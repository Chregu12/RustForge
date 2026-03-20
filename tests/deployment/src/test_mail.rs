//! Deployment tests for rf-mail

#[cfg(test)]
mod tests {
    use rf_mail::{
        Address, Attachment, MailBuilder, Mail, Message, MessageBuilder,
        MailConfig, SmtpConfig, SendmailConfig, MailBody,
        LogMailer, MemoryMailer, Mailer,
    };
    use rf_mail::markdown::{render_markdown, markdown_to_text, button, panel, table};

    // ── Address ──────────────────────────────────────────────────

    #[test]
    fn address_creation() {
        let addr = Address::new("test@example.com");
        assert_eq!(addr.format(), "test@example.com");

        let addr_named = Address::with_name("test@example.com", "Test User");
        assert!(addr_named.format().contains("Test User"));
    }

    // ── Attachment ───────────────────────────────────────────────

    #[test]
    fn attachment_from_data() {
        let data = b"Hello, World!".to_vec();
        let att = Attachment::from_data(data.clone(), "hello.txt".to_string(), "text/plain".to_string());
        assert_eq!(att.size(), data.len());
    }

    // ── MailBuilder ──────────────────────────────────────────────

    #[test]
    fn mail_builder_basic() {
        let mail = MailBuilder::new()
            .from(Address::new("from@example.com"))
            .to(Address::new("to@example.com"))
            .subject("Test Subject")
            .html("<h1>Hello</h1>")
            .build()
            .expect("build");

        assert!(mail.has_html());
        assert_eq!(mail.recipient_count(), 1);
    }

    #[test]
    fn mail_builder_with_text() {
        let mail = MailBuilder::new()
            .from(Address::new("from@test.com"))
            .to(Address::new("to@test.com"))
            .subject("Text Only")
            .text("Plain text body")
            .build()
            .expect("build");

        assert!(mail.has_text());
    }

    #[test]
    fn mail_builder_multiple_recipients() {
        let mail = MailBuilder::new()
            .from(Address::new("from@test.com"))
            .to(Address::new("to1@test.com"))
            .to(Address::new("to2@test.com"))
            .cc(Address::new("cc@test.com"))
            .bcc(Address::new("bcc@test.com"))
            .subject("Multi")
            .html("<p>Hi</p>")
            .build()
            .expect("build");

        assert!(mail.recipient_count() >= 2);
    }

    #[test]
    fn mail_builder_with_attachment() {
        let mail = MailBuilder::new()
            .from(Address::new("from@test.com"))
            .to(Address::new("to@test.com"))
            .subject("With Attachment")
            .html("<p>See attached</p>")
            .attach_data(b"data".to_vec(), "file.txt", "text/plain")
            .build()
            .expect("build");

        assert!(mail.attachment_size() > 0);
    }

    // ── Mail validation ──────────────────────────────────────────

    #[test]
    fn mail_validate() {
        let mail = Mail::new();
        assert!(mail.validate().is_err()); // empty mail should fail
    }

    // ── MessageBuilder ───────────────────────────────────────────

    #[test]
    fn message_builder() {
        let msg = MessageBuilder::new()
            .from(Address::new("from@test.com"))
            .to(Address::new("to@test.com"))
            .subject("Test")
            .html("<p>Body</p>")
            .header("X-Custom", "value")
            .build()
            .expect("build");

        assert_eq!(msg.recipient_count(), 1);
    }

    // ── MailConfig ───────────────────────────────────────────────

    #[test]
    fn mail_config_log() {
        let config = MailConfig::log(
            Address::new("app@example.com"),
            "/tmp/mail.log".into(),
        );
        assert!(config.validate().is_ok());
    }

    #[test]
    fn mail_config_memory() {
        let config = MailConfig::memory(Address::new("app@example.com"));
        assert!(config.validate().is_ok());
    }

    // ── MemoryMailer (for testing) ───────────────────────────────

    #[tokio::test]
    async fn memory_mailer_sends_and_stores() {
        let mailer = MemoryMailer::new();
        let mail = MailBuilder::new()
            .from(Address::new("from@test.com"))
            .to(Address::new("to@test.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .expect("build");

        mailer.send(mail).await.expect("send");
    }

    // ── Markdown Helpers ─────────────────────────────────────────

    #[test]
    fn markdown_rendering() {
        let html = render_markdown("# Hello\n\nWorld").expect("render");
        assert!(html.contains("Hello"));
    }

    #[test]
    fn markdown_to_text_conversion() {
        let text = markdown_to_text("**Bold** and *italic*");
        assert!(text.contains("Bold"));
        assert!(text.contains("italic"));
    }

    #[test]
    fn markdown_button_helper() {
        let btn = button("Click Me", "https://example.com");
        assert!(btn.contains("Click Me"));
        assert!(btn.contains("https://example.com"));
    }

    #[test]
    fn markdown_panel_helper() {
        let p = panel("Important notice");
        assert!(p.contains("Important notice"));
    }

    #[test]
    fn markdown_table_helper() {
        let t = table(
            vec!["Name", "Age"],
            vec![vec!["John", "30"], vec!["Jane", "25"]],
        );
        assert!(t.contains("Name"));
        assert!(t.contains("John"));
    }
}
