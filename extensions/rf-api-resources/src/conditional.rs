//! Conditional resource field helpers.

use serde::Serialize;

/// Helper for conditional resource fields.
#[derive(Debug, Clone)]
pub struct Conditional<T> {
    value: T,
}

impl<T> Conditional<T> {
    /// Create a new conditional value.
    pub fn new(value: T) -> Self {
        Self { value }
    }

    /// Include the value when condition is true.
    pub fn when(self, condition: bool) -> Option<T> {
        if condition {
            Some(self.value)
        } else {
            None
        }
    }

    /// Include the value unless condition is true.
    pub fn unless(self, condition: bool) -> Option<T> {
        if !condition {
            Some(self.value)
        } else {
            None
        }
    }

    /// Include the value when closure returns true.
    pub fn when_fn<F>(self, f: F) -> Option<T>
    where
        F: FnOnce(&T) -> bool,
    {
        if f(&self.value) {
            Some(self.value)
        } else {
            None
        }
    }
}

/// Helper for merging conditional values.
pub struct MergeWhen<T> {
    values: Vec<T>,
}

impl<T> MergeWhen<T> {
    /// Create a new merge helper.
    pub fn new() -> Self {
        Self { values: Vec::new() }
    }

    /// Add a value when condition is true.
    pub fn when(mut self, condition: bool, value: T) -> Self {
        if condition {
            self.values.push(value);
        }
        self
    }

    /// Get all collected values.
    pub fn values(self) -> Vec<T> {
        self.values
    }
}

impl<T> Default for MergeWhen<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Trait for loading nested resources.
pub trait LoadRelations {
    /// Load specified relations.
    fn load(&mut self, relations: &[&str]);

    /// Check if a relation is loaded.
    fn is_loaded(&self, relation: &str) -> bool;
}

/// Helper for including nested resources conditionally.
#[derive(Debug, Clone, Serialize)]
pub struct WithRelation<T, R> {
    #[serde(flatten)]
    resource: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    relation: Option<R>,
}

impl<T, R> WithRelation<T, R> {
    /// Create a new resource with optional relation.
    pub fn new(resource: T, relation: Option<R>) -> Self {
        Self { resource, relation }
    }

    /// Include the relation when condition is true.
    pub fn when(resource: T, condition: bool, relation: R) -> Self {
        Self {
            resource,
            relation: if condition { Some(relation) } else { None },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conditional_when() {
        let value = Conditional::new("secret");
        assert!(value.clone().when(true).is_some());
        assert!(value.when(false).is_none());
    }

    #[test]
    fn test_conditional_unless() {
        let value = Conditional::new("secret");
        assert!(value.clone().unless(false).is_some());
        assert!(value.unless(true).is_none());
    }

    #[test]
    fn test_conditional_when_fn() {
        let value = Conditional::new(42);
        let result = value.when_fn(|v| *v > 40);
        assert_eq!(result, Some(42));

        let value = Conditional::new(30);
        let result = value.when_fn(|v| *v > 40);
        assert_eq!(result, None);
    }

    #[test]
    fn test_merge_when() {
        let merge = MergeWhen::new()
            .when(true, "first")
            .when(false, "second")
            .when(true, "third");

        let values = merge.values();
        assert_eq!(values.len(), 2);
        assert_eq!(values[0], "first");
        assert_eq!(values[1], "third");
    }

    #[test]
    fn test_with_relation() {
        #[derive(Debug, Clone, Serialize)]
        struct User {
            id: i64,
            name: String,
        }

        #[derive(Debug, Clone, Serialize)]
        struct Post {
            id: i64,
            title: String,
        }

        let user = User {
            id: 1,
            name: "John".to_string(),
        };

        let post = Post {
            id: 1,
            title: "Test".to_string(),
        };

        let with_relation = WithRelation::when(user.clone(), true, post.clone());
        let json = serde_json::to_value(&with_relation).unwrap();
        assert!(json["relation"].is_object());

        let without_relation = WithRelation::when(user, false, post);
        let json = serde_json::to_value(&without_relation).unwrap();
        assert!(json["relation"].is_null());
    }
}
