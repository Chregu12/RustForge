//! Resource transformation traits and implementations.

use serde::{Serialize, Serializer};
use serde_json::Value;
use std::collections::HashMap;

/// Marker trait for API resource types.
///
/// # Which resource crate should I use?
///
/// RustForge ships two resource crates with different roles:
///
/// | | `rf-api-resources` | `rf-resources` |
/// |---|---|---|
/// | **Role** | Marker trait + collection / wrapping helpers | Full model-to-resource transformer contract |
/// | **`Resource` requires** | Just `Serialize` | `Serialize + from_model(Model) + Sized` |
/// | **`to_json()` returns** | `serde_json::Result<Value>` (can fail) | `serde_json::Value` (panics on error) |
/// | **Wrapping** | `.wrap(key)` → `WrappedResource` | n/a |
/// | **Use when** | You want collection/pagination helpers and simple wrapping | You need a typed Model→Resource pipeline |
///
/// **Important:** Switching between the two traits is NOT a drop-in replacement because
/// `to_json()` has different return types (one wraps in `Result`, the other panics-or-returns-Value).
/// Prefer `rf-api-resources::Resource` for public API handlers; prefer `rf-resources::Resource`
/// when you need the `from_model` transformer contract enforced by the compiler.
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
///
/// `axum::Json(wrapped)` and `serde_json::to_value(&wrapped)` both produce
/// `{"<key>": { …resource fields… }}`, identical to `wrapped.to_json()`.
#[derive(Debug, Clone)]
pub struct WrappedResource<T> {
    resource: T,
    key: String,
}

/// Manual Serialize so that both `axum::Json(wrapped)` and `serde_json::to_value(&wrapped)`
/// emit `{"<key>": { …resource fields… }}` — the same shape as `to_json()`.
///
/// The old `#[derive(Serialize)]` with `#[serde(flatten)] resource` + `#[serde(skip)] key`
/// silently dropped the wrapper and emitted flat fields at the top level.
impl<T: Serialize> Serialize for WrappedResource<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.key, &self.resource)?;
        map.end()
    }
}

impl<T: Serialize> WrappedResource<T> {
    /// Get the JSON representation with wrapping.
    ///
    /// Equivalent to `serde_json::to_value(self)` after the Serialize fix —
    /// both paths now produce `{"<key>": { …resource fields… }}`.
    pub fn to_json(&self) -> serde_json::Result<Value> {
        serde_json::to_value(self)
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

    /// Regression: serde_json::to_value (what axum::Json calls) must produce the
    /// same {"data":{...}} envelope as to_json(), not flat fields at the top level.
    #[test]
    fn test_wrapped_resource_serialize_matches_to_json() {
        let resource = TestResource {
            id: 42,
            name: "Serialize".to_string(),
            email: Some("s@test.com".to_string()),
        };
        let wrapped = resource.wrap("data");
        let via_serialize = serde_json::to_value(&wrapped).unwrap();
        let via_to_json = wrapped.to_json().unwrap();
        // Both paths must be identical.
        assert_eq!(
            via_serialize, via_to_json,
            "axum::Json path diverges from to_json()"
        );
        // The wrapper key must exist at the top level.
        assert!(
            via_serialize["data"].is_object(),
            "top-level 'data' key missing from Serialize output"
        );
        assert_eq!(via_serialize["data"]["id"], 42);
        assert_eq!(via_serialize["data"]["name"], "Serialize");
        // Flat top-level fields must NOT appear (the old broken behavior).
        assert!(
            via_serialize.get("id").is_none(),
            "flat 'id' must not appear at top level"
        );
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
