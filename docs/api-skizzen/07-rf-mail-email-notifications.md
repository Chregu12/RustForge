# API-Skizze: rf-mail - Email & Notifications System

**Phase**: Phase 2 - Modular Rebuild
**PR-Slice**: #8
**Status**: Planning
**Date**: 2025-11-09

## 1. Overview

The `rf-mail` crate provides a unified email and notification system with multiple backend support, template rendering, and queue integration.

**Key Features:**
- Multiple backends (SMTP, Memory, Mock)
- Email builders with attachments
- Template support with handlebars
- Queue integration with rf-jobs
- Rich email formatting (HTML + Plain Text)
- Testing support

**Comparison with Laravel Mail:**
- ✅ Mailable trait (like Laravel's Mailable)
- ✅ Multiple drivers (SMTP, Memory)
- ✅ Template rendering
- ✅ Queue integration
- ✅ Attachments support
- ⏳ Mailables with markdown (future)
- ⏳ Notification channels (future)

## 2. Core Types

### 2.1 Mailer Trait

```rust
use async_trait::async_trait;
use std::error::Error;

#[async_trait]
pub trait Mailer: Send + Sync {
    /// Send an email
    async fn send(&self, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>>;

    /// Send multiple emails
    async fn send_batch(&self, messages: &[Message]) -> Result<(), Box<dyn Error + Send + Sync>> {
        for message in messages {
            self.send(message).await?;
        }
        Ok(())
    }
}

#[async_trait]
pub trait Mailable: Send + Sync {
    /// Build the email message
    fn build(&self) -> Result<Message, Box<dyn Error + Send + Sync>>;

    /// Send immediately
    async fn send(&self, mailer: &dyn Mailer) -> Result<(), Box<dyn Error + Send + Sync>> {
        let message = self.build()?;
        mailer.send(&message).await
    }

    /// Queue for later sending (requires rf-jobs integration)
    fn queue(&self) -> Option<String> {
        None // Override to enable queueing
    }
}
```

### 2.2 Message Structure

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Message ID
    pub id: String,

    /// From address
    pub from: Address,

    /// To addresses
    pub to: Vec<Address>,

    /// CC addresses
    pub cc: Vec<Address>,

    /// BCC addresses
    pub bcc: Vec<Address>,

    /// Reply-to address
    pub reply_to: Option<Address>,

    /// Subject
    pub subject: String,

    /// HTML body
    pub html: Option<String>,

    /// Plain text body
    pub text: Option<String>,

    /// Attachments
    pub attachments: Vec<Attachment>,

    /// Custom headers
    pub headers: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

impl Address {
    pub fn new(email: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: None,
        }
    }

    pub fn with_name(email: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            email: email.into(),
            name: Some(name.into()),
        }
    }
}
```

### 2.3 Message Builder

```rust
pub struct MessageBuilder {
    message: Message,
}

impl MessageBuilder {
    pub fn new() -> Self {
        Self {
            message: Message {
                id: uuid::Uuid::new_v4().to_string(),
                from: Address::new(""),
                to: Vec::new(),
                cc: Vec::new(),
                bcc: Vec::new(),
                reply_to: None,
                subject: String::new(),
                html: None,
                text: None,
                attachments: Vec::new(),
                headers: HashMap::new(),
            },
        }
    }

    pub fn from(mut self, address: Address) -> Self {
        self.message.from = address;
        self
    }

    pub fn to(mut self, address: Address) -> Self {
        self.message.to.push(address);
        self
    }

    pub fn cc(mut self, address: Address) -> Self {
        self.message.cc.push(address);
        self
    }

    pub fn bcc(mut self, address: Address) -> Self {
        self.message.bcc.push(address);
        self
    }

