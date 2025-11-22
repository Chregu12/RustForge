//! Component Attribute Bag
//!
//! Provides attribute handling for Blade components

use std::collections::HashMap;

/// Attribute bag for components
///
/// Stores and manages component attributes with support for merging,
/// conditional attributes, and HTML rendering
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeBag {
    attributes: HashMap<String, String>,
}

impl AttributeBag {
    /// Create a new empty attribute bag
    pub fn new() -> Self {
        Self {
            attributes: HashMap::new(),
        }
    }

    /// Create from a list of key-value pairs
    pub fn from_pairs(pairs: Vec<(String, String)>) -> Self {
        let mut attributes = HashMap::new();
        for (key, value) in pairs {
            attributes.insert(key, value);
        }
        Self { attributes }
    }

    /// Get an attribute value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Set an attribute value
    pub fn set(&mut self, key: String, value: String) {
        self.attributes.insert(key, value);
    }

    /// Check if attribute exists
    pub fn has(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Remove an attribute
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.attributes.remove(key)
    }

    /// Merge attributes from another bag
    ///
    /// Special handling for class attribute - concatenates instead of replacing
    pub fn merge(&mut self, other: &AttributeBag) {
        for (key, value) in &other.attributes {
            if key == "class" {
                // Merge class attributes
                if let Some(existing) = self.attributes.get("class") {
                    self.attributes
                        .insert(key.clone(), format!("{} {}", existing, value));
                } else {
                    self.attributes.insert(key.clone(), value.clone());
                }
            } else {
                self.attributes.insert(key.clone(), value.clone());
            }
        }
    }

    /// Create a new bag with specific keys excluded
    pub fn except(&self, keys: &[&str]) -> AttributeBag {
        let mut new_attrs = HashMap::new();
        for (key, value) in &self.attributes {
            if !keys.contains(&key.as_str()) {
                new_attrs.insert(key.clone(), value.clone());
            }
        }
        AttributeBag {
            attributes: new_attrs,
        }
    }

    /// Create a new bag with only specific keys
    pub fn only(&self, keys: &[&str]) -> AttributeBag {
        let mut new_attrs = HashMap::new();
        for (key, value) in &self.attributes {
            if keys.contains(&key.as_str()) {
                new_attrs.insert(key.clone(), value.clone());
            }
        }
        AttributeBag {
            attributes: new_attrs,
        }
    }

    /// Render attributes as HTML string
    ///
    /// Example: `class="btn btn-primary" id="submit-btn"`
    pub fn to_html(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        for (key, value) in &self.attributes {
            if value.is_empty() {
                // Boolean attribute
                parts.push(key.clone());
            } else {
                // Escape value for safety
                let escaped = html_escape(value);
                parts.push(format!("{}=\"{}\"", key, escaped));
            }
        }

        parts.join(" ")
    }

    /// Get all attributes as a HashMap
    pub fn all(&self) -> &HashMap<String, String> {
        &self.attributes
    }

    /// Check if bag is empty
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty()
    }

    /// Get number of attributes
    pub fn len(&self) -> usize {
        self.attributes.len()
    }
}

impl Default for AttributeBag {
    fn default() -> Self {
        Self::new()
    }
}

/// HTML escape for attribute values
fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_attribute_bag() {
        let bag = AttributeBag::new();
        assert!(bag.is_empty());
        assert_eq!(bag.len(), 0);
    }

    #[test]
    fn test_from_pairs() {
        let bag = AttributeBag::from_pairs(vec![
            ("class".to_string(), "btn".to_string()),
            ("id".to_string(), "submit".to_string()),
        ]);

        assert_eq!(bag.len(), 2);
        assert_eq!(bag.get("class"), Some(&"btn".to_string()));
        assert_eq!(bag.get("id"), Some(&"submit".to_string()));
    }

    #[test]
    fn test_set_and_get() {
        let mut bag = AttributeBag::new();
        bag.set("type".to_string(), "button".to_string());

        assert_eq!(bag.get("type"), Some(&"button".to_string()));
    }

    #[test]
    fn test_has() {
        let mut bag = AttributeBag::new();
        bag.set("disabled".to_string(), "".to_string());

        assert!(bag.has("disabled"));
        assert!(!bag.has("enabled"));
    }

    #[test]
    fn test_remove() {
        let mut bag = AttributeBag::new();
        bag.set("temp".to_string(), "value".to_string());

        assert!(bag.has("temp"));
        let removed = bag.remove("temp");
        assert_eq!(removed, Some("value".to_string()));
        assert!(!bag.has("temp"));
    }

    #[test]
    fn test_merge_class_attributes() {
        let mut bag1 = AttributeBag::new();
        bag1.set("class".to_string(), "btn".to_string());

        let mut bag2 = AttributeBag::new();
        bag2.set("class".to_string(), "btn-primary".to_string());

        bag1.merge(&bag2);

        assert_eq!(bag1.get("class"), Some(&"btn btn-primary".to_string()));
    }

    #[test]
    fn test_merge_non_class_attributes() {
        let mut bag1 = AttributeBag::new();
        bag1.set("id".to_string(), "old".to_string());

        let mut bag2 = AttributeBag::new();
        bag2.set("id".to_string(), "new".to_string());

        bag1.merge(&bag2);

        assert_eq!(bag1.get("id"), Some(&"new".to_string()));
    }

    #[test]
    fn test_except() {
        let mut bag = AttributeBag::new();
        bag.set("class".to_string(), "btn".to_string());
        bag.set("id".to_string(), "submit".to_string());
        bag.set("type".to_string(), "button".to_string());

        let filtered = bag.except(&["id", "type"]);

        assert_eq!(filtered.len(), 1);
        assert!(filtered.has("class"));
        assert!(!filtered.has("id"));
        assert!(!filtered.has("type"));
    }

    #[test]
    fn test_only() {
        let mut bag = AttributeBag::new();
        bag.set("class".to_string(), "btn".to_string());
        bag.set("id".to_string(), "submit".to_string());
        bag.set("type".to_string(), "button".to_string());

        let filtered = bag.only(&["class", "id"]);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.has("class"));
        assert!(filtered.has("id"));
        assert!(!filtered.has("type"));
    }

    #[test]
    fn test_to_html() {
        let mut bag = AttributeBag::new();
        bag.set("class".to_string(), "btn btn-primary".to_string());
        bag.set("id".to_string(), "submit".to_string());

        let html = bag.to_html();

        // Order may vary
        assert!(html.contains("class=\"btn btn-primary\""));
        assert!(html.contains("id=\"submit\""));
    }

    #[test]
    fn test_to_html_boolean_attribute() {
        let mut bag = AttributeBag::new();
        bag.set("disabled".to_string(), "".to_string());

        let html = bag.to_html();

        assert_eq!(html, "disabled");
    }

    #[test]
    fn test_html_escape() {
        let mut bag = AttributeBag::new();
        bag.set(
            "data".to_string(),
            "<script>alert('xss')</script>".to_string(),
        );

        let html = bag.to_html();

        assert!(html.contains("&lt;script&gt;"));
        assert!(!html.contains("<script>"));
    }
}
