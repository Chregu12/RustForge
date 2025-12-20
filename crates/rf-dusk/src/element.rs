//! Element interactions

use crate::{DuskError, DuskResult};
use serde::{Deserialize, Serialize};

/// Represents a DOM element
#[derive(Debug, Clone)]
pub struct Element {
    pub selector: String,
    pub tag_name: String,
    pub text: Option<String>,
    pub value: Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
}

impl Element {
    /// Create a new element
    pub fn new(selector: impl Into<String>) -> Self {
        Self {
            selector: selector.into(),
            tag_name: String::new(),
            text: None,
            value: None,
            attributes: std::collections::HashMap::new(),
        }
    }

    /// Get element attribute
    pub fn attr(&self, name: &str) -> Option<&String> {
        self.attributes.get(name)
    }

    /// Check if element has a class
    pub fn has_class(&self, class: &str) -> bool {
        self.attributes
            .get("class")
            .map(|classes| classes.split_whitespace().any(|c| c == class))
            .unwrap_or(false)
    }

    /// Check if element is a specific tag
    pub fn is_tag(&self, tag: &str) -> bool {
        self.tag_name.eq_ignore_ascii_case(tag)
    }
}

/// Element assertion builder
pub struct ElementAssertion {
    pub element: Element,
}

impl ElementAssertion {
    pub fn new(element: Element) -> Self {
        Self { element }
    }

    /// Assert element has text
    pub fn has_text(&self, expected: &str) -> DuskResult<&Self> {
        let text = self.element.text.as_deref().unwrap_or("");
        if !text.contains(expected) {
            return Err(DuskError::AssertionFailed(format!(
                "Element text '{}' does not contain '{}'",
                text, expected
            )));
        }
        Ok(self)
    }

    /// Assert element has value
    pub fn has_value(&self, expected: &str) -> DuskResult<&Self> {
        let value = self.element.value.as_deref().unwrap_or("");
        if value != expected {
            return Err(DuskError::AssertionFailed(format!(
                "Element value '{}' does not match '{}'",
                value, expected
            )));
        }
        Ok(self)
    }

    /// Assert element has attribute
    pub fn has_attribute(&self, name: &str) -> DuskResult<&Self> {
        if !self.element.attributes.contains_key(name) {
            return Err(DuskError::AssertionFailed(format!(
                "Element does not have attribute '{}'",
                name
            )));
        }
        Ok(self)
    }

    /// Assert element has attribute with value
    pub fn has_attribute_value(&self, name: &str, expected: &str) -> DuskResult<&Self> {
        match self.element.attributes.get(name) {
            Some(value) if value == expected => Ok(self),
            Some(value) => Err(DuskError::AssertionFailed(format!(
                "Attribute '{}' value '{}' does not match '{}'",
                name, value, expected
            ))),
            None => Err(DuskError::AssertionFailed(format!(
                "Element does not have attribute '{}'",
                name
            ))),
        }
    }

    /// Assert element has class
    pub fn has_class(&self, class: &str) -> DuskResult<&Self> {
        if !self.element.has_class(class) {
            return Err(DuskError::AssertionFailed(format!(
                "Element does not have class '{}'",
                class
            )));
        }
        Ok(self)
    }
}
