#[derive(Debug, Clone, Copy)]
pub enum ListStyle {
    Bullet,
    Numbered,
    Dash,
    Arrow,
    Check,
}

impl ListStyle {
    fn marker(&self, index: usize) -> String {
        match self {
            ListStyle::Bullet => "•".to_string(),
            ListStyle::Numbered => format!("{}.", index + 1),
            ListStyle::Dash => "-".to_string(),
            ListStyle::Arrow => "→".to_string(),
            ListStyle::Check => "✓".to_string(),
        }
    }
}

pub struct List {
    items: Vec<String>,
    style: ListStyle,
    indent: usize,
}

impl List {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            style: ListStyle::Bullet,
            indent: 2,
        }
    }

    pub fn with_style(mut self, style: ListStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_indent(mut self, indent: usize) -> Self {
        self.indent = indent;
        self
    }

    pub fn add(&mut self, item: impl Into<String>) {
        self.items.push(item.into());
    }

    pub fn add_items(&mut self, items: Vec<String>) {
        self.items.extend(items);
    }

    pub fn render(&self) -> String {
        let mut output = Vec::new();

        for (idx, item) in self.items.iter().enumerate() {
            let marker = self.style.marker(idx);
            let indent = " ".repeat(self.indent);
            output.push(format!("{}{} {}", indent, marker, item));
        }

        output.join("\n")
    }

    pub fn print(&self) {
        println!("{}", self.render());
    }
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_creation() {
        let mut list = List::new();
        list.add("Item 1");
        list.add("Item 2");

        assert_eq!(list.items.len(), 2);
    }

    #[test]
    fn test_list_styles() {
        assert!(ListStyle::Bullet.marker(0).contains("•"));
        assert!(ListStyle::Numbered.marker(0).contains("1"));
        assert!(ListStyle::Dash.marker(0).contains("-"));
    }
}
