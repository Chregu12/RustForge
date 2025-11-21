//! Markdown rendering for email templates with components

use crate::MailError;
use pulldown_cmark::{html, Options, Parser};
use regex::Regex;
use std::sync::OnceLock;

/// Render markdown to HTML
///
/// # Example
///
/// ```
/// use rf_mail::markdown::render_markdown;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let html = render_markdown("# Hello\n\nThis is **bold**")?;
/// assert!(html.contains("<h1>"));
/// assert!(html.contains("<strong>"));
/// # Ok(())
/// # }
/// ```
pub fn render_markdown(markdown: &str) -> Result<String, MailError> {
    // First, process custom components
    let processed = process_components(markdown)?;

    // Then convert markdown to HTML
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);

    let parser = Parser::new_ext(&processed, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    Ok(html_output)
}

/// Convert markdown to plain text (strip formatting)
pub fn markdown_to_text(markdown: &str) -> String {
    // Remove markdown formatting
    let text = markdown
        .lines()
        .map(|line| {
            // Remove headers
            let line = line.trim_start_matches('#').trim();
            // Remove bold/italic
            let line = line.replace("**", "").replace("*", "");
            // Remove links [text](url) -> text
            let re = Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap();
            let line = re.replace_all(&line, "$1");
            line.into_owned()
        })
        .collect::<Vec<_>>()
        .join("\n");

    text
}

/// Process custom email components in markdown
///
/// Supports:
/// - @button(url) text @endbutton
/// - @panel text @endpanel
/// - @table ... @endtable (enhanced table rendering)
fn process_components(markdown: &str) -> Result<String, MailError> {
    let mut result = markdown.to_string();

    // Process @button components
    result = process_buttons(&result)?;

    // Process @panel components
    result = process_panels(&result)?;

    // Process @table components
    result = process_tables(&result)?;

    Ok(result)
}

/// Process @button(url) text @endbutton
fn process_buttons(markdown: &str) -> Result<String, MailError> {
    static BUTTON_RE: OnceLock<Regex> = OnceLock::new();
    let re = BUTTON_RE.get_or_init(|| {
        Regex::new(r"@button\(([^\)]+)\)\s*\n?(.*?)\n?@endbutton").unwrap()
    });

    let result = re.replace_all(markdown, |caps: &regex::Captures| {
        let url = &caps[1];
        let text = &caps[2].trim();

        format!(
            r#"<table width="100%" cellpadding="0" cellspacing="0" style="margin: 20px 0;">
    <tr>
        <td align="center">
            <a href="{}" style="display: inline-block; padding: 12px 24px; background-color: #4F46E5; color: white; text-decoration: none; border-radius: 6px; font-weight: 600;">
                {}
            </a>
        </td>
    </tr>
</table>"#,
            url, text
        )
    });

    Ok(result.to_string())
}

/// Process @panel text @endpanel
fn process_panels(markdown: &str) -> Result<String, MailError> {
    static PANEL_RE: OnceLock<Regex> = OnceLock::new();
    let re = PANEL_RE.get_or_init(|| {
        Regex::new(r"@panel\s*\n?(.*?)\n?@endpanel").unwrap()
    });

    let result = re.replace_all(markdown, |caps: &regex::Captures| {
        let content = &caps[1].trim();

        format!(
            r#"<div style="background-color: #F3F4F6; border-left: 4px solid #4F46E5; padding: 16px; margin: 20px 0; border-radius: 4px;">
    {}
</div>"#,
            content
        )
    });

    Ok(result.to_string())
}

