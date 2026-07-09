//! # rf-mail-facade
//!
//! Laravel-style Mail facade for RustForge
//!
//! ## Features
//!
//! - **Static Mail API**: Use `Mail::send()`, `Mail::to()`, etc. - no `.await` needed!
//! - **Global Mailer**: Thread-safe global mail state
//! - **Laravel-Compatible**: Familiar API for Laravel developers
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_mail::facade::Mail;
//!
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Send mail to specific address
//! let mailer = Mail::to("user@example.com");
//! # Ok(())
//! # }
//! ```

use once_cell::sync::Lazy;
use crate::bridge::BridgedSmtpMailer;
use crate::{Address, FileMailer, Mail as MailMessage, MailBuilder, MemoryMailer, MailResult, Mailable, SmtpConfig};
use std::path::PathBuf;
use std::sync::RwLock;

/// Global in-memory mailer instance (kept for backwards-compatible inspection).
///
/// Uses `std::sync::RwLock` for synchronous access (no `.await` needed).
pub static GLOBAL_MAILER: Lazy<RwLock<MemoryMailer>> = Lazy::new(|| {
    RwLock::new(MemoryMailer::new())
});

/// Default mailbox directory used by the file transport when `MAIL_MAILBOX`
/// is not set.
fn default_mailbox() -> PathBuf {
    if let Ok(dir) = std::env::var("MAIL_MAILBOX") {
        PathBuf::from(dir)
    } else {
        std::env::temp_dir().join("rustforge-mailbox")
    }
}

/// Global filesystem transport backing the synchronous `Mail` facade.
///
/// This is the real default transport: `Mail::to(..).send(..)` writes each
/// message to disk as an `.eml` file. Delivery is fully synchronous, so it works
/// with or without a Tokio runtime.
pub static GLOBAL_FILE_MAILER: Lazy<RwLock<FileMailer>> =
    Lazy::new(|| RwLock::new(FileMailer::new(default_mailbox())));

/// Optional real SMTP transport backing the synchronous `Mail` facade.
///
/// `None` by default — the facade delivers via [`GLOBAL_FILE_MAILER`] until an
/// SMTP transport is configured with [`Mail::smtp`]. When set, every subsequent
/// `Mail::to(..).send(..)` is delivered over real SMTP (lettre's async transport)
/// through the deadlock-safe [`crate::bridge::AsyncBridge`], so the sync facade
/// works with or without an ambient Tokio runtime.
pub static GLOBAL_SMTP_MAILER: Lazy<RwLock<Option<BridgedSmtpMailer>>> =
    Lazy::new(|| RwLock::new(None));

/// Default `From` address used by the convenience one-liners ([`Mail::raw`],
/// [`Mail::to`]`(..).subject(..).text(..).send()`) when the caller does not set
/// one explicitly. Initialized from the `MAIL_FROM` environment variable, falling
/// back to `noreply@rustforge.local`, and reconfigurable at runtime via
/// [`Mail::from`].
pub static GLOBAL_MAIL_FROM: Lazy<RwLock<String>> = Lazy::new(|| {
    RwLock::new(
        std::env::var("MAIL_FROM").unwrap_or_else(|_| "noreply@rustforge.local".to_string()),
    )
});

/// Read the current default sender address.
fn default_from() -> String {
    GLOBAL_MAIL_FROM
        .read()
        .expect("rf-mail default-from lock poisoned")
        .clone()
}

pub struct Mail;

impl Mail {
    /// Send a mailable
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::Mail;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Mail::send(my_mailable)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send<M: Mailable>(mailable: M) -> MailResult<()> {
        deliver(mailable, None)
    }

    /// Create a mailer for a specific recipient
    ///
    /// # Examples
    ///
    /// ```rust
    /// use rf_mail::facade::Mail;
    ///
    /// let mailer = Mail::to("user@example.com");
    /// ```
    pub fn to(address: impl Into<String>) -> Mailer {
        Mailer::new(address.into())
    }

    /// Send a one-off plain-text email in a single call — no `Mailable` struct to
    /// define.
    ///
    /// This is the ergonomic counterpart to the [`Mailable`] trait path for a
    /// trivial text notification. It builds a real [`Mail`](crate::Mail) with the
    /// configured default sender (see [`Mail::from`]) and delivers it through the
    /// exact same transport as every other facade send (fake ➜ SMTP ➜ the default
    /// `.eml`-on-disk [`FileMailer`]).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_mail::facade::Mail;
    ///
    /// # fn example() -> rf_mail::MailResult<()> {
    /// Mail::raw("user@example.com", "Upload complete", "Your file finished processing.")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn raw(
        to: impl Into<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> MailResult<()> {
        Mailer::new(to.into()).subject(subject).text(body).send()
    }

