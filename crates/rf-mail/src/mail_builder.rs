//! Enhanced mail builder with templating and markdown support

use crate::{Address, Attachment, Mail, MailBody, MailError, TemplateEngine};
use serde::Serialize;
use std::path::Path;

/// Fluent builder for constructing Mail objects
///
/// # Example
///
/// ```
/// use rf_mail::{MailBuilder, Address};
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mail = MailBuilder::new()
///     .from(Address::new("sender@example.com"))
///     .to(Address::new("recipient@example.com"))
///     .subject("Hello!")
///     .text("Hello, World!")
///     .build()?;
/// # Ok(())
/// # }
/// ```
pub struct MailBuilder {
    to: Vec<Address>,
    cc: Vec<Address>,
    bcc: Vec<Address>,
    from: Option<Address>,
    reply_to: Option<Address>,
    subject: String,
    html: Option<String>,
    text: Option<String>,
    markdown: Option<String>,
    attachments: Vec<Attachment>,
    template_engine: Option<TemplateEngine>,
}

impl MailBuilder {
    /// Create a new mail builder
    pub fn new() -> Self {
        Self {
            to: Vec::new(),
            cc: Vec::new(),
            bcc: Vec::new(),
            from: None,
            reply_to: None,
            subject: String::new(),
            html: None,
            text: None,
            markdown: None,
            attachments: Vec::new(),
            template_engine: None,
        }
    }

    /// Set the from address
    pub fn from(mut self, address: impl Into<Address>) -> Self {
        self.from = Some(address.into());
        self
    }

    /// Add a to address
    pub fn to(mut self, address: impl Into<Address>) -> Self {
        self.to.push(address.into());
        self
    }

    /// Add multiple to addresses
    pub fn to_many(mut self, addresses: Vec<Address>) -> Self {
        self.to.extend(addresses);
        self
    }

    /// Add a cc address
    pub fn cc(mut self, address: impl Into<Address>) -> Self {
        self.cc.push(address.into());
        self
    }

    /// Add a bcc address
    pub fn bcc(mut self, address: impl Into<Address>) -> Self {
        self.bcc.push(address.into());
        self
    }

    /// Set the reply-to address
    pub fn reply_to(mut self, address: impl Into<Address>) -> Self {
        self.reply_to = Some(address.into());
        self
    }

    /// Set the subject
    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    /// Set HTML body
    pub fn html(mut self, html: impl Into<String>) -> Self {
        self.html = Some(html.into());
        self
    }

    /// Set plain text body
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// Set markdown body (will be converted to HTML)
    pub fn markdown(mut self, markdown: impl Into<String>) -> Self {
        self.markdown = Some(markdown.into());
        self
    }

    /// Render a template with data
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::MailBuilder;
    /// use serde_json::json;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mail = MailBuilder::new()
    ///     .view("welcome", json!({
    ///         "name": "Alice",
    ///         "url": "https://example.com"
    ///     }))?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn view(mut self, template: &str, data: impl Serialize) -> Result<Self, MailError> {
        // Render against the per-builder engine if one was explicitly supplied
        // via `with_template_engine`; otherwise resolve the template from the
        // process-global shared engine (see `templates::register_template`) so a
        // named template registered at boot is actually found — rather than
        // spinning up a fresh empty engine that can only report "Template not
        // found".
        let rendered = match self.template_engine.as_ref() {
            Some(engine) => engine.render(template, &data)?,
            None => crate::templates::render_global(template, &data)?,
        };