    pub fn reply_to(mut self, address: Address) -> Self {
        self.message.reply_to = Some(address);
        self
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.message.subject = subject.into();
        self
    }

    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.message.html = Some(html.into());
        self
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.message.text = Some(text.into());
        self
    }

    pub fn attach(mut self, attachment: Attachment) -> Self {
        self.message.attachments.push(attachment);
        self
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.message.headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Result<Message, MailError> {
        // Validate message
        if self.message.from.email.is_empty() {
            return Err(MailError::InvalidMessage("From address is required".into()));
        }
        if self.message.to.is_empty() {
            return Err(MailError::InvalidMessage("At least one To address is required".into()));
        }
        if self.message.subject.is_empty() {
            return Err(MailError::InvalidMessage("Subject is required".into()));
        }
        if self.message.html.is_none() && self.message.text.is_none() {
            return Err(MailError::InvalidMessage("Either HTML or text body is required".into()));
        }

        Ok(self.message)
    }
}
```

## 3. Backend Implementations

### 3.1 SMTP Backend

```rust
use lettre::{
    transport::smtp::authentication::Credentials,
    Message as LettreMessage, SmtpTransport, Transport,
};

pub struct SmtpMailer {
    transport: SmtpTransport,
}

impl SmtpMailer {
    pub fn new(config: SmtpConfig) -> Result<Self, MailError> {
        let credentials = Credentials::new(
            config.username.clone(),
            config.password.clone(),
        );

        let transport = SmtpTransport::relay(&config.host)?
            .port(config.port)
            .credentials(credentials)
            .build();

        Ok(Self { transport })
    }
}

