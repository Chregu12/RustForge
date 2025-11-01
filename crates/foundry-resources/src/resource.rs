//! Resource transformation traits and implementations

use async_trait::async_trait;
use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Context for resource transformation
#[derive(Debug, Clone, Default)]
pub struct ResourceContext {
    /// Fields to include (sparse fieldsets)
    pub fields: Option<Vec<String>>,
    /// Include nested resources
    pub includes: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Options for resource transformation
#[derive(Debug, Clone, Default)]
pub struct ResourceOptions {
    /// Enable field filtering
    pub enable_filtering: bool,
    /// Enable nested resources
    pub enable_includes: bool,
    /// Maximum depth for nested resources
    pub max_depth: usize,
}

impl Default for ResourceOptions {
    fn default() -> Self {
        Self {
            enable_filtering: true,
            enable_includes: true,
            max_depth: 3,
        }
    }
}

/// Main trait for API Resource transformation
#[async_trait]
pub trait Resource: Serialize + Sized {
    /// The underlying model type
    type Model;

    /// Transform a model into a resource
    fn from_model(model: Self::Model) -> Self;

    /// Transform with context
    fn from_model_with_context(model: Self::Model, _context: &ResourceContext) -> Self {
        Self::from_model(model)
    }

    /// Get resource type name
    fn resource_type() -> &'static str {
        std::any::type_name::<Self>()
    }

    /// Get available fields
    fn available_fields() -> Vec<&'static str> {
        vec![]
    }

    /// Apply field filtering
    fn filter_fields(&self, fields: &[String]) -> serde_json::Value {
        let value = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);

        if let serde_json::Value::Object(mut map) = value {
            map.retain(|k, _| fields.iter().any(|f| f == k));
            serde_json::Value::Object(map)
        } else {
            value
        }
    }

    /// Convert to JSON value
    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }

    /// Convert with context
    fn to_json_with_context(&self, context: &ResourceContext) -> serde_json::Value {
        if let Some(ref fields) = context.fields {
            self.filter_fields(fields)
        } else {
            self.to_json()
        }
    }
}

/// Trait for resources with relationships
#[async_trait]
pub trait ResourceWithRelations: Resource {
    /// Load relationships based on includes
    async fn load_relations(&mut self, _includes: &[String]) -> crate::Result<()> {
        Ok(())
    }

    /// Get available relationships
    fn available_relations() -> Vec<&'static str> {
        vec![]
    }
}

/// Trait for conditional resource attributes
pub trait ConditionalResource: Resource {
    /// Conditionally include fields based on closure
    fn when<F>(&self, condition: F, field_name: &str) -> Option<serde_json::Value>
    where
        F: Fn() -> bool,
    {
        if condition() {
            let value = self.to_json();
            if let serde_json::Value::Object(map) = value {
                map.get(field_name).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Include field when not null
    fn when_not_null(&self, field_name: &str) -> Option<serde_json::Value> {
        let value = self.to_json();
        if let serde_json::Value::Object(map) = value {
            let field = map.get(field_name)?;
            if !field.is_null() {
                Some(field.clone())
            } else {
                None
            }
        } else {
            None
        }
    }
}

/// Wrapper for optional resource fields
#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Optional<T> {
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<T>,
}

impl<T> Optional<T> {
    pub fn new(value: Option<T>) -> Self {
        Self { value }
    }

    pub fn some(value: T) -> Self {
        Self { value: Some(value) }
    }

    pub fn none() -> Self {
        Self { value: None }
    }
}

impl<T> From<Option<T>> for Optional<T> {
    fn from(value: Option<T>) -> Self {
        Self { value }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct TestResource {
        id: i32,
        name: String,
    }

    impl Resource for TestResource {
        type Model = (i32, String);

        fn from_model(model: Self::Model) -> Self {
            Self {
                id: model.0,
                name: model.1,
            }
        }
    }

    #[test]
    fn test_resource_transformation() {
        let resource = TestResource::from_model((1, "Test".to_string()));
        assert_eq!(resource.id, 1);
        assert_eq!(resource.name, "Test");
    }

    #[test]
    fn test_field_filtering() {
        let resource = TestResource::from_model((1, "Test".to_string()));
        let filtered = resource.filter_fields(&["id".to_string()]);

        let obj = filtered.as_object().unwrap();
        assert!(obj.contains_key("id"));
        assert!(!obj.contains_key("name"));
    }
}
