//! Component Tag Parser
//!
//! Parses <x-*> component tags with attributes and slots

use super::slots::{Slot, SlotBag};
use regex::Regex;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParseError {
    #[error("Invalid component tag: {0}")]
    InvalidTag(String),

    #[error("Malformed attribute: {0}")]
    MalformedAttribute(String),

    #[error("Regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Represents a parsed component tag
#[derive(Debug, Clone)]
pub struct ComponentTag {
    /// Component name (without x- prefix)
    pub name: String,

    /// Component attributes
    pub attributes: HashMap<String, String>,

    /// Slot bag containing all slots
    pub slots: SlotBag,

    /// Full tag content
    pub raw: String,
}

/// Component tag parser
pub struct ComponentParser {
    // Regex patterns
    component_tag_re: Regex,
    self_closing_re: Regex,
    slot_re: Regex,
    attribute_re: Regex,
}

impl ComponentParser {
    /// Create a new component parser
    pub fn new() -> Result<Self, ParseError> {
        Ok(Self {
            // Match: <x-alert type="danger">content</x-alert>
            component_tag_re: Regex::new(
                r#"(?s)<x-([a-zA-Z0-9._-]+)((?:\s+[^>]*)?)>(.*?)</x-\1>"#
            )?,
            // Match: <x-alert type="danger" />
            self_closing_re: Regex::new(
                r#"<x-([a-zA-Z0-9._-]+)((?:\s+[^>]*)?)\s*/>"#
            )?,
            // Match: <x-slot name="header" class="bold">content</x-slot>
            slot_re: Regex::new(
                r#"(?s)<x-slot(?:\s+name="([^"]+)")?((?:\s+[^>]*)?)>(.*?)</x-slot>"#
            )?,
            // Match attributes: type="danger" or :type="variable"
            attribute_re: Regex::new(
                r#"(?:(\w+)="([^"]*)"|:(\w+)="([^"]*)")"#
            )?,
        })
    }

    /// Parse all component tags in a template
    pub fn parse_all(&self, template: &str) -> Result<Vec<ComponentTag>, ParseError> {
        let mut tags = Vec::new();

        // Parse regular components
        for cap in self.component_tag_re.captures_iter(template) {
            let full_match = cap.get(0).unwrap().as_str();
            let name = cap.get(1).unwrap().as_str();
            let attrs_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let content = cap.get(3).unwrap().as_str();

            let tag = self.parse_component_tag(name, attrs_str, content, full_match)?;
            tags.push(tag);
        }

        // Parse self-closing components
        for cap in self.self_closing_re.captures_iter(template) {
            let full_match = cap.get(0).unwrap().as_str();
            let name = cap.get(1).unwrap().as_str();
            let attrs_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");

            let tag = self.parse_component_tag(name, attrs_str, "", full_match)?;
            tags.push(tag);
        }

        Ok(tags)
    }

    /// Parse a single component tag
    pub fn parse_component_tag(
        &self,
        name: &str,
        attrs_str: &str,
        content: &str,
        raw: &str,
    ) -> Result<ComponentTag, ParseError> {
        let attributes = self.parse_attributes(attrs_str)?;
        let slots = self.parse_slots(content)?;

        Ok(ComponentTag {
            name: name.to_string(),
            attributes,
            slots,
            raw: raw.to_string(),
        })
    }

    /// Parse attributes from attribute string
    pub fn parse_attributes(&self, attrs_str: &str) -> Result<HashMap<String, String>, ParseError> {
        let mut attributes = HashMap::new();

        for cap in self.attribute_re.captures_iter(attrs_str) {
            if let Some(name) = cap.get(1) {
                // Static attribute: type="danger"
                let value = cap.get(2).unwrap().as_str();
                attributes.insert(name.as_str().to_string(), value.to_string());
            } else if let Some(name) = cap.get(3) {
                // Bound attribute: :type="variable"
                let expr = cap.get(4).unwrap().as_str();
                // Store with {{ }} markers for later evaluation
                attributes.insert(
                    name.as_str().to_string(),
                    format!("{{{{ {} }}}}", expr),
                );
            }
        }

        Ok(attributes)
    }

    /// Parse slots from component content
    pub fn parse_slots(&self, content: &str) -> Result<SlotBag, ParseError> {
        let mut bag = SlotBag::new();
        let mut remaining_content = content.to_string();

        // Extract named slots
        for cap in self.slot_re.captures_iter(content) {
            let full_match = cap.get(0).unwrap().as_str();
            let name = cap.get(1).map(|m| m.as_str()).unwrap_or("default");
            let attrs_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            let slot_content = cap.get(3).unwrap().as_str();

            // Parse slot attributes
            let attributes = self.parse_attributes(attrs_str)?;

            // Create slot
            let slot = Slot::with_attributes(name, slot_content.trim(), attributes);
            bag.add_slot(slot);

            // Remove from remaining content
            remaining_content = remaining_content.replace(full_match, "");
        }

        // Remaining content is the default slot
        let default_content = remaining_content.trim();
        if !default_content.is_empty() {
            bag.set_default(default_content);
        }

        Ok(bag)
    }

    /// Find all component tags in template
    pub fn find_component_tags(&self, template: &str) -> Vec<String> {
        let mut tags = Vec::new();

        // Find regular component tags
        for cap in self.component_tag_re.captures_iter(template) {
            tags.push(cap.get(0).unwrap().as_str().to_string());
        }

        // Find self-closing tags
        for cap in self.self_closing_re.captures_iter(template) {
            tags.push(cap.get(0).unwrap().as_str().to_string());
        }

        tags
    }

    /// Check if template contains component tags
    pub fn has_components(&self, template: &str) -> bool {
        self.component_tag_re.is_match(template) || self.self_closing_re.is_match(template)
    }
}

impl Default for ComponentParser {
    fn default() -> Self {
        Self::new().expect("Failed to create component parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_component() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"<x-alert type="danger">Error message</x-alert>"#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 1);

        let tag = &tags[0];
        assert_eq!(tag.name, "alert");
        assert_eq!(tag.attributes.get("type"), Some(&"danger".to_string()));
        assert_eq!(
            tag.slots.default().map(|s| s.content.as_str()),
            Some("Error message")
        );
    }

    #[test]
    fn test_parse_self_closing_component() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"<x-input name="email" />"#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 1);

        let tag = &tags[0];
        assert_eq!(tag.name, "input");
        assert_eq!(tag.attributes.get("name"), Some(&"email".to_string()));
    }

    #[test]
    fn test_parse_bound_attributes() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"<x-alert :type="alertType">Message</x-alert>"#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 1);

        let tag = &tags[0];
        assert_eq!(
            tag.attributes.get("type"),
            Some(&"{{ alertType }}".to_string())
        );
    }

    #[test]
    fn test_parse_named_slots() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"
            <x-card>
                <x-slot name="header">Card Title</x-slot>
                <x-slot name="footer">Card Footer</x-slot>
                Card Body
            </x-card>
        "#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 1);

        let tag = &tags[0];
        assert!(tag.slots.has("header"));
        assert!(tag.slots.has("footer"));
        assert!(tag.slots.has("default"));

        assert_eq!(
            tag.slots.get("header").map(|s| s.content.as_str()),
            Some("Card Title")
        );
        assert_eq!(
            tag.slots.get("footer").map(|s| s.content.as_str()),
            Some("Card Footer")
        );
        assert_eq!(
            tag.slots.default().map(|s| s.content.as_str()),
            Some("Card Body")
        );
    }

    #[test]
    fn test_parse_slot_with_attributes() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"
            <x-card>
                <x-slot name="header" class="font-bold">Title</x-slot>
                Body
            </x-card>
        "#;

        let tags = parser.parse_all(template).unwrap();
        let tag = &tags[0];

        let header = tag.slots.get("header").unwrap();
        assert_eq!(header.get("class"), Some(&"font-bold".to_string()));
    }

    #[test]
    fn test_parse_multiple_components() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"
            <x-alert type="info">Info</x-alert>
            <x-alert type="danger">Error</x-alert>
        "#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_parse_nested_components() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"
            <x-card>
                <x-alert type="info">Nested alert</x-alert>
            </x-card>
        "#;

        let tags = parser.parse_all(template).unwrap();
        // Should find both components
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_has_components() {
        let parser = ComponentParser::new().unwrap();

        assert!(parser.has_components("<x-alert>Test</x-alert>"));
        assert!(parser.has_components("<x-input />"));
        assert!(!parser.has_components("<div>No components</div>"));
    }

    #[test]
    fn test_find_component_tags() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"
            <x-alert>Test</x-alert>
            <x-input />
        "#;

        let tags = parser.find_component_tags(template);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn test_component_with_hyphenated_name() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"<x-my-component>Content</x-my-component>"#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "my-component");
    }

    #[test]
    fn test_component_with_dotted_name() {
        let parser = ComponentParser::new().unwrap();
        let template = r#"<x-form.input>Content</x-form.input>"#;

        let tags = parser.parse_all(template).unwrap();
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "form.input");
    }
}