        self.html = Some(rendered);
        Ok(self)
    }

    /// Render a template with a layout
    pub fn view_with_layout(
        mut self,
        template: &str,
        layout: &str,
        data: impl Serialize,
    ) -> Result<Self, MailError> {
        // Resolve both the inner template and the layout the same way `view`
        // does: from a per-builder engine when set, else the process-global
        // shared engine.
        let render = |name: &str, value: &serde_json::Value| -> Result<String, MailError> {
            match self.template_engine.as_ref() {
                Some(engine) => engine.render(name, value),
                None => crate::templates::render_global(name, value),
            }
        };

        // First render the template
        let data_value = serde_json::to_value(&data)?;
        let content = render(template, &data_value)?;

        // Then render the layout with the content
        let mut layout_data = data_value;
        if let Some(obj) = layout_data.as_object_mut() {
            obj.insert("content".to_string(), serde_json::Value::String(content));
        }

        let rendered = render(layout, &layout_data)?;
        self.html = Some(rendered);

        Ok(self)
    }

    /// Render a Tera template using rf-view (requires 'view' feature)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rf_mail::MailBuilder;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mail = MailBuilder::new()
    ///     .tera_view("emails/welcome", json!({
    ///         "name": "Alice",
    ///         "url": "https://example.com"
    ///     }))
    ///     .await?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "view")]
    pub async fn tera_view(
        mut self,
        template: &str,
        data: impl Serialize,
    ) -> Result<Self, MailError> {
        let view = rf_view::View::make(template, data);
        let rendered = view.render().await?;

        self.html = Some(rendered);
        Ok(self)
    }

    /// Render a Tera template with layout using rf-view (requires 'view' feature)
    ///
    /// # Example
    ///
    /// ```ignore
    /// use rf_mail::MailBuilder;
    /// use serde_json::json;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mail = MailBuilder::new()
    ///     .tera_view_with_layout("emails/welcome", "layouts/email", json!({
    ///         "name": "Alice"
    ///     }))
    ///     .await?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "view")]
    pub async fn tera_view_with_layout(
        mut self,
        template: &str,
        layout: &str,
        data: impl Serialize,
    ) -> Result<Self, MailError> {
        let view = rf_view::View::make(template, data).layout(layout);
        let rendered = view.render().await?;

        self.html = Some(rendered);
        Ok(self)
    }

    /// Attach a file from filesystem
    ///
    /// # Example
    ///
    /// ```no_run
    /// use rf_mail::MailBuilder;
    /// use std::path::Path;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mail = MailBuilder::new()
    ///     .attach(Path::new("/path/to/file.pdf"))?
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn attach(mut self, path: impl AsRef<Path>) -> Result<Self, MailError> {
        let attachment = Attachment::from_file(path)?;
        self.attachments.push(attachment);
        Ok(self)
    }

    /// Attach data directly
    ///
    /// # Example
    ///
    /// ```
    /// use rf_mail::MailBuilder;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let data = b"Hello, World!".to_vec();
    /// let mail = MailBuilder::new()
    ///     .attach_data(data, "hello.txt", "text/plain")
    ///     .build()?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn attach_data(mut self, data: Vec<u8>, filename: &str, content_type: &str) -> Self {
        let attachment =
            Attachment::from_data(data, filename.to_string(), content_type.to_string());
        self.attachments.push(attachment);
        self
    }

    /// Set a custom template engine
    pub fn with_template_engine(mut self, engine: TemplateEngine) -> Self {
        self.template_engine = Some(engine);
        self
    }

    /// Build the final Mail object
    pub fn build(self) -> Result<Mail, MailError> {
        // Determine the body type
        let body = match (self.html, self.text, self.markdown) {
            // Markdown takes precedence and gets converted
            (_, _, Some(md)) => {
                let html = crate::markdown::render_markdown(&md)?;
                // Also generate plain text from markdown
                let text = crate::markdown::markdown_to_text(&md);
                MailBody::Both { html, text }
            }
            // Both HTML and text
            (Some(html), Some(text), None) => MailBody::Both { html, text },
            // HTML only
            (Some(html), None, None) => MailBody::Html(html),
            // Text only
            (None, Some(text), None) => MailBody::Text(text),
            // Nothing
            (None, None, None) => {
                return Err(MailError::InvalidMessage(
                    "At least one body type (html, text, or markdown) is required".into(),
                ))
            }
        };

        let mail = Mail {
            id: uuid::Uuid::new_v4().to_string(),
            to: self.to,
            cc: self.cc,
            bcc: self.bcc,
            from: self
                .from
                .ok_or_else(|| MailError::InvalidMessage("From address is required".into()))?,
            reply_to: self.reply_to,
            subject: self.subject,
            body,
            attachments: self.attachments,
        };

        // Validate before returning
        mail.validate().map_err(MailError::InvalidMessage)?;

        Ok(mail)
    }
}

impl Default for MailBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_basic() {
        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert_eq!(mail.from.email, "sender@example.com");
        assert_eq!(mail.to.len(), 1);
        assert_eq!(mail.subject, "Test");
        assert!(mail.has_text());
    }

    #[test]
    fn test_builder_html_and_text() {
        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .html("<h1>Hello</h1>")
            .text("Hello")
            .build()
            .unwrap();

        assert!(matches!(mail.body, MailBody::Both { .. }));
        assert_eq!(mail.html(), Some("<h1>Hello</h1>"));
        assert_eq!(mail.text(), Some("Hello"));
    }

    #[test]
    fn test_builder_validation() {
        let result = MailBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_to_many() {
        let addresses = vec![
            Address::new("user1@example.com"),
            Address::new("user2@example.com"),
        ];

        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to_many(addresses)
            .subject("Test")
            .text("Hello")
            .build()
            .unwrap();

        assert_eq!(mail.to.len(), 2);
    }

    #[test]
    fn test_view_renders_from_shared_global_engine() {
        // A template registered into the process-global shared engine must be
        // resolvable by `MailBuilder::view` — no per-builder engine required.
        // Previously `view` spun up a fresh empty engine and always errored with
        // "Template not found". Use a uniquely-named template so this never
        // collides with other tests sharing the global engine.
        crate::templates::register_template(
            "mail_builder_shared_probe",
            "<h1>Hi, {{name}}!</h1>",
        )
        .expect("register into shared engine");

        let mail = MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new("user@example.com"))
            .subject("Welcome")
            .view("mail_builder_shared_probe", serde_json::json!({ "name": "Bob" }))
            .expect("view must resolve the shared template")
            .build()
            .expect("mail builds");

        assert_eq!(mail.html(), Some("<h1>Hi, Bob!</h1>"));
    }

    #[test]
    fn test_view_with_explicit_engine_still_used() {
        // An engine explicitly supplied via with_template_engine takes precedence
        // over the shared global engine.
        let mut engine = TemplateEngine::new();
        engine
            .register_template("local_only", "local:{{v}}")
            .unwrap();

        let mail = MailBuilder::new()
            .from(Address::new("noreply@example.com"))
            .to(Address::new("user@example.com"))
            .subject("X")
            .with_template_engine(engine)
            .view("local_only", serde_json::json!({ "v": 42 }))
            .expect("explicit engine resolves its own template")
            .build()
            .unwrap();

        assert_eq!(mail.html(), Some("local:42"));
    }

    #[test]
    fn test_builder_attach_data() {
        let data = b"Hello, World!".to_vec();

        let mail = MailBuilder::new()
            .from(Address::new("sender@example.com"))
            .to(Address::new("recipient@example.com"))
            .subject("Test")
            .text("Hello")
            .attach_data(data, "hello.txt", "text/plain")
            .build()
            .unwrap();

        assert_eq!(mail.attachments.len(), 1);
        assert_eq!(mail.attachments[0].filename, "hello.txt");
    }
}
