//! Blade Component Slots
//!
//! Support for named slots and default slot content in components

use serde_json::Value;
use std::collections::HashMap;

/// Represents a single slot in a component
#[derive(Debug, Clone)]
pub struct Slot {
    /// Slot name (e.g., "header", "footer")
    pub name: String,

    /// Slot content (HTML)
    pub content: String,

    /// Slot attributes (for scoped slots)
    pub attributes: HashMap<String, String>,
}

impl Slot {
    /// Create a new slot
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            attributes: HashMap::new(),
        }
    }

    /// Create a slot with attributes
    pub fn with_attributes(
        name: impl Into<String>,
        content: impl Into<String>,
        attributes: HashMap<String, String>,
    ) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            attributes,
        }
    }

    /// Get slot content
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Get slot attribute
    pub fn get(&self, key: &str) -> Option<&String> {
        self.attributes.get(key)
    }

    /// Check if slot has attribute
    pub fn has(&self, key: &str) -> bool {
        self.attributes.contains_key(key)
    }

    /// Get all attributes as HTML string
    pub fn attributes_html(&self) -> String {
        self.attributes
            .iter()
            .map(|(k, v)| {
                let escaped = v
                    .replace('&', "&amp;")
                    .replace('"', "&quot;")
                    .replace('<', "&lt;")
                    .replace('>', "&gt;");
                format!("{}=\"{}\"", k, escaped)
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Collection of slots for a component
#[derive(Debug, Clone)]
pub struct SlotBag {
    /// Named slots
    slots: HashMap<String, Slot>,

    /// Default slot (unnamed content)
    default_slot: Option<Slot>,
}

impl SlotBag {
    /// Create an empty slot bag
    pub fn new() -> Self {
        Self {
            slots: HashMap::new(),
            default_slot: None,
        }
    }

    /// Add a named slot
    pub fn add_slot(&mut self, slot: Slot) {
        if slot.name == "default" {
            self.default_slot = Some(slot);
        } else {
            self.slots.insert(slot.name.clone(), slot);
        }
    }

    /// Set the default slot
    pub fn set_default(&mut self, content: impl Into<String>) {
        self.default_slot = Some(Slot::new("default", content));
    }

    /// Get a named slot
    pub fn get(&self, name: &str) -> Option<&Slot> {
        if name == "default" {
            self.default_slot.as_ref()
        } else {
            self.slots.get(name)
        }
    }

    /// Get the default slot
    pub fn default(&self) -> Option<&Slot> {
        self.default_slot.as_ref()
    }

    /// Check if a slot exists
    pub fn has(&self, name: &str) -> bool {
        if name == "default" {
            self.default_slot.is_some()
        } else {
            self.slots.contains_key(name)
        }
    }

    /// Get all slot names
    pub fn slot_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.slots.keys().map(|s| s.as_str()).collect();
        if self.default_slot.is_some() {
            names.push("default");
        }
        names.sort();
        names
    }

    /// Check if any slots exist
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty() && self.default_slot.is_none()
    }

    /// Convert to HashMap for template rendering
    pub fn to_map(&self) -> HashMap<String, String> {
        let mut map = HashMap::new();

        // Add default slot
        if let Some(slot) = &self.default_slot {
            map.insert("default".to_string(), slot.content.clone());
        }

        // Add named slots
        for (name, slot) in &self.slots {
            map.insert(name.clone(), slot.content.clone());
        }

        map
    }

    /// Convert to JSON value for template rendering
    pub fn to_json(&self) -> Value {
        let mut map = serde_json::Map::new();

        // Add default slot
        if let Some(slot) = &self.default_slot {
            map.insert("default".to_string(), Value::String(slot.content.clone()));
        }

        // Add named slots
        for (name, slot) in &self.slots {
            map.insert(name.clone(), Value::String(slot.content.clone()));
        }

        Value::Object(map)
    }
}

impl Default for SlotBag {
    fn default() -> Self {
        Self::new()
    }
}

impl From<HashMap<String, String>> for SlotBag {
    fn from(map: HashMap<String, String>) -> Self {
        let mut bag = SlotBag::new();

        for (name, content) in map {
            if name == "default" {
                bag.set_default(content);
            } else {
                bag.add_slot(Slot::new(name, content));
            }
        }

        bag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slot_creation() {
        let slot = Slot::new("header", "<h1>Title</h1>");
        assert_eq!(slot.name, "header");
        assert_eq!(slot.content, "<h1>Title</h1>");
        assert!(slot.attributes.is_empty());
    }

    #[test]
    fn test_slot_with_attributes() {
        let mut attrs = HashMap::new();
        attrs.insert("class".to_string(), "font-bold".to_string());

        let slot = Slot::with_attributes("header", "<h1>Title</h1>", attrs);
        assert_eq!(slot.name, "header");
        assert!(slot.has("class"));
        assert_eq!(slot.get("class"), Some(&"font-bold".to_string()));
    }

    #[test]
    fn test_slot_attributes_html() {
        let mut attrs = HashMap::new();
        attrs.insert("class".to_string(), "font-bold".to_string());
        attrs.insert("id".to_string(), "header".to_string());

        let slot = Slot::with_attributes("header", "<h1>Title</h1>", attrs);
        let html = slot.attributes_html();

        assert!(html.contains("class=\"font-bold\""));
        assert!(html.contains("id=\"header\""));
    }

    #[test]
    fn test_slot_bag_creation() {
        let bag = SlotBag::new();
        assert!(bag.is_empty());
    }

    #[test]
    fn test_slot_bag_add_slot() {
        let mut bag = SlotBag::new();
        bag.add_slot(Slot::new("header", "<h1>Title</h1>"));

        assert!(!bag.is_empty());
        assert!(bag.has("header"));
        assert_eq!(bag.get("header").unwrap().content, "<h1>Title</h1>");
    }

    #[test]
    fn test_slot_bag_default_slot() {
        let mut bag = SlotBag::new();
        bag.set_default("Default content");

        assert!(bag.has("default"));
        assert_eq!(bag.default().unwrap().content, "Default content");
    }

    #[test]
    fn test_slot_bag_multiple_slots() {
        let mut bag = SlotBag::new();
        bag.set_default("Main content");
        bag.add_slot(Slot::new("header", "Header"));
        bag.add_slot(Slot::new("footer", "Footer"));

        assert!(bag.has("default"));
        assert!(bag.has("header"));
        assert!(bag.has("footer"));
        assert_eq!(bag.slot_names().len(), 3);
    }

    #[test]
    fn test_slot_bag_to_map() {
        let mut bag = SlotBag::new();
        bag.set_default("Main");
        bag.add_slot(Slot::new("header", "Header"));

        let map = bag.to_map();
        assert_eq!(map.get("default"), Some(&"Main".to_string()));
        assert_eq!(map.get("header"), Some(&"Header".to_string()));
    }

    #[test]
    fn test_slot_bag_from_hashmap() {
        let mut map = HashMap::new();
        map.insert("default".to_string(), "Main".to_string());
        map.insert("header".to_string(), "Header".to_string());

        let bag: SlotBag = map.into();
        assert!(bag.has("default"));
        assert!(bag.has("header"));
    }

    #[test]
    fn test_slot_bag_to_json() {
        let mut bag = SlotBag::new();
        bag.set_default("Main");
        bag.add_slot(Slot::new("header", "Header"));

        let json = bag.to_json();
        assert!(json.is_object());

        let obj = json.as_object().unwrap();
        assert_eq!(obj.get("default").and_then(|v| v.as_str()), Some("Main"));
        assert_eq!(obj.get("header").and_then(|v| v.as_str()), Some("Header"));
    }
}