/// Process @table ... @endtable (enhanced styling)
fn process_tables(markdown: &str) -> Result<String, MailError> {
    static TABLE_RE: OnceLock<Regex> = OnceLock::new();
    let re = TABLE_RE.get_or_init(|| {
        Regex::new(r"(?s)@table\s*\n(.*?)\n@endtable").unwrap()
    });

    let result = re.replace_all(markdown, |caps: &regex::Captures| {
        let table_content = &caps[1];

        // Parse the markdown table
        let lines: Vec<&str> = table_content.lines().collect();
        if lines.len() < 2 {
            return table_content.to_string();
        }

        // Extract headers
        let headers: Vec<&str> = lines[0]
            .split('|')
            .map(|h| h.trim())
            .filter(|h| !h.is_empty())
            .collect();

        // Skip separator line (line[1])
        // Extract rows
        let rows: Vec<Vec<&str>> = lines
            .iter()
            .skip(2)
            .map(|line| {
                line.split('|')
                    .map(|c| c.trim())
                    .filter(|c| !c.is_empty())
                    .collect()
            })
            .collect();

        // Generate HTML table with styling
        let mut html = String::from(
            r#"<table style="width: 100%; border-collapse: collapse; margin: 20px 0;">
    <thead>
        <tr style="background-color: #F3F4F6;">"#,
        );

        for header in &headers {
            html.push_str(&format!(
                r#"
            <th style="padding: 12px; text-align: left; border-bottom: 2px solid #E5E7EB; font-weight: 600;">{}</th>"#,
                header
            ));
        }

        html.push_str(
            r#"
        </tr>
    </thead>
    <tbody>"#,
        );

        for row in &rows {
            html.push_str(r#"
        <tr style="border-bottom: 1px solid #E5E7EB;">"#);
            for cell in row {
                html.push_str(&format!(
                    r#"
            <td style="padding: 12px;">{}</td>"#,
                    cell
                ));
            }
            html.push_str(
                r#"
        </tr>"#,
            );
        }

        html.push_str(
            r#"
    </tbody>
</table>"#,
        );

        html
    });

    Ok(result.to_string())
}

/// Helper functions for creating components programmatically

/// Create a button component
pub fn button(text: &str, url: &str) -> String {
    format!("@button({})\n{}\n@endbutton", url, text)
}

/// Create a panel component
pub fn panel(content: &str) -> String {
    format!("@panel\n{}\n@endpanel", content)
}

/// Create a table component
pub fn table(headers: Vec<&str>, rows: Vec<Vec<&str>>) -> String {
    let mut md = String::from("@table\n");

    // Headers
    md.push_str("| ");
    md.push_str(&headers.join(" | "));
    md.push_str(" |\n");

    // Separator
    md.push_str("|");
    for _ in &headers {
        md.push_str("---|");
    }
    md.push('\n');

    // Rows
    for row in rows {
        md.push_str("| ");
        md.push_str(&row.join(" | "));
        md.push_str(" |\n");
    }

    md.push_str("@endtable");
    md
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_markdown() {
        let md = "# Hello\n\nThis is **bold** text.";
        let html = render_markdown(md).unwrap();

        assert!(html.contains("<h1>"));
        assert!(html.contains("<strong>"));
    }

    #[test]
    fn test_markdown_to_text() {
        let md = "# Hello\n\n**Bold** and *italic* text.";
        let text = markdown_to_text(md);

        assert!(!text.contains('#'));
        assert!(!text.contains('*'));
        assert!(text.contains("Hello"));
        assert!(text.contains("Bold"));
    }

    #[test]
    fn test_button_component() {
        let md = "@button(https://example.com)\nClick Me\n@endbutton";
        let result = process_buttons(md).unwrap();

        assert!(result.contains("href=\"https://example.com\""));
        assert!(result.contains("Click Me"));
        assert!(result.contains("<a"));
    }

    #[test]
    fn test_panel_component() {
        let md = "@panel\nImportant message\n@endpanel";
        let result = process_panels(md).unwrap();

        assert!(result.contains("Important message"));
        assert!(result.contains("<div"));
        assert!(result.contains("background-color"));
    }

    #[test]
    fn test_table_component() {
        let md = r#"@table
| Name | Age | City |
|------|-----|------|
| Alice | 30 | NYC |
| Bob | 25 | LA |
@endtable"#;

        let result = process_components(md).unwrap();

        assert!(result.contains("<table"));
        assert!(result.contains("Name"));
        assert!(result.contains("Alice"));
        assert!(result.contains("NYC"));
    }

    #[test]
    fn test_button_helper() {
        let component = button("Click Here", "https://example.com");
        assert!(component.contains("@button(https://example.com)"));
        assert!(component.contains("Click Here"));
    }

    #[test]
    fn test_table_helper() {
        let component = table(
            vec!["Name", "Age"],
            vec![vec!["Alice", "30"], vec!["Bob", "25"]],
        );
        assert!(component.contains("@table"));
        assert!(component.contains("Name"));
        assert!(component.contains("Alice"));
    }
}
