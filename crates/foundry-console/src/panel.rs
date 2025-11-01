#[derive(Debug, Clone, Copy)]
pub enum PanelStyle {
    Single,
    Double,
    Rounded,
    Bold,
}

impl PanelStyle {
    fn chars(&self) -> PanelChars {
        match self {
            PanelStyle::Single => PanelChars {
                top_left: '┌',
                top_right: '┐',
                bottom_left: '└',
                bottom_right: '┘',
                horizontal: '─',
                vertical: '│',
            },
            PanelStyle::Double => PanelChars {
                top_left: '╔',
                top_right: '╗',
                bottom_left: '╚',
                bottom_right: '╝',
                horizontal: '═',
                vertical: '║',
            },
            PanelStyle::Rounded => PanelChars {
                top_left: '╭',
                top_right: '╮',
                bottom_left: '╰',
                bottom_right: '╯',
                horizontal: '─',
                vertical: '│',
            },
            PanelStyle::Bold => PanelChars {
                top_left: '┏',
                top_right: '┓',
                bottom_left: '┗',
                bottom_right: '┛',
                horizontal: '━',
                vertical: '┃',
            },
        }
    }
}

struct PanelChars {
    top_left: char,
    top_right: char,
    bottom_left: char,
    bottom_right: char,
    horizontal: char,
    vertical: char,
}

pub struct Panel {
    title: Option<String>,
    content: Vec<String>,
    style: PanelStyle,
    width: Option<usize>,
    padding: usize,
}

impl Panel {
    pub fn new(content: impl Into<String>) -> Self {
        Self {
            title: None,
            content: vec![content.into()],
            style: PanelStyle::Single,
            width: None,
            padding: 1,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_style(mut self, style: PanelStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_padding(mut self, padding: usize) -> Self {
        self.padding = padding;
        self
    }

    pub fn add_line(&mut self, line: impl Into<String>) {
        self.content.push(line.into());
    }

    pub fn render(&self) -> String {
        let chars = self.style.chars();

        // Calculate the width
        let content_width = self.calculate_width();
        let inner_width = content_width + (self.padding * 2);

        let mut output = Vec::new();

        // Top border with optional title
        if let Some(title) = &self.title {
            let title_stripped = strip_ansi(title);
            let title_len = title_stripped.len();
            let border_len = inner_width.saturating_sub(title_len + 2);
            let left_border = border_len / 2;
            let right_border = border_len - left_border;

            output.push(format!(
                "{}{}{}{}{}{}",
                chars.top_left,
                chars.horizontal.to_string().repeat(left_border),
                " ",
                title,
                " ",
                chars.horizontal.to_string().repeat(right_border),
            ));

            // Add closing character
            let last_line = output.last_mut().unwrap();
            last_line.push(chars.top_right);
        } else {
            output.push(format!(
                "{}{}{}",
                chars.top_left,
                chars.horizontal.to_string().repeat(inner_width),
                chars.top_right
            ));
        }

        // Content lines
        for line in &self.content {
            let line_stripped = strip_ansi(line);
            let line_len = line_stripped.len();

            let padding_left = " ".repeat(self.padding);
            let padding_right = " ".repeat(content_width - line_len + self.padding);

            output.push(format!(
                "{}{}{}{}{}",
                chars.vertical,
                padding_left,
                line,
                padding_right,
                chars.vertical
            ));
        }

        // Bottom border
        output.push(format!(
            "{}{}{}",
            chars.bottom_left,
            chars.horizontal.to_string().repeat(inner_width),
            chars.bottom_right
        ));

        output.join("\n")
    }

    pub fn print(&self) {
        println!("{}", self.render());
    }

    fn calculate_width(&self) -> usize {
        if let Some(width) = self.width {
            width
        } else {
            let content_max = self
                .content
                .iter()
                .map(|line| strip_ansi(line).len())
                .max()
                .unwrap_or(0);

            let title_len = self
                .title
                .as_ref()
                .map(|t| strip_ansi(t).len() + 2)
                .unwrap_or(0);

            content_max.max(title_len)
        }
    }
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::new();
    let mut chars = s.chars();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip until 'm'
            for c in chars.by_ref() {
                if c == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_panel_creation() {
        let panel = Panel::new("Test content");
        assert_eq!(panel.content.len(), 1);
    }

    #[test]
    fn test_panel_with_title() {
        let panel = Panel::new("Content").with_title("Title");
        assert!(panel.title.is_some());
    }
}