    /// Configure the default `From` address used by the convenience one-liners
    /// ([`Mail::raw`] and the [`Mail::to`]`(..).subject(..).text(..).send()`
    /// builder) when a message does not set its own sender.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_mail::facade::Mail;
    ///
    /// Mail::from("hello@myapp.test");
    /// ```
    pub fn from(address: impl Into<String>) {
        *GLOBAL_MAIL_FROM
            .write()
            .expect("rf-mail default-from lock poisoned") = address.into();
    }

    /// Configure the mailbox directory used by the default file transport.
    ///
    /// Every subsequent `Mail::to(..).send(..)` writes `.eml` files here.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_mail::facade::Mail;
    ///
    /// Mail::mailbox("/var/mail/rustforge");
    /// ```
    pub fn mailbox(dir: impl Into<PathBuf>) {
        *GLOBAL_FILE_MAILER.write().unwrap() = FileMailer::new(dir.into());
    }

    /// Get the current mailbox directory used by the default file transport.
    pub fn mailbox_path() -> PathBuf {
        GLOBAL_FILE_MAILER.read().unwrap().mailbox().to_path_buf()
    }

    /// Route the synchronous facade through a **real SMTP transport** (lettre's
    /// async `AsyncSmtpTransport`), driven behind a deadlock-safe bridge.
    ///
    /// After this call every `Mail::to(..).send(..)` connects to the configured
    /// SMTP server and delivers over the wire. Building the transport is cheap and
    /// offline — lettre connects lazily on the first actual send — so this returns
    /// `Ok` even without a reachable server; a *live* SMTP server is only required
    /// to complete an end-to-end delivery. Call [`Mail::use_file_transport`] to
    /// revert to the default `.eml`-on-disk transport.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_mail::{facade::Mail, SmtpConfig};
    ///
    /// # fn example() -> rf_mail::MailResult<()> {
    /// Mail::smtp(SmtpConfig {
    ///     host: "smtp.example.com".into(),
    ///     port: 587,
    ///     username: "user".into(),
    ///     password: "secret".into(),
    ///     from_address: "noreply@example.com".into(),
    ///     from_name: Some("MyApp".into()),
    /// })?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn smtp(config: SmtpConfig) -> MailResult<()> {
        let mailer = BridgedSmtpMailer::connect_smtp(config)?;
        *GLOBAL_SMTP_MAILER.write().unwrap() = Some(mailer);
        Ok(())
    }

    /// Revert the synchronous facade to the default filesystem transport,
    /// tearing down any SMTP transport previously set with [`Mail::smtp`].
    pub fn use_file_transport() {
        *GLOBAL_SMTP_MAILER.write().unwrap() = None;
    }
}

/// Build a mailable synchronously and deliver it through the real default
/// transport.
///
/// When mail faking is enabled (see [`crate::testing::fake`]), the message is
/// recorded there instead of being written to disk, mirroring Laravel's
/// `Mail::fake()`. Otherwise the message is written as an `.eml` file via the
/// global [`FileMailer`]. Delivery is synchronous and never blocks on an async
/// runtime.
fn deliver<M: Mailable>(mailable: M, to_override: Option<&str>) -> MailResult<()> {
    let mut mail = mailable.build().build()?;

    // `Mail::to(addr)` acts as a recipient override only when the mailable did
    // not specify its own recipients, preserving prior facade behavior.
    if let Some(addr) = to_override {
        if mail.to.is_empty() {
            mail.to.push(crate::Address::new(addr));
        }
    }

    deliver_mail(mail)
}

/// Route an already-built [`Mail`](crate::Mail) through the real facade transport
/// chain: mail-fake recorder (if enabled) ➜ configured SMTP ➜ the default
/// `.eml`-on-disk [`FileMailer`]. Shared by the [`Mailable`] path ([`deliver`])
/// and the convenience one-liners ([`Mail::raw`] / [`DraftMail::send`]).
fn deliver_mail(mail: MailMessage) -> MailResult<()> {
    if let Some(fake) = crate::testing::get_fake() {
        fake.record(mail);
        return Ok(());
    }

    // Prefer a configured real SMTP transport (delivered over the deadlock-safe
    // bridge), falling back to the default `.eml`-on-disk file transport.
    if let Some(smtp) = GLOBAL_SMTP_MAILER.read().unwrap().as_ref() {
        return smtp.deliver(mail);
    }

    let mailer = GLOBAL_FILE_MAILER.read().unwrap();
    mailer.deliver(&mail)?;
    Ok(())
}

