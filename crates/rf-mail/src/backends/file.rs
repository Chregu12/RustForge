//! Filesystem transport - delivers each email as an `.eml` file on disk.
//!
//! This is the real default transport for RustForge's synchronous `Mail` facade.
//! Unlike [`MemoryMailer`](crate::MemoryMailer), which only records messages in
//! memory, the [`FileMailer`] actually *delivers* mail by writing a standard
//! RFC 5322 message to a mailbox directory. The resulting `.eml` files can be
//! opened directly by mail clients (Thunderbird, Outlook, Apple Mail) or
//! inspected in tests.
//!
//! Delivery uses **synchronous `std::fs`** so it can be driven from the sync
//! `Mail::to(..).send(..)` facade without spinning up (or blocking on) a Tokio
//! runtime. The async [`Mailer`] trait is also implemented for use alongside the
//! other backends.

use crate::{Address, Mail, MailBody, MailError, Mailer};
use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};

/// Mailer that writes each message to disk as an `.eml` file.
///
/// # Example
///
/// ```
/// use rf_mail::{FileMailer, MailBuilder, Address};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let dir = std::env::temp_dir().join("rf-mail-doctest");
/// let mailer = FileMailer::new(&dir);
///
/// let mail = MailBuilder::new()
///     .from(Address::new("noreply@example.com"))
///     .to(Address::new("user@example.com"))
///     .subject("Hello")
///     .text("Hi there!")
///     .build()?;
///
/// let path = mailer.deliver(&mail)?;
/// assert!(path.exists());
///
/// let eml = std::fs::read_to_string(&path)?;
/// assert!(eml.contains("Subject: Hello"));
/// assert!(eml.contains("user@example.com"));
/// # let _ = std::fs::remove_file(path);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct FileMailer {
    mailbox: PathBuf,
}

impl FileMailer {
    /// Create a new file mailer writing to `mailbox`.
    ///
    /// The directory is created lazily on first delivery if it does not exist.
    pub fn new(mailbox: impl Into<PathBuf>) -> Self {
        Self {
            mailbox: mailbox.into(),
        }
    }

    /// The mailbox directory messages are written to.
    pub fn mailbox(&self) -> &Path {
        &self.mailbox
    }

    /// Deliver a message synchronously, writing it as an `.eml` file.
    ///
    /// Returns the path of the written file.
    ///
    /// # Errors
    ///
    /// Returns [`MailError::IoError`] if the mailbox directory cannot be created
    /// or the file cannot be written.
    pub fn deliver(&self, mail: &Mail) -> Result<PathBuf, MailError> {
        fs::create_dir_all(&self.mailbox)?;

        let path = self.mailbox.join(Self::file_name(mail));
        let eml = Self::to_eml(mail);
        fs::write(&path, eml)?;

        tracing::info!(
            path = %path.display(),
            subject = %mail.subject,
            "Email delivered to mailbox",
        );

        Ok(path)
    }

    /// Build a filesystem-safe, time-ordered filename for a message.
    fn file_name(mail: &Mail) -> String {
        // Prefix with a UTC timestamp so files sort chronologically, and suffix
        // with the message id so concurrent sends never collide.
        let ts = chrono::Utc::now().format("%Y%m%dT%H%M%S%3f");
        let id = sanitize(&mail.id);
        format!("{ts}-{id}.eml")
    }

    /// Render a [`Mail`] into an RFC 5322 message string.
    fn to_eml(mail: &Mail) -> String {
        let mut out = String::new();

        push_header(&mut out, "From", &format_address(&mail.from));
        push_header(&mut out, "To", &format_address_list(&mail.to));
        if !mail.cc.is_empty() {
            push_header(&mut out, "Cc", &format_address_list(&mail.cc));
        }
        if !mail.bcc.is_empty() {
            push_header(&mut out, "Bcc", &format_address_list(&mail.bcc));
        }
        if let Some(reply_to) = &mail.reply_to {
            push_header(&mut out, "Reply-To", &format_address(reply_to));
        }
        push_header(&mut out, "Subject", &mail.subject);
        push_header(&mut out, "Date", &chrono::Utc::now().to_rfc2822());
        push_header(&mut out, "Message-ID", &format!("<{}@rustforge>", mail.id));
        push_header(&mut out, "MIME-Version", "1.0");

        match &mail.body {
            MailBody::Text(text) => {
                push_header(&mut out, "Content-Type", "text/plain; charset=utf-8");
                out.push_str("\r\n");
                push_body(&mut out, text);
            }
            MailBody::Html(html) => {
                push_header(&mut out, "Content-Type", "text/html; charset=utf-8");
                out.push_str("\r\n");
                push_body(&mut out, html);
            }
            MailBody::Both { html, text } => {
                let boundary = format!("rustforge-{}", mail.id);
                push_header(
                    &mut out,
                    "Content-Type",
                    &format!("multipart/alternative; boundary=\"{boundary}\""),
                );
                out.push_str("\r\n");
                out.push_str(&format!("--{boundary}\r\n"));
                out.push_str("Content-Type: text/plain; charset=utf-8\r\n\r\n");
                push_body(&mut out, text);
                out.push_str(&format!("\r\n--{boundary}\r\n"));
                out.push_str("Content-Type: text/html; charset=utf-8\r\n\r\n");
                push_body(&mut out, html);
                out.push_str(&format!("\r\n--{boundary}--\r\n"));
            }
        }

        out
    }
}

