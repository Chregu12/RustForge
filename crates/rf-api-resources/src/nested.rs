//! Nested Resource Loading
//!
//! Supports loading related resources lazily or eagerly

use serde::Serialize;

/// Trait for resources that can load nested relations
pub trait LoadsRelations {
    /// Get available relations
    fn available_relations(&self) -> Vec<&'static str>;

    /// Check if relation is loaded
    fn is_loaded(&self, relation: &str) -> bool;

    /// Load a relation (if not already loaded)
    fn load_relation(&mut self, relation: &str) -> Result<(), LoadError>;
}

#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("Relation '{0}' not found")]
    RelationNotFound(String),

    #[error("Failed to load relation: {0}")]
    LoadFailed(String),
}

/// Wrapper for nested resources
///
/// # Example
///
/// ```rust,ignore
/// #[derive(Serialize)]
/// struct UserResource {
///     id: i64,
///     name: String,
///     #[serde(skip_serializing_if = "Option::is_none")]
///     posts: Option<NestedResource<Vec<PostResource>>>,
/// }
///
/// impl UserResource {
///     fn with_posts(mut self) -> Self {
///         // Load posts from database
///         self.posts = Some(NestedResource::loaded(posts));
///         self
///     }
/// }
/// ```
#[derive(Debug, Clone)]
pub enum NestedResource<T> {
    NotLoaded,
    Loaded(T),
}

impl<T> NestedResource<T> {
    pub fn loaded(value: T) -> Self {
        Self::Loaded(value)
    }

    pub fn is_loaded(&self) -> bool {
        matches!(self, Self::Loaded(_))
    }

    pub fn as_ref(&self) -> Option<&T> {
        match self {
            Self::Loaded(ref v) => Some(v),
            Self::NotLoaded => None,
        }
    }

    pub fn as_mut(&mut self) -> Option<&mut T> {
        match self {
            Self::Loaded(ref mut v) => Some(v),
            Self::NotLoaded => None,
        }
    }

    pub fn unwrap(self) -> T {
        match self {
            Self::Loaded(v) => v,
            Self::NotLoaded => panic!("Called unwrap on NotLoaded NestedResource"),
        }
    }

    pub fn map<U, F>(self, f: F) -> NestedResource<U>
    where
        F: FnOnce(T) -> U,
    {
        match self {
            Self::Loaded(v) => NestedResource::Loaded(f(v)),
            Self::NotLoaded => NestedResource::NotLoaded,
        }
    }
}

impl<T: Serialize> Serialize for NestedResource<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Loaded(v) => v.serialize(serializer),
            Self::NotLoaded => serializer.serialize_none(),
        }
    }
}

/// Resource transformer that handles eager loading
pub struct ResourceTransformer {
    with_relations: Vec<String>,
}

impl ResourceTransformer {
    pub fn new() -> Self {
        Self {
            with_relations: Vec::new(),
        }
    }

    /// Specify which relations to load
    pub fn with(mut self, relations: Vec<&str>) -> Self {
        self.with_relations = relations.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Check if a relation should be loaded
    pub fn should_load(&self, relation: &str) -> bool {
        self.with_relations.contains(&relation.to_string())
            || self.with_relations.contains(&"*".to_string())
    }
}

impl Default for ResourceTransformer {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper for parsing ?with=posts,comments query parameters
pub fn parse_with_param(with_param: &str) -> Vec<String> {
    with_param
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[derive(Debug, Clone, Serialize)]
    struct Post {
        id: i64,
        title: String,
    }

    #[derive(Debug, Clone, Serialize)]
    struct User {
        id: i64,
        name: String,
        #[serde(skip_serializing_if = "NestedResource::is_not_loaded")]
        posts: NestedResource<Vec<Post>>,
    }

    impl NestedResource<Vec<Post>> {
        fn is_not_loaded(&self) -> bool {
            !self.is_loaded()
        }
    }

    #[test]
    fn test_nested_resource_loaded() {
        let posts = vec![
            Post {
                id: 1,
                title: "Post 1".to_string(),
            },
            Post {
                id: 2,
                title: "Post 2".to_string(),
            },
        ];

        let nested = NestedResource::loaded(posts);
        assert!(nested.is_loaded());
        assert_eq!(nested.as_ref().unwrap().len(), 2);
    }

    #[test]
    fn test_nested_resource_not_loaded() {
        let nested: NestedResource<Vec<Post>> = NestedResource::NotLoaded;
        assert!(!nested.is_loaded());
        assert!(nested.as_ref().is_none());
    }

    #[test]
    fn test_serialization_with_nested() {
        let user = User {
            id: 1,
            name: "John".to_string(),
            posts: NestedResource::loaded(vec![Post {
                id: 1,
                title: "Post 1".to_string(),
            }]),
        };

        let json = serde_json::to_value(&user).unwrap();
        assert!(json["posts"].is_array());
        assert_eq!(json["posts"][0]["title"], "Post 1");
    }

    #[test]
    fn test_serialization_without_nested() {
        let user = User {
            id: 1,
            name: "John".to_string(),
            posts: NestedResource::NotLoaded,
        };

        let json = serde_json::to_value(&user).unwrap();
        assert!(json.get("posts").is_none());
    }

    #[test]
    fn test_resource_transformer() {
        let transformer = ResourceTransformer::new().with(vec!["posts", "comments"]);

        assert!(transformer.should_load("posts"));
        assert!(transformer.should_load("comments"));
        assert!(!transformer.should_load("likes"));
    }

    #[test]
    fn test_parse_with_param() {
        let relations = parse_with_param("posts,comments,author");
        assert_eq!(relations.len(), 3);
        assert_eq!(relations[0], "posts");
        assert_eq!(relations[1], "comments");
        assert_eq!(relations[2], "author");
    }

    #[test]
    fn test_nested_resource_map() {
        let nested = NestedResource::loaded(vec![1, 2, 3]);
        let mapped = nested.map(|v| v.len());

        assert!(mapped.is_loaded());
        assert_eq!(mapped.unwrap(), 3);
    }
}