pub struct Mailer {
    pub to: String,
}

impl Mailer {
    pub fn new(to: String) -> Self {
        Self { to }
    }

    /// Send a mailable to this recipient
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use crate::Mail;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// // Mail::to("user@example.com").send(my_mailable)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn send<M: Mailable>(self, mailable: M) -> MailResult<()> {
        deliver(mailable, Some(&self.to))
    }

    /// Begin a fluent one-off text/HTML message to this recipient by setting its
    /// subject. Returns a [`DraftMail`] whose `.text(..)` / `.html(..)` set the
    /// body and whose `.send()` delivers it — no `Mailable` struct required:
    ///
    /// ```rust,no_run
    /// use rf_mail::facade::Mail;
    ///
    /// # fn example() -> rf_mail::MailResult<()> {
    /// Mail::to("user@example.com")
    ///     .subject("Upload complete")
    ///     .text("Your file finished processing.")
    ///     .send()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn subject(self, subject: impl Into<String>) -> DraftMail {
        DraftMail::new(self.to).subject(subject)
    }

    /// Begin a fluent one-off message to this recipient by setting a plain-text
    /// body (see [`Mailer::subject`]).
    pub fn text(self, body: impl Into<String>) -> DraftMail {
        DraftMail::new(self.to).text(body)
    }

    /// Begin a fluent one-off message to this recipient by setting an HTML body
    /// (see [`Mailer::subject`]).
    pub fn html(self, body: impl Into<String>) -> DraftMail {
        DraftMail::new(self.to).html(body)
    }
}

/// A one-off message under construction, produced by [`Mail::to`]`(..).subject(..)`
/// (or `.text(..)`/`.html(..)`). Accumulates subject/body/sender and delivers
/// through the real facade transport on [`send`](DraftMail::send) — the
/// no-`Mailable` convenience path.
pub struct DraftMail {
    to: String,
    from: Option<String>,
    subject: String,
    text: Option<String>,
    html: Option<String>,
}

impl DraftMail {
    fn new(to: String) -> Self {
        Self {
            to,
            from: None,
            subject: String::new(),
            text: None,
            html: None,
        }
    }

    /// Set the subject line.
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set the plain-text body.
    pub fn text(mut self, body: impl Into<String>) -> Self {
        self.text = Some(body.into());
        self
    }

    /// Set the HTML body.
    pub fn html(mut self, body: impl Into<String>) -> Self {
        self.html = Some(body.into());
        self
    }

    /// Override the sender for this message (defaults to [`Mail::from`]).
    pub fn from(mut self, address: impl Into<String>) -> Self {
        self.from = Some(address.into());
        self
    }

