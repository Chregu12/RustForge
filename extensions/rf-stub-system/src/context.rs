use std::collections::HashMap;

/// Context for rendering a stub with data
#[derive(Debug, Clone)]
pub struct StubContext {
    pub name: String,
    pub namespace: String,
    pub properties: HashMap<String, String>,
    pub custom: HashMap<String, String>,
}

impl StubContext {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            namespace: String::new(),
            properties: HashMap::new(),
            custom: HashMap::new(),
        }
    }

    pub fn with_namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = namespace.into();
        self
    }

    pub fn with_property(mut self, name: impl Into<String>, type_: impl Into<String>) -> Self {
        self.properties.insert(name.into(), type_.into());
        self
    }

    pub fn with_custom(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom.insert(key.into(), value.into());
        self
    }

    /// Merge another context into this one
    pub fn merge(mut self, other: StubContext) -> Self {
        self.properties.extend(other.properties);
        self.custom.extend(other.custom);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_creation() {
        let ctx = StubContext::new("User")
            .with_namespace("app::models")
            .with_property("id", "i64")
            .with_property("name", "String")
            .with_custom("table", "users");

        assert_eq!(ctx.name, "User");
        assert_eq!(ctx.namespace, "app::models");
        assert_eq!(ctx.properties.len(), 2);
        assert_eq!(ctx.custom.get("table"), Some(&"users".to_string()));
    }

    #[test]
    fn test_context_merge() {
        let ctx1 = StubContext::new("User")
            .with_property("id", "i64");

        let ctx2 = StubContext::new("User")
            .with_property("name", "String")
            .with_custom("table", "users");

        let merged = ctx1.merge(ctx2);

        assert_eq!(merged.properties.len(), 2);
        assert_eq!(merged.custom.len(), 1);
    }
}
