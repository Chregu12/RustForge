//! Component Props
//!
//! Type-safe prop handling for Blade components

use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PropError {
    #[error("Required prop missing: {0}")]
    RequiredPropMissing(String),

    #[error("Invalid prop type for {name}: expected {expected}, got {got}")]
    InvalidPropType {
        name: String,
        expected: String,
        got: String,
    },

    #[error("Prop validation failed: {0}")]
    ValidationFailed(String),
}

pub type PropResult<T> = Result<T, PropError>;

/// Component props container
#[derive(Debug, Clone)]
pub struct ComponentProps {
    props: HashMap<String, Value>,
    required: Vec<String>,
    defaults: HashMap<String, Value>,
}

impl ComponentProps {
    /// Create a new props container
    pub fn new() -> Self {
        Self {
            props: HashMap::new(),
            required: Vec::new(),
            defaults: HashMap::new(),
        }
    }

    /// Set a prop value
    pub fn set(&mut self, key: String, value: Value) {
        self.props.insert(key, value);
    }

    /// Get a prop value
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.props.get(key).or_else(|| self.defaults.get(key))
    }

    /// Get a prop as string
    pub fn get_string(&self, key: &str) -> Option<String> {
        self.get(key)?.as_str().map(|s| s.to_string())
    }

    /// Get a prop as integer
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key)?.as_i64()
    }

    /// Get a prop as float
    pub fn get_float(&self, key: &str) -> Option<f64> {
        self.get(key)?.as_f64()
    }

    /// Get a prop as boolean
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key)?.as_bool()
    }

    /// Mark a prop as required
    pub fn require(&mut self, key: String) {
        if !self.required.contains(&key) {
            self.required.push(key);
        }
    }

    /// Set a default value for a prop
    pub fn default(&mut self, key: String, value: Value) {
        self.defaults.insert(key, value);
    }

    /// Validate that all required props are present
    pub fn validate(&self) -> PropResult<()> {
        for required_key in &self.required {
            if !self.props.contains_key(required_key) && !self.defaults.contains_key(required_key) {
                return Err(PropError::RequiredPropMissing(required_key.clone()));
            }
        }
        Ok(())
    }

    /// Get all props as HashMap
    pub fn all(&self) -> HashMap<String, Value> {
        let mut all = self.defaults.clone();
        all.extend(self.props.clone());
        all
    }

    /// Create from attributes (string key-value pairs)
    pub fn from_attributes(attributes: &[(String, String)]) -> Self {
        let mut props = Self::new();

        for (key, value) in attributes {
            // Try to parse value as JSON for type inference
            if let Ok(json_value) = serde_json::from_str::<Value>(value) {
                props.set(key.clone(), json_value);
            } else {
                // Default to string
                props.set(key.clone(), Value::String(value.clone()));
            }
        }

        props
    }
}

impl Default for ComponentProps {
    fn default() -> Self {
        Self::new()
    }
}

/// Prop definition for documentation/validation
#[derive(Debug, Clone)]
pub struct PropDefinition {
    pub name: String,
    pub required: bool,
    pub default: Option<Value>,
    pub prop_type: PropType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PropType {
    String,
    Integer,
    Float,
    Boolean,
    Array,
    Object,
    Any,
}

impl PropDefinition {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            required: false,
            default: None,
            prop_type: PropType::Any,
        }
    }

    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    pub fn default_value(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    pub fn with_type(mut self, prop_type: PropType) -> Self {
        self.prop_type = prop_type;
        self
    }

    pub fn validate(&self, value: &Value) -> PropResult<()> {
        match (&self.prop_type, value) {
            (PropType::String, Value::String(_)) => Ok(()),
            (PropType::Integer, Value::Number(n)) if n.is_i64() => Ok(()),
            (PropType::Float, Value::Number(_)) => Ok(()),
            (PropType::Boolean, Value::Bool(_)) => Ok(()),
            (PropType::Array, Value::Array(_)) => Ok(()),
            (PropType::Object, Value::Object(_)) => Ok(()),
            (PropType::Any, _) => Ok(()),
            _ => Err(PropError::InvalidPropType {
                name: self.name.clone(),
                expected: format!("{:?}", self.prop_type),
                got: match value {
                    Value::String(_) => "String".to_string(),
                    Value::Number(_) => "Number".to_string(),
                    Value::Bool(_) => "Boolean".to_string(),
                    Value::Array(_) => "Array".to_string(),
                    Value::Object(_) => "Object".to_string(),
                    Value::Null => "Null".to_string(),
                },
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_create_props() {
        let props = ComponentProps::new();
        assert!(props.all().is_empty());
    }

    #[test]
    fn test_set_and_get() {
        let mut props = ComponentProps::new();
        props.set("name".to_string(), json!("Alice"));

        assert_eq!(props.get("name"), Some(&json!("Alice")));
    }

    #[test]
    fn test_get_string() {
        let mut props = ComponentProps::new();
        props.set("title".to_string(), json!("Hello World"));

        assert_eq!(props.get_string("title"), Some("Hello World".to_string()));
    }

    #[test]
    fn test_get_int() {
        let mut props = ComponentProps::new();
        props.set("count".to_string(), json!(42));

        assert_eq!(props.get_int("count"), Some(42));
    }

    #[test]
    fn test_get_bool() {
        let mut props = ComponentProps::new();
        props.set("active".to_string(), json!(true));

        assert_eq!(props.get_bool("active"), Some(true));
    }

    #[test]
    fn test_required_prop_missing() {
        let mut props = ComponentProps::new();
        props.require("name".to_string());

        let result = props.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_required_prop_present() {
        let mut props = ComponentProps::new();
        props.require("name".to_string());
        props.set("name".to_string(), json!("Alice"));

        let result = props.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_default_value() {
        let mut props = ComponentProps::new();
        props.default("type".to_string(), json!("primary"));

        assert_eq!(props.get("type"), Some(&json!("primary")));
    }

    #[test]
    fn test_override_default() {
        let mut props = ComponentProps::new();
        props.default("type".to_string(), json!("primary"));
        props.set("type".to_string(), json!("secondary"));

        assert_eq!(props.get("type"), Some(&json!("secondary")));
    }

    #[test]
    fn test_from_attributes() {
        let attributes = vec![
            ("class".to_string(), "btn".to_string()),
            ("id".to_string(), "submit".to_string()),
        ];

        let props = ComponentProps::from_attributes(&attributes);

        assert_eq!(props.get_string("class"), Some("btn".to_string()));
        assert_eq!(props.get_string("id"), Some("submit".to_string()));
    }

    #[test]
    fn test_prop_definition() {
        let def = PropDefinition::new("name")
            .required()
            .with_type(PropType::String);

        assert_eq!(def.name, "name");
        assert!(def.required);
        assert_eq!(def.prop_type, PropType::String);
    }

    #[test]
    fn test_prop_validation_success() {
        let def = PropDefinition::new("name").with_type(PropType::String);
        let result = def.validate(&json!("Alice"));

        assert!(result.is_ok());
    }

    #[test]
    fn test_prop_validation_failure() {
        let def = PropDefinition::new("count").with_type(PropType::Integer);
        let result = def.validate(&json!("not a number"));

        assert!(result.is_err());
    }
}
