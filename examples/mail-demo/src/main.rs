//! Email system demonstration
//!
//! This example demonstrates the rf-mail crate functionality:
//! - Basic email sending
//! - Mailable usage
//! - Template rendering
//! - Attachments
//! - Different backends

use rf_mail::*;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting rf-mail demonstration...\n");

    // Use MemoryMailer for demonstration
    let mailer = MemoryMailer::new();

    demo_basic_email(&mailer).await?;
    demo_welcome_mailable(&mailer).await?;
    demo_password_reset(&mailer).await?;
    demo_attachments(&mailer).await?;
    demo_templates(&mailer).await?;
    demo_multipart(&mailer).await?;
    demo_batch_sending(&mailer).await?;
    demo_testing(&mailer).await?;

    info!("\n✅ All demonstrations completed successfully!");
    info!("📧 Total emails sent: {}", mailer.sent_count());

    Ok(())
}

/// Demo 1: Basic email sending
async fn demo_basic_email(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 1: Basic Email Sending");
    info!("=============================");

    let message = MessageBuilder::new()
        .from(Address::with_name("sender@example.com", "Sender Name"))
        .to(Address::with_name("recipient@example.com", "Recipient Name"))
        .subject("Hello from rf-mail!")
        .html("<h1>Hello!</h1><p>This is a test email.</p>")
        .text("Hello!\n\nThis is a test email.")
        .build()?;

    mailer.send(&message).await?;

    info!("✓ Basic email sent");
    info!("  From: {}", message.from.format());
    info!("  To: {}", message.to[0].format());
    info!("  Subject: {}\n", message.subject);

    Ok(())
}

/// Demo 2: Welcome email using Mailable
async fn demo_welcome_mailable(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 2: Welcome Email (Mailable)");
    info!("==================================");

    let welcome = WelcomeEmail {
        to: Address::with_name("newuser@example.com", "Alice Johnson"),
        user_name: "Alice".into(),
        app_name: "RustForge Demo".into(),
    };

    welcome.send(mailer).await?;

    info!("✓ Welcome email sent");
    info!("  To: {}", welcome.to.format());
    info!("  User: {}\n", welcome.user_name);

    Ok(())
}

/// Demo 3: Password reset email
async fn demo_password_reset(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 3: Password Reset Email");
    info!("=============================");

    let reset = PasswordResetEmail {
        to: Address::new("user@example.com"),
        reset_token: "abc123xyz789".into(),
        reset_url: "https://example.com/reset-password".into(),
        expires_minutes: 60,
    };

    reset.send(mailer).await?;

    info!("✓ Password reset email sent");
    info!("  To: {}", reset.to.email);
    info!("  Token: {}", reset.reset_token);
    info!("  Expires: {} minutes\n", reset.expires_minutes);

    Ok(())
}

/// Demo 4: Email with attachments
async fn demo_attachments(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 4: Email with Attachments");
    info!("===============================");

    // Create mock PDF data
    let pdf_data = b"Mock PDF content - this would be real PDF bytes".to_vec();

    let message = MessageBuilder::new()
        .from(Address::new("reports@example.com"))
        .to(Address::new("user@example.com"))
        .subject("Monthly Report")
        .html("<h2>Monthly Report</h2><p>Please find your monthly report attached.</p>")
        .text("Monthly Report\n\nPlease find your monthly report attached.")
        .attach(Attachment::new("report.pdf", "application/pdf", pdf_data))
        .build()?;

    mailer.send(&message).await?;

    info!("✓ Email with attachment sent");
    info!("  Attachments: {}", message.attachments.len());
    info!(
        "  Attachment size: {} bytes\n",
        message.attachment_size()
    );

    Ok(())
}

/// Demo 5: Template rendering
async fn demo_templates(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 5: Template Rendering");
    info!("===========================");

    let mut engine = TemplateEngine::new();

    engine.register_template(
        "invoice",
        r#"
<!DOCTYPE html>
<html>
<body>
    <h1>Invoice #{{invoice_id}}</h1>
    <p>Dear {{customer_name}},</p>
    <p>Your invoice total is ${{amount}}.</p>
    <p>Thank you for your business!</p>
</body>
</html>
        "#,
    )?;

    let data = serde_json::json!({
        "invoice_id": "INV-001",
        "customer_name": "Bob Smith",
        "amount": "1,234.56"
    });

    let html = engine.render("invoice", &data)?;

    let message = MessageBuilder::new()
        .from(Address::new("billing@example.com"))
        .to(Address::new("bob@example.com"))
        .subject("Invoice #INV-001")
        .html(html)
        .build()?;

    mailer.send(&message).await?;

    info!("✓ Template email sent");
    info!("  Template: invoice");
    info!("  Invoice ID: INV-001\n");

    Ok(())
}

/// Demo 6: Multipart email (HTML + Text)
async fn demo_multipart(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 6: Multipart Email (HTML + Text)");
    info!("======================================");

    let message = MessageBuilder::new()
        .from(Address::with_name("newsletter@example.com", "Newsletter"))
        .to(Address::new("subscriber@example.com"))
        .subject("Weekly Newsletter")
        .html(r#"
            <html>
                <body>
                    <h1>This Week's Highlights</h1>
                    <ul>
                        <li><strong>Feature 1:</strong> New dashboard released</li>
                        <li><strong>Feature 2:</strong> Performance improvements</li>
                    </ul>
                </body>
            </html>
        "#)
        .text(r#"
This Week's Highlights

- Feature 1: New dashboard released
- Feature 2: Performance improvements
        "#)
        .build()?;

    mailer.send(&message).await?;

    info!("✓ Multipart email sent");
    info!("  Has HTML: {}", message.html.is_some());
    info!("  Has Text: {}\n", message.text.is_some());

    Ok(())
}

/// Demo 7: Batch email sending
async fn demo_batch_sending(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 7: Batch Email Sending");
    info!("============================");

    let users = vec![
        ("user1@example.com", "User One"),
        ("user2@example.com", "User Two"),
        ("user3@example.com", "User Three"),
    ];

    let mut messages = Vec::new();

    for (email, name) in users {
        let message = MessageBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::with_name(email, name))
            .subject("System Notification")
            .text(format!("Hello {}, this is a system notification.", name))
            .build()?;

        messages.push(message);
    }

    mailer.send_batch(&messages).await?;

    info!("✓ Batch emails sent");
    info!("  Batch size: {} emails\n", messages.len());

    Ok(())
}

/// Demo 8: Testing utilities
async fn demo_testing(mailer: &MemoryMailer) -> Result<(), Box<dyn std::error::Error>> {
    info!("Demo 8: Testing Utilities");
    info!("==========================");

    // Clear previous messages
    mailer.clear();

    // Send test email
    let message = MessageBuilder::new()
        .from(Address::new("test@example.com"))
        .to(Address::new("testuser@example.com"))
        .subject("Test Email")
        .text("Test content")
        .build()?;

    mailer.send(&message).await?;

    // Test assertions
    assert!(mailer.was_sent_to("testuser@example.com"));
    assert!(mailer.was_sent_with_subject("Test Email"));
    assert_eq!(mailer.sent_count(), 1);

    let last = mailer.last_message().unwrap();
    assert_eq!(last.subject, "Test Email");

    info!("✓ All test assertions passed");
    info!("  was_sent_to: ✓");
    info!("  was_sent_with_subject: ✓");
    info!("  sent_count: ✓");
    info!("  last_message: ✓\n");

    Ok(())
}