#[async_trait]
impl Mailer for SmtpMailer {
    async fn send(&self, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>> {
        let lettre_message = convert_to_lettre(message)?;

        // Send in blocking task to avoid blocking async runtime
        let transport = self.transport.clone();
        tokio::task::spawn_blocking(move || {
            transport.send(&lettre_message)
        })
        .await??;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmtpConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub from_address: String,
    pub from_name: Option<String>,
}

fn convert_to_lettre(message: &Message) -> Result<LettreMessage, MailError> {
    let mut builder = LettreMessage::builder()
        .from(format!("{} <{}>",
            message.from.name.as_deref().unwrap_or(""),
            message.from.email
        ).parse()?)
        .subject(&message.subject);

    for to in &message.to {
        builder = builder.to(format!("{} <{}>",
            to.name.as_deref().unwrap_or(""),
            to.email
        ).parse()?);
    }

    // Add body
    let body = if let Some(html) = &message.html {
        if let Some(text) = &message.text {
            // Multipart with both HTML and text
            format!("--boundary\r\nContent-Type: text/plain\r\n\r\n{}\r\n--boundary\r\nContent-Type: text/html\r\n\r\n{}\r\n--boundary--", text, html)
        } else {
            html.clone()
        }
    } else if let Some(text) = &message.text {
        text.clone()
    } else {
        return Err(MailError::InvalidMessage("No body content".into()));
    };

    Ok(builder.body(body)?)
}
```

### 3.2 Memory Backend (Testing)

```rust
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct MemoryMailer {
    sent: Arc<Mutex<Vec<Message>>>,
}

impl MemoryMailer {
    pub fn new() -> Self {
        Self {
            sent: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get all sent messages (for testing)
    pub fn sent_messages(&self) -> Vec<Message> {
        self.sent.lock().unwrap().clone()
    }

    /// Clear sent messages
    pub fn clear(&self) {
        self.sent.lock().unwrap().clear();
    }

    /// Check if message was sent
    pub fn was_sent_to(&self, email: &str) -> bool {
        self.sent.lock().unwrap().iter().any(|msg| {
            msg.to.iter().any(|addr| addr.email == email)
        })
    }
}

#[async_trait]
impl Mailer for MemoryMailer {
    async fn send(&self, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>> {
        self.sent.lock().unwrap().push(message.clone());
        tracing::info!(
            to = ?message.to,
            subject = %message.subject,
            "Email stored in memory"
        );
        Ok(())
    }
}
```

### 3.3 Mock Backend (Testing with Assertions)

```rust
#[derive(Clone)]
pub struct MockMailer {
    should_fail: bool,
}

impl MockMailer {
    pub fn new() -> Self {
        Self { should_fail: false }
    }

    pub fn with_failure() -> Self {
        Self { should_fail: true }
    }
}

#[async_trait]
impl Mailer for MockMailer {
    async fn send(&self, message: &Message) -> Result<(), Box<dyn Error + Send + Sync>> {
        if self.should_fail {
            return Err(Box::new(MailError::SendFailed("Mock failure".into())));
        }

        tracing::debug!(
            to = ?message.to,
            subject = %message.subject,
            "Mock email sent"
        );

        Ok(())
    }
}
```

## 4. Template Support

### 4.1 Template Engine

```rust
use handlebars::Handlebars;
use serde::Serialize;

pub struct TemplateEngine {
    handlebars: Handlebars<'static>,
}

impl TemplateEngine {
    pub fn new() -> Self {
        Self {
            handlebars: Handlebars::new(),
        }
    }

    /// Register a template
    pub fn register_template(&mut self, name: &str, template: &str) -> Result<(), MailError> {
        self.handlebars.register_template_string(name, template)?;
        Ok(())
    }

    /// Render template with data
    pub fn render<T: Serialize>(&self, name: &str, data: &T) -> Result<String, MailError> {
        Ok(self.handlebars.render(name, data)?)
    }
}

/// Mailable with template support
pub trait TemplateMailable: Mailable {
    /// Template name
    fn template(&self) -> &str;

    /// Template data
    fn data(&self) -> serde_json::Value;

    /// Build message using template
    fn build_with_template(&self, engine: &TemplateEngine) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let html = engine.render(self.template(), &self.data())?;

        let mut builder = MessageBuilder::new();
        builder = builder.html(html);

        // Subclasses override to add to, from, subject, etc.
        Ok(builder.build()?)
    }
}
```

### 4.2 Common Email Templates

```rust
/// Welcome email
pub struct WelcomeEmail {
    pub to: Address,
    pub user_name: String,
    pub app_name: String,
}

impl Mailable for WelcomeEmail {
    fn build(&self) -> Result<Message, Box<dyn Error + Send + Sync>> {
        MessageBuilder::new()
            .from(Address::with_name("noreply@example.com", &self.app_name))
            .to(self.to.clone())
            .subject(format!("Welcome to {}!", self.app_name))
            .html(format!(
                r#"
                <html>
                    <body>
                        <h1>Welcome, {}!</h1>
                        <p>Thank you for joining {}.</p>
                    </body>
                </html>
                "#,
                self.user_name, self.app_name
            ))
            .text(format!(
                "Welcome, {}!\n\nThank you for joining {}.",
                self.user_name, self.app_name
            ))
            .build()
            .map_err(Into::into)
    }
}

/// Password reset email
pub struct PasswordResetEmail {
    pub to: Address,
    pub reset_token: String,
    pub reset_url: String,
    pub expires_minutes: u32,
}

impl Mailable for PasswordResetEmail {
    fn build(&self) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let url = format!("{}?token={}", self.reset_url, self.reset_token);

        MessageBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(self.to.clone())
            .subject("Reset Your Password")
            .html(format!(
                r#"
                <html>
                    <body>
                        <h1>Password Reset</h1>
                        <p>Click the link below to reset your password:</p>
                        <a href="{}">Reset Password</a>
                        <p>This link expires in {} minutes.</p>
                    </body>
                </html>
                "#,
                url, self.expires_minutes
            ))
            .text(format!(
                "Password Reset\n\nVisit this link to reset your password:\n{}\n\nThis link expires in {} minutes.",
                url, self.expires_minutes
            ))
            .build()
            .map_err(Into::into)
    }
}
```

## 5. Queue Integration

### 5.1 Queueable Mail Job

```rust
use rf_jobs::{Job, JobContext, JobResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMailJob {
    message: Message,
}

#[async_trait]
impl Job for SendMailJob {
    async fn handle(&self, ctx: JobContext) -> JobResult {
        ctx.log("Sending queued email");

        // Get mailer from app container
        // For now, we'll use SMTP from config
        let mailer = get_mailer_from_context(&ctx)?;

        mailer.send(&self.message).await?;

        ctx.log(&format!("Email sent to {:?}", self.message.to));
        Ok(())
    }

    fn queue(&self) -> &str {
        "emails"
    }

    fn max_attempts(&self) -> u32 {
        3
    }

    fn backoff(&self) -> Duration {
        Duration::from_secs(60)
    }
}

impl SendMailJob {
    pub fn new(message: Message) -> Self {
        Self { message }
    }
}

// Helper to queue a mailable
pub async fn queue_mailable<M: Mailable>(
    mailable: &M,
    queue_manager: &QueueManager,
) -> Result<uuid::Uuid, Box<dyn Error + Send + Sync>> {
    let message = mailable.build()?;
    let job = SendMailJob::new(message);
    Ok(queue_manager.dispatch(job).await?)
}
```

## 6. Error Handling

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MailError {
    #[error("Invalid message: {0}")]
    InvalidMessage(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Template error: {0}")]
    TemplateError(#[from] handlebars::RenderError),

    #[error("SMTP error: {0}")]
    SmtpError(#[from] lettre::error::Error),

    #[error("Address parse error: {0}")]
    AddressError(#[from] lettre::address::AddressError),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}
```

## 7. Configuration

### 7.1 Mail Config

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailConfig {
    /// Default mailer driver
    pub driver: MailDriver,

    /// From address (default)
    pub from_address: String,

    /// From name (default)
    pub from_name: String,

    /// SMTP configuration
    pub smtp: Option<SmtpConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MailDriver {
    Smtp,
    Memory,
    Mock,
}

impl Default for MailConfig {
    fn default() -> Self {
        Self {
            driver: MailDriver::Mock,
            from_address: "noreply@example.com".into(),
            from_name: "Application".into(),
            smtp: None,
        }
    }
}
```

### 7.2 Config File Example

```toml
[mail]
driver = "smtp"
from_address = "noreply@myapp.com"
from_name = "MyApp"

[mail.smtp]
host = "smtp.gmail.com"
port = 587
username = "user@gmail.com"
password = "app_password"
from_address = "noreply@myapp.com"
from_name = "MyApp"
```

## 8. Usage Examples

### 8.1 Basic Email

```rust
use rf_mail::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create mailer
    let mailer = MemoryMailer::new();

    // Build and send email
    let message = MessageBuilder::new()
        .from(Address::with_name("sender@example.com", "Sender"))
        .to(Address::with_name("recipient@example.com", "Recipient"))
        .subject("Hello from rf-mail!")
        .html("<h1>Hello!</h1><p>This is a test email.</p>")
        .text("Hello!\n\nThis is a test email.")
        .build()?;

    mailer.send(&message).await?;

    println!("Email sent!");
    Ok(())
}
```

### 8.2 Using Mailable

```rust
// Send welcome email
let welcome = WelcomeEmail {
    to: Address::with_name("user@example.com", "John Doe"),
    user_name: "John".into(),
    app_name: "MyApp".into(),
};

welcome.send(&mailer).await?;
```

### 8.3 Queueing Emails

```rust
use rf_jobs::QueueManager;

// Queue welcome email
let queue_manager = QueueManager::new("redis://localhost:6379").await?;
let job_id = queue_mailable(&welcome, &queue_manager).await?;

println!("Email queued with ID: {}", job_id);
```

### 8.4 With Attachments

```rust
let pdf_data = std::fs::read("report.pdf")?;

let message = MessageBuilder::new()
    .from(Address::new("sender@example.com"))
    .to(Address::new("recipient@example.com"))
    .subject("Monthly Report")
    .html("<p>Please find the monthly report attached.</p>")
    .attach(Attachment {
        filename: "report.pdf".into(),
        content_type: "application/pdf".into(),
        data: pdf_data,
    })
    .build()?;

mailer.send(&message).await?;
```

### 8.5 Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_email_sending() {
        let mailer = MemoryMailer::new();

        let welcome = WelcomeEmail {
            to: Address::new("test@example.com"),
            user_name: "Test User".into(),
            app_name: "TestApp".into(),
        };

        welcome.send(&mailer).await.unwrap();

        // Verify email was sent
        assert!(mailer.was_sent_to("test@example.com"));

        let sent = mailer.sent_messages();
        assert_eq!(sent.len(), 1);
        assert_eq!(sent[0].subject, "Welcome to TestApp!");
    }
}
```

## 9. Implementation Plan

### Phase 1: Core (30 minutes)
- [ ] Create rf-mail crate
- [ ] Implement Message and Address types
- [ ] Implement MessageBuilder
- [ ] Implement Mailer and Mailable traits
- [ ] Add error types

### Phase 2: Backends (45 minutes)
- [ ] Implement MemoryMailer
- [ ] Implement MockMailer
- [ ] Implement SmtpMailer with lettre
- [ ] Add configuration support

### Phase 3: Templates & Common Emails (30 minutes)
- [ ] Integrate handlebars
- [ ] Implement TemplateEngine
- [ ] Create WelcomeEmail
- [ ] Create PasswordResetEmail

### Phase 4: Queue Integration (20 minutes)
- [ ] Implement SendMailJob
- [ ] Add queue_mailable helper
- [ ] Test with rf-jobs integration

### Phase 5: Examples & Tests (25 minutes)
- [ ] Create mail-demo example
- [ ] Write unit tests
- [ ] Write integration tests
- [ ] Add documentation

**Total Estimated Time: 2.5 hours**

## 10. Dependencies

```toml
[dependencies]
async-trait.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
tokio.workspace = true
uuid.workspace = true

# Email
lettre = { version = "0.11", features = ["tokio1-rustls-tls", "smtp-transport", "builder"] }
handlebars = "5.1"

# Optional
rf-jobs = { path = "../rf-jobs", optional = true }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

## 11. Testing Strategy

### Unit Tests
- Message validation
- Builder pattern
- Address parsing
- Template rendering

### Integration Tests
- MemoryMailer storage
- SMTP sending (with test server)
- Queue integration
- Error handling

### Examples
- Basic email sending
- Mailable usage
- Template emails
- Attachment handling
- Queue integration

## 12. Future Enhancements

1. **Markdown Emails**: Laravel-like markdown mailables
2. **Notification Channels**: Multi-channel notifications (email, SMS, Slack)
3. **Email Verification**: Built-in email verification flow
4. **Inline Images**: Support for embedded images
5. **Email Analytics**: Track opens and clicks
6. **Rate Limiting**: Prevent email spam
7. **AWS SES Backend**: Amazon SES integration
8. **Sendgrid Backend**: Sendgrid API integration

## 13. Comparison with Laravel

| Feature | Laravel | rf-mail | Status |
|---------|---------|---------|--------|
| Mailable class | ✅ | ✅ | ✅ Complete |
| Multiple drivers | ✅ | ✅ | ✅ Complete |
| Queue emails | ✅ | ✅ | ✅ Complete |
| Attachments | ✅ | ✅ | ✅ Complete |
| Templates | ✅ | ✅ | ✅ Complete |
| Markdown emails | ✅ | ⏳ | ⏳ Future |
| Notifications | ✅ | ⏳ | ⏳ Future |
| Email verification | ✅ | ⏳ | ⏳ Future |
| Preview in browser | ✅ | ⏳ | ⏳ Future |

**Feature Parity**: ~65% (6/9 core features)

---

**Estimated Lines of Code**: ~1,500 production + ~400 tests + ~300 examples = **~2,200 total**
