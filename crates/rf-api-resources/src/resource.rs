//! Resource transformation traits and implementations.

use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

/// Trait for transforming models into API resources.
pub trait Resource: Serialize {
    /// Transform the resource into JSON.
    fn to_json(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
    }

    /// Wrap the resource with a custom key.
    fn wrap(self, key: &str) -> WrappedResource<Self>
    where
        Self: Sized,
    {
        WrappedResource {
            resource: self,
            key: key.to_string(),
        }
    }

    /// Add additional metadata to the resource.
    fn with_meta(self, meta: HashMap<String, Value>) -> ResourceWithMeta<Self>
    where
        Self: Sized,
    {
        ResourceWithMeta {
            resource: self,
            meta,
        }
    }
}

/// A wrapped resource with a custom key.
#[derive(Debug, Clone, Serialize)]
pub struct WrappedResource<T> {
    #[serde(flatten)]
    resource: T,
    #[serde(skip)]
    key: String,
}

impl<T: Serialize> WrappedResource<T> {
    /// Get the JSON representation with wrapping.
    pub fn to_json(&self) -> serde_json::Result<Value> {
        let mut map = serde_json::Map::new();
        map.insert(self.key.clone(), serde_json::to_value(&self.resource)?);
        Ok(Value::Object(map))
    }
}

/// A resource with additional metadata.
#[derive(Debug, Clone, Serialize)]
pub struct ResourceWithMeta<T> {
    #[serde(flatten)]
    resource: T,
    meta: HashMap<String, Value>,
}

impl<T: Serialize> ResourceWithMeta<T> {
    /// Get the JSON representation with metadata.
    pub fn to_json(&self) -> serde_json::Result<Value> {
        let mut map = serde_json::Map::new();

        // Add resource data
        if let Value::Object(obj) = serde_json::to_value(&self.resource)? {
            map.extend(obj);
        }

        // Add metadata
        map.insert("meta".to_string(), serde_json::to_value(&self.meta)?);

        Ok(Value::Object(map))
    }
}

/// Helper trait for conditional resource attributes.
pub trait ConditionalAttribute {
    /// Include the attribute when the condition is true.
    fn when(&self, condition: bool) -> Option<&Self> {
        if condition {
            Some(self)
        } else {
            None
        }
    }

    /// Include the attribute unless the condition is true.
    fn unless(&self, condition: bool) -> Option<&Self> {
        self.when(!condition)
    }
}

impl<T> ConditionalAttribute for T {}

/// Macro to conditionally include fields in resources.
#[macro_export]
macro_rules! when {
    ($condition:expr, $value:expr) => {
        if $condition {
            Some($value)
        } else {
            None
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Debug, Clone, Serialize)]
    struct TestResource {
        id: i64,
        name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        email: Option<String>,
    }

    impl Resource for TestResource {}

    #[test]
    fn test_resource_to_json() {
        let resource = TestResource {
            id: 1,
            name: "Test".to_string(),
            email: Some("test@example.com".to_string()),
        };

        let json = resource.to_json().unwrap();
        assert!(json.is_object());
        assert_eq!(json["id"], 1);
        assert_eq!(json["name"], "Test");
    }

    #[test]
    fn test_wrapped_resource() {
        let resource = TestResource {
            id: 1,
            name: "Test".to_string(),
            email: None,
        };

        let wrapped = resource.wrap("data");
        let json = wrapped.to_json().unwrap();

        assert!(json["data"].is_object());
        assert_eq!(json["data"]["id"], 1);
    }

    #[test]
    fn test_resource_with_meta() {
        let resource = TestResource {
            id: 1,
            name: "Test".to_string(),
            email: None,
        };

        let mut meta = HashMap::new();
        meta.insert("version".to_string(), Value::String("1.0".to_string()));

        let with_meta = resource.with_meta(meta);
        let json = with_meta.to_json().unwrap();

        assert_eq!(json["id"], 1);
        assert!(json["meta"].is_object());
        assert_eq!(json["meta"]["version"], "1.0");
    }

    #[test]
    fn test_conditional_attribute() {
        let value = "secret";

        assert!(value.when(true).is_some());
        assert!(value.when(false).is_none());
        assert!(value.unless(false).is_some());
        assert!(value.unless(true).is_none());
    }
}