    /// Build the real [`Mail`](crate::Mail) and deliver it through the facade's
    /// transport chain (fake ➜ SMTP ➜ default file transport).
    pub fn send(self) -> MailResult<()> {
        let from = self.from.unwrap_or_else(default_from);

        let has_html = self.html.is_some();
        let mut builder = MailBuilder::new()
            .from(Address::new(from))
            .to(Address::new(self.to))
            .subject(self.subject);

        if let Some(html) = self.html {
            builder = builder.html(html);
        }
        // Default to an empty text body when neither body was supplied so a bare
        // `Mail::to(x).subject(y).send()` still produces a valid message rather
        // than failing MailBuilder's "at least one body required" validation.
        if let Some(text) = self.text {
            builder = builder.text(text);
        } else if !has_html {
            builder = builder.text(String::new());
        }

        deliver_mail(builder.build()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Address, MailBody, MailBuilder};

    /// A minimal real `Mailable` used to exercise the facade's build path.
    struct SmokeMail {
        to: String,
        subject: String,
        body: String,
    }

    impl Mailable for SmokeMail {
        fn build(&self) -> MailBuilder {
            MailBuilder::new()
                .from(Address::new("noreply@rustforge.test"))
                .to(Address::new(&self.to))
                .subject(&self.subject)
                .text(&self.body)
        }
    }

    #[test]
    fn test_mail_to() {
        let mailer = Mail::to("test@example.com");
        assert_eq!(mailer.to, "test@example.com");
    }

    #[test]
    fn test_facade_mailable_builds_expected_fields() {
        // The facade delivers via `mailable.build().build()`; assert that path
        // yields a Mail carrying exactly the fields the builder was given.
        let mail = SmokeMail {
            to: "user@example.com".into(),
            subject: "Hello".into(),
            body: "Body text".into(),
        }
        .build()
        .build()
        .expect("mailable should build a valid Mail");

        assert_eq!(mail.to.len(), 1);
        assert_eq!(mail.to[0].email, "user@example.com");
        assert_eq!(mail.from.email, "noreply@rustforge.test");
        assert_eq!(mail.subject, "Hello");
        match &mail.body {
            MailBody::Text(text) => assert_eq!(text, "Body text"),
            other => panic!("expected text body, got {other:?}"),
        }
    }

    #[test]
    fn test_facade_html_and_text_produce_multipart_body() {
        // Supplying both html and text should yield a multipart Both body.
        let mail = MailBuilder::new()
            .from(Address::new("noreply@rustforge.test"))
            .to(Address::new("user@example.com"))
            .subject("Multipart")
            .html("<p>Hi</p>")
            .text("Hi")
            .build()
            .expect("builder with html+text should succeed");

        match &mail.body {
            MailBody::Both { html, text } => {
                assert_eq!(html, "<p>Hi</p>");
                assert_eq!(text, "Hi");
            }
            other => panic!("expected multipart body, got {other:?}"),
        }
    }

    #[test]
    fn test_facade_mailbox_configuration_roundtrip() {
        // `Mail::mailbox` reconfigures the default file transport; the path must
        // round-trip through `Mail::mailbox_path`.
        let dir = std::env::temp_dir().join(format!("rf-mail-facade-box-{}", uuid::Uuid::new_v4()));
        Mail::mailbox(&dir);
        assert_eq!(Mail::mailbox_path(), dir);
    }

    #[test]
    fn test_raw_and_builder_one_liners_build_and_deliver() {
        // `Mail::raw` and the `Mail::to(..).subject(..).text(..).send()` builder
        // must build a real Mail (correct sender/recipient/subject/body) and route
        // it through the SAME facade transport chain as the Mailable path — with no
        // Mailable struct defined. The mail fake records the actual built Mail, so
        // asserting through it proves the message content without racing the
        // process-global file-transport dir. (The real `.eml`-on-disk proof lives
        // in the single-threaded sandbox probe.) Run sequentially in one test: the
        // fake recorder is process-global, so parallel `#[test]`s would race on it.
        crate::testing::fake();
        Mail::from("sender@rustforge.test");

        // (1) The `raw` one-liner.
        Mail::raw("recipient@example.com", "Upload complete", "Your file is ready.")
            .expect("Mail::raw should deliver a message");
        crate::testing::assert_sent(|m| {
            m.subject == "Upload complete"
                && m.from.email == "sender@rustforge.test"
                && m.to.iter().any(|a| a.email == "recipient@example.com")
                && m.text() == Some("Your file is ready.")
        });

        // (2) The fluent builder, with a per-message sender override.
        Mail::to("builder@example.com")
            .subject("Hi there")
            .text("builder body")
            .from("other@rustforge.test")
            .send()
            .expect("builder send should succeed");
        crate::testing::assert_sent(|m| {
            m.subject == "Hi there"
                && m.from.email == "other@rustforge.test"
                && m.to.iter().any(|a| a.email == "builder@example.com")
                && m.text() == Some("builder body")
        });

        crate::testing::restore();
    }

    #[test]
    fn test_facade_file_transport_writes_eml() {
        // The default facade transport is the real on-disk FileMailer. Deliver a
        // builder-produced Mail through it and assert a well-formed .eml lands on
        // disk. Uses a unique temp dir so it never touches the repo and never
        // races the process-global facade state.
        let dir = std::env::temp_dir().join(format!("rf-mail-facade-eml-{}", uuid::Uuid::new_v4()));
        let mailer = FileMailer::new(&dir);

        let mail = MailBuilder::new()
            .from(Address::new("noreply@rustforge.test"))
            .to(Address::new("recipient@example.com"))
            .subject("Smoke Subject")
            .text("smoke body")
            .build()
            .expect("mail should build");

        let path = mailer.deliver(&mail).expect("delivery should write a file");
        assert!(path.exists(), "an .eml file should be written to disk");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("eml"));

        let eml = std::fs::read_to_string(&path).expect("written .eml should be readable");
        assert!(eml.contains("Subject: Smoke Subject"), "eml missing subject: {eml}");
        assert!(eml.contains("recipient@example.com"), "eml missing recipient: {eml}");
        assert!(eml.contains("smoke body"), "eml missing body: {eml}");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
