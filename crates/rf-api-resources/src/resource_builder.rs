//! Advanced Resource Builder with Laravel-like features

use serde::{Serialize, Serializer};
use serde_json::{Map, Value};
use std::collections::HashMap;

/// Builder for creating dynamic API resources
///
/// # Example
///
/// ```rust,ignore
/// let resource = ResourceBuilder::new()
///     .add("id", user.id)
///     .add("name", user.name)
///     .when(is_admin, |r| r.add("secret", user.secret))
///     .when_loaded(&user.posts, |r, posts| {
///         r.add("posts", PostResource::collection(posts))
///     })
///     .merge_when(show_timestamps, json!({
///         "created_at": user.created_at,
///         "updated_at": user.updated_at,
///     }))
///     .build();
/// ```
pub struct ResourceBuilder {
    data: Map<String, Value>,
    loaded_relations: HashMap<String, bool>,
}

impl ResourceBuilder {
    pub fn new() -> Self {
        Self {
            data: Map::new(),
            loaded_relations: HashMap::new(),
        }
    }

    /// Add a field to the resource
    pub fn add<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Serialize,
    {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
        self
    }

    /// Conditionally add a field
    pub fn when<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            f(self)
        } else {
            self
        }
    }

    /// Conditionally add a field unless condition is true
    pub fn unless<F>(self, condition: bool, f: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        self.when(!condition, f)
    }

    /// Add field only when relation is loaded
    pub fn when_loaded<T, F>(mut self, relation_name: &str, relation: &Option<T>, f: F) -> Self
    where
        F: FnOnce(Self, &T) -> Self,
    {
        if let Some(rel) = relation {
            self.loaded_relations.insert(relation_name.to_string(), true);
            f(self, rel)
        } else {
            self.loaded_relations.insert(relation_name.to_string(), false);
            self
        }
    }

    /// Add field only when relation collection is loaded
    pub fn when_loaded_vec<T, F>(mut self, relation_name: &str, relation: &[T], f: F) -> Self
    where
        F: FnOnce(Self, &[T]) -> Self,
    {
        self.loaded_relations.insert(relation_name.to_string(), !relation.is_empty());
        if !relation.is_empty() {
            f(self, relation)
        } else {
            self
        }
    }

    /// Merge additional data into resource
    pub fn merge(mut self, data: Value) -> Self {
        if let Value::Object(map) = data {
            self.data.extend(map);
        }
        self
    }

    /// Conditionally merge data
    pub fn merge_when(self, condition: bool, data: Value) -> Self {
        if condition {
            self.merge(data)
        } else {
            self
        }
    }

    /// Add metadata
    pub fn with_meta(mut self, meta: Value) -> Self {
        self.data.insert("meta".to_string(), meta);
        self
    }

    /// Build the final JSON value
    pub fn build(self) -> Value {
        Value::Object(self.data)
    }

    /// Check if a relation was loaded
    pub fn is_loaded(&self, relation: &str) -> bool {
        self.loaded_relations.get(relation).copied().unwrap_or(false)
    }
}

impl Default for ResourceBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl Serialize for ResourceBuilder {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.data.serialize(serializer)
    }
}

/// Macro for creating resources more conveniently
///
/// # Example
///
/// ```rust,ignore
/// let resource = resource! {
///     "id" => user.id,
///     "name" => user.name,
///     "email" => user.email,
/// };
/// ```
#[macro_export]
macro_rules! resource {
    (
        $($key:literal => $value:expr),* $(,)?
    ) => {{
        let mut builder = $crate::resource_builder::ResourceBuilder::new();
        $(
            builder = builder.add($key, $value);
        )*
        builder.build()
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resource_builder() {
        let resource = ResourceBuilder::new()
            .add("id", 1)
            .add("name", "Test")
            .add("email", "test@example.com")
            .build();

        assert_eq!(resource["id"], 1);
        assert_eq!(resource["name"], "Test");
        assert_eq!(resource["email"], "test@example.com");
    }

    #[test]
    fn test_conditional_add() {
        let is_admin = true;
        let resource = ResourceBuilder::new()
            .add("id", 1)
            .when(is_admin, |r| r.add("admin", true))
            .when(false, |r| r.add("hidden", "secret"))
            .build();

        assert_eq!(resource["id"], 1);
        assert_eq!(resource["admin"], true);
        assert!(resource.get("hidden").is_none());
    }

    #[test]
    fn test_when_loaded() {
        let posts = Some(vec!["post1", "post2"]);
        let comments: Option<Vec<String>> = None;

        let resource = ResourceBuilder::new()
            .add("id", 1)
            .when_loaded("posts", &posts, |r, p| r.add("posts", p))
            .when_loaded("comments", &comments, |r, c| r.add("comments", c))
            .build();

        assert!(resource.get("posts").is_some());
        assert!(resource.get("comments").is_none());
    }

    #[test]
    fn test_merge() {
        let resource = ResourceBuilder::new()
            .add("id", 1)
            .merge(json!({
                "name": "Test",
                "email": "test@example.com"
            }))
            .build();

        assert_eq!(resource["id"], 1);
        assert_eq!(resource["name"], "Test");
        assert_eq!(resource["email"], "test@example.com");
    }

    #[test]
    fn test_merge_when() {
        let show_timestamps = true;
        let resource = ResourceBuilder::new()
            .add("id", 1)
            .merge_when(show_timestamps, json!({
                "created_at": "2024-01-01",
                "updated_at": "2024-01-02"
            }))
            .merge_when(false, json!({
                "hidden": "value"
            }))
            .build();

        assert_eq!(resource["id"], 1);
        assert_eq!(resource["created_at"], "2024-01-01");
        assert!(resource.get("hidden").is_none());
    }

    #[test]
    fn test_with_meta() {
        let resource = ResourceBuilder::new()
            .add("id", 1)
            .with_meta(json!({
                "version": "1.0",
                "api": "v1"
            }))
            .build();

        assert_eq!(resource["id"], 1);
        assert!(resource["meta"].is_object());
        assert_eq!(resource["meta"]["version"], "1.0");
    }
}
