//! Integration tests for rf-mail

use rf_mail::prelude::*;

#[tokio::test]
async fn test_simple_mail_builder() {
    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Test Email")
        .text("Hello, World!")
        .build()
        .unwrap();

    assert_eq!(mail.from.email, "sender@example.com");
    assert_eq!(mail.to[0].email, "recipient@example.com");
    assert_eq!(mail.subject, "Test Email");
    assert!(mail.has_text());
}

#[tokio::test]
async fn test_html_and_text_mail() {
    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Test")
        .html("<h1>Hello</h1>")
        .text("Hello")
        .build()
        .unwrap();

    assert!(mail.has_html());
    assert!(mail.has_text());
    assert_eq!(mail.html(), Some("<h1>Hello</h1>"));
    assert_eq!(mail.text(), Some("Hello"));
}

#[tokio::test]
async fn test_markdown_mail() {
    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Markdown Test")
        .markdown("# Hello\n\nThis is **bold**.")
        .build()
        .unwrap();

    assert!(mail.has_html());
    assert!(mail.has_text());

    let html = mail.html().unwrap();
    assert!(html.contains("<h1>"));
    assert!(html.contains("<strong>"));
}

#[tokio::test]
async fn test_markdown_button_component() {
    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Button Test")
        .markdown("@button(https://example.com)\nClick Me\n@endbutton")
        .build()
        .unwrap();

    let html = mail.html().unwrap();
    assert!(html.contains("href=\"https://example.com\""));
    assert!(html.contains("Click Me"));
}

#[tokio::test]
async fn test_markdown_panel_component() {
    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Panel Test")
        .markdown("@panel\nImportant Info\n@endpanel")
        .build()
        .unwrap();

    let html = mail.html().unwrap();
    assert!(html.contains("Important Info"));
    assert!(html.contains("background-color"));
}

#[tokio::test]
async fn test_markdown_table_component() {
    let markdown = r#"@table
| Name | Age |
|------|-----|
| Alice | 30 |
@endtable"#;

    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Table Test")
        .markdown(markdown)
        .build()
        .unwrap();

    let html = mail.html().unwrap();
    assert!(html.contains("<table"));
    assert!(html.contains("Alice"));
}

#[tokio::test]
async fn test_memory_mailer() {
    let mailer = MemoryMailer::new();

    let message = MessageBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Test")
        .text("Hello")
        .build()
        .unwrap();

    mailer.send(message.into()).await.unwrap();

    let sent = mailer.sent_messages();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].subject, "Test");
}

#[tokio::test]
async fn test_mailable_trait() {
    struct TestMail {
        to: String,
        name: String,
    }

    impl Mailable for TestMail {
        fn build(&self) -> MailBuilder {
            MailBuilder::new()
                .from(Address::new("noreply@example.com"))
                .to(Address::new(&self.to))
                .subject("Test Mail")
                .text(format!("Hello, {}!", self.name))
        }
    }

    let mailer = MemoryMailer::new();
    let mail = TestMail {
        to: "user@example.com".into(),
        name: "Alice".into(),
    };

    mail.send(&mailer).await.unwrap();

    let sent = mailer.sent_messages();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].to[0].email, "user@example.com");
}

#[tokio::test]
async fn test_mail_fake() {
    use rf_mail::testing::{assert_sent, assert_sent_count, fake, restore};

    let fake_mailer = fake();

    let message = MessageBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("Fake Test")
        .text("Hello")
        .build()
        .unwrap();

    fake_mailer.send(message.into()).await.unwrap();

    assert_sent_count(1);
    assert_sent(|msg| msg.subject == "Fake Test");

    restore();
}

#[tokio::test]
async fn test_welcome_email_mailable() {
    let mailer = MemoryMailer::new();

    let welcome = WelcomeEmail {
        to: Address::new("user@example.com"),
        user_name: "Alice".into(),
        app_name: "TestApp".into(),
    };

    welcome.send(&mailer).await.unwrap();

    let sent = mailer.sent_messages();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].subject.contains("Welcome"));
}

#[tokio::test]
async fn test_order_shipped_mailable() {
    let mailer = MemoryMailer::new();

    let order_shipped = OrderShippedMail {
        to: "customer@example.com".into(),
        customer_name: "Bob".into(),
        order_id: "ORD-123".into(),
        tracking_url: "https://tracking.example.com/123".into(),
    };

    let mail = order_shipped.build().build().unwrap();

    // Convert Mail to Message for sending
    let message = MessageBuilder::new()
        .from(mail.from)
        .to_many(mail.to)
        .subject(mail.subject)
        .html(mail.html().unwrap_or(""))
        .text(mail.text().unwrap_or(""))
        .build()
        .unwrap();

    mailer.send(message.into()).await.unwrap();

    let sent = mailer.sent_messages();
    assert_eq!(sent.len(), 1);
    assert!(sent[0].subject.contains("Shipped"));
}

#[tokio::test]
async fn test_attachment() {
    let data = b"Hello, World!".to_vec();

    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("recipient@example.com"))
        .subject("With Attachment")
        .text("See attached file")
        .attach_data(data, "hello.txt", "text/plain")
        .build()
        .unwrap();

    assert_eq!(mail.attachments.len(), 1);
    assert_eq!(mail.attachments[0].filename, "hello.txt");
    assert_eq!(mail.attachments[0].content_type, "text/plain");
}

#[tokio::test]
async fn test_multiple_recipients() {
    let mail = MailBuilder::new()
        .from(Address::new("sender@example.com"))
        .to(Address::new("user1@example.com"))
        .to(Address::new("user2@example.com"))
        .cc(Address::new("cc@example.com"))
        .bcc(Address::new("bcc@example.com"))
        .subject("Multiple Recipients")
        .text("Hello everyone")
        .build()
        .unwrap();

    assert_eq!(mail.to.len(), 2);
    assert_eq!(mail.cc.len(), 1);
    assert_eq!(mail.bcc.len(), 1);
    assert_eq!(mail.recipient_count(), 4);
}

#[tokio::test]
async fn test_mail_config() {
    let smtp = SmtpMailConfig::new("localhost", 1025).with_credentials("user", "pass");

    let config = MailConfig::smtp(Address::new("noreply@example.com"), smtp);

    assert_eq!(config.driver, MailDriver::Smtp);
    assert!(config.validate().is_ok());
}

#[tokio::test]
async fn test_smtp_config_presets() {
    let gmail = SmtpMailConfig::gmail("user@gmail.com", "password");
    assert_eq!(gmail.host, "smtp.gmail.com");
    assert_eq!(gmail.port, 587);

    let sendgrid = SmtpMailConfig::sendgrid("api_key");
    assert_eq!(sendgrid.host, "smtp.sendgrid.net");
}