#[async_trait]
impl Mailer for FileMailer {
    async fn send(&self, mail: Mail) -> Result<(), MailError> {
        // std::fs is fine here: writes are quick and non-deadlocking. This keeps
        // the exact same delivery path as the synchronous facade.
        self.deliver(&mail).map(|_| ())
    }
}

fn push_header(out: &mut String, name: &str, value: &str) {
    // Collapse CR/LF in header values to guard against header injection.
    let value = value.replace(['\r', '\n'], " ");
    out.push_str(name);
    out.push_str(": ");
    out.push_str(&value);
    out.push_str("\r\n");
}

fn push_body(out: &mut String, body: &str) {
    // Normalize line endings to CRLF as required by RFC 5322.
    for line in body.split('\n') {
        out.push_str(line.trim_end_matches('\r'));
        out.push_str("\r\n");
    }
}

fn format_address(addr: &Address) -> String {
    match &addr.name {
        Some(name) if !name.is_empty() => format!("{} <{}>", name, addr.email),
        _ => addr.email.clone(),
    }
}

fn format_address_list(addrs: &[Address]) -> String {
    addrs
        .iter()
        .map(format_address)
        .collect::<Vec<_>>()
        .join(", ")
}

/// Replace characters that are unsafe in filenames.
fn sanitize(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MailBuilder};

    #[test]
    fn writes_eml_file_with_recipient_subject_body() {
        let dir = std::env::temp_dir().join(format!("rf-file-mailer-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mailer = FileMailer::new(&dir);

        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test Subject")
            .text("Hello, filesystem!")
            .build()
            .unwrap();

        let path = mailer.deliver(&mail).unwrap();
        assert!(path.exists());
        assert_eq!(path.extension().unwrap(), "eml");

        let eml = fs::read_to_string(&path).unwrap();
        assert!(eml.contains("Subject: Test Subject"), "eml:\n{eml}");
        assert!(eml.contains("To: recipient@example.com"), "eml:\n{eml}");
        assert!(eml.contains("From: sender@example.com"), "eml:\n{eml}");
        assert!(eml.contains("Hello, filesystem!"), "eml:\n{eml}");
        assert!(eml.contains("Content-Type: text/plain"), "eml:\n{eml}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn multipart_body_uses_boundary() {
        let dir = std::env::temp_dir().join(format!("rf-file-mailer-mp-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mailer = FileMailer::new(&dir);
        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Multipart")
            .html("<h1>Hi</h1>")
            .text("Hi")
            .build()
            .unwrap();

        let path = mailer.deliver(&mail).unwrap();
        let eml = fs::read_to_string(&path).unwrap();
        assert!(eml.contains("multipart/alternative"), "eml:\n{eml}");
        assert!(eml.contains("text/plain"), "eml:\n{eml}");
        assert!(eml.contains("text/html"), "eml:\n{eml}");
        assert!(eml.contains("<h1>Hi</h1>"), "eml:\n{eml}");

        let _ = fs::remove_dir_all(&dir);
    }

    #[tokio::test]
    async fn async_send_also_delivers() {
        let dir = std::env::temp_dir().join(format!("rf-file-mailer-async-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let mailer = FileMailer::new(&dir);
        let mail = MailBuilder::new()
            .from(Address::new("a@example.com"))
            .to(Address::new("b@example.com"))
            .subject("Async")
            .text("body")
            .build()
            .unwrap();

        mailer.send(mail).await.unwrap();

        let count = fs::read_dir(&dir).unwrap().count();
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(&dir);
    }
}
