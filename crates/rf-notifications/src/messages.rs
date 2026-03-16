//! Message types for different notification channels

use serde::{Deserialize, Serialize};

/// Laravel-style mail message with greeting, lines, and action button
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailMessage {
    pub subject: String,
    pub greeting: Option<String>,
    pub lines: Vec<String>,
    pub action: Option<MailAction>,
    pub markdown: bool,
    pub from: Option<String>,
    pub to: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MailAction {
    pub text: String,
    pub url: String,
}

impl MailMessage {
    pub fn new() -> Self {
        Self {
            subject: String::new(),
            greeting: None,
            lines: Vec::new(),
            action: None,
            markdown: false,
            from: None,
            to: Vec::new(),
        }
    }

    pub fn subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = subject.into();
        self
    }

    pub fn greeting(mut self, greeting: impl Into<String>) -> Self {
        self.greeting = Some(greeting.into());
        self
    }

    pub fn line(mut self, line: impl Into<String>) -> Self {
        self.lines.push(line.into());
        self
    }

    pub fn action(mut self, text: impl Into<String>, url: impl Into<String>) -> Self {
        self.action = Some(MailAction {
            text: text.into(),
            url: url.into(),
        });
        self
    }

    pub fn markdown(mut self, enabled: bool) -> Self {
        self.markdown = enabled;
        self
    }

    pub fn from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    pub fn to(mut self, to: impl Into<String>) -> Self {
        self.to.push(to.into());
        self
    }

    /// Render to plain text
    pub fn to_text(&self) -> String {
        let mut text = String::new();

        if let Some(greeting) = &self.greeting {
            text.push_str(greeting);
            text.push_str("\n\n");
        }

        for line in &self.lines {
            text.push_str(line);
            text.push_str("\n\n");
        }

        if let Some(action) = &self.action {
            text.push_str(&format!("{}: {}\n\n", action.text, action.url));
        }

        text
    }

    /// Render to HTML
    pub fn to_html(&self) -> String {
        fn escape_html(s: &str) -> String {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
                .replace('"', "&quot;")
                .replace('\'', "&#x27;")
        }

        let mut html = String::from("<html><body>");

        if let Some(greeting) = &self.greeting {
            html.push_str(&format!("<h1>{}</h1>", escape_html(greeting)));
        }

        for line in &self.lines {
            html.push_str(&format!("<p>{}</p>", escape_html(line)));
        }

        if let Some(action) = &self.action {
            html.push_str(&format!(
                r#"<p><a href="{}" style="background-color: #3490dc; color: white; padding: 10px 20px; text-decoration: none; border-radius: 5px;">{}</a></p>"#,
                escape_html(&action.url), escape_html(&action.text)
            ));
        }

        html.push_str("</body></html>");
        html
    }
}

impl Default for MailMessage {
    fn default() -> Self {
        Self::new()
    }
}

/// Database notification data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseNotification {
    pub title: String,
    pub message: String,
    pub data: serde_json::Value,
}

impl DatabaseNotification {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            data: serde_json::Value::Null,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    pub fn data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }
}

impl Default for DatabaseNotification {
    fn default() -> Self {
        Self::new()
    }
}

/// SMS message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmsMessage {
    pub content: String,
}

impl SmsMessage {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
        }
    }
}

/// Slack message with attachments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackMessage {
    pub text: String,
    pub attachments: Vec<SlackAttachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackAttachment {
    pub title: Option<String>,
    pub text: String,
    pub color: String,
    pub fields: Vec<SlackField>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackField {
    pub title: String,
    pub value: String,
    pub short: bool,
}

impl SlackMessage {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            attachments: Vec::new(),
        }
    }

    pub fn attachment(mut self, attachment: SlackAttachment) -> Self {
        self.attachments.push(attachment);
        self
    }
}

impl SlackAttachment {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            title: None,
            text: text.into(),
            color: "good".to_string(),
            fields: Vec::new(),
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn color(mut self, color: impl Into<String>) -> Self {
        self.color = color.into();
        self
    }

    pub fn field(
        mut self,
        title: impl Into<String>,
        value: impl Into<String>,
        short: bool,
    ) -> Self {
        self.fields.push(SlackField {
            title: title.into(),
            value: value.into(),
            short,
        });
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mail_message_builder() {
        let msg = MailMessage::new()
            .subject("Test")
            .greeting("Hello!")
            .line("Line 1")
            .line("Line 2")
            .action("Click Here", "https://example.com");

        assert_eq!(msg.subject, "Test");
        assert_eq!(msg.greeting, Some("Hello!".to_string()));
        assert_eq!(msg.lines.len(), 2);
        assert!(msg.action.is_some());
    }

    #[test]
    fn test_mail_message_to_text() {
        let msg = MailMessage::new()
            .greeting("Hello!")
            .line("Line 1")
            .action("Click", "https://example.com");

        let text = msg.to_text();
        assert!(text.contains("Hello!"));
        assert!(text.contains("Line 1"));
        assert!(text.contains("Click"));
    }

    #[test]
    fn test_mail_message_to_html() {
        let msg = MailMessage::new().greeting("Hello!").line("Line 1");

        let html = msg.to_html();
        assert!(html.contains("<h1>Hello!</h1>"));
        assert!(html.contains("<p>Line 1</p>"));
    }

    #[test]
    fn test_database_notification() {
        let notification = DatabaseNotification::new()
            .title("Test")
            .message("Test message")
            .data(serde_json::json!({"key": "value"}));

        assert_eq!(notification.title, "Test");
        assert_eq!(notification.message, "Test message");
    }

    #[test]
    fn test_sms_message() {
        let msg = SmsMessage::new("Hello via SMS");
        assert_eq!(msg.content, "Hello via SMS");
    }

    #[test]
    fn test_slack_message() {
        let msg = SlackMessage::new("Hello Slack").attachment(
            SlackAttachment::new("Attachment text")
                .title("Title")
                .color("good")
                .field("Field 1", "Value 1", true),
        );

        assert_eq!(msg.text, "Hello Slack");
        assert_eq!(msg.attachments.len(), 1);
        assert_eq!(msg.attachments[0].fields.len(), 1);
    }
}
