use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

/// Context for template rendering
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {
    data: HashMap<String, Value>,
}

impl Context {
    /// Create a new empty context
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context from a serializable value
    pub fn from_value<T: Serialize>(value: T) -> Result<Self, serde_json::Error> {
        let json = serde_json::to_value(value)?;
        if let Value::Object(map) = json {
            Ok(Self {
                data: map.into_iter().collect(),
            })
        } else {
            Ok(Self {
                data: HashMap::new(),
            })
        }
    }

    /// Insert a value into the context
    pub fn insert<T: Serialize>(&mut self, key: impl Into<String>, value: T) -> &mut Self {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.data.insert(key.into(), json_value);
        }
        self
    }

    /// Get a value from the context
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Remove a value from the context
    pub fn remove(&mut self, key: &str) -> Option<Value> {
        self.data.remove(key)
    }

    /// Check if a key exists in the context
    pub fn contains_key(&self, key: &str) -> bool {
        self.data.contains_key(key)
    }

    /// Merge another context into this one
    pub fn merge(&mut self, other: Context) -> &mut Self {
        self.data.extend(other.data);
        self
    }

    /// Convert to a Tera context
    pub fn to_tera(&self) -> tera::Context {
        let mut ctx = tera::Context::new();
        for (key, value) in &self.data {
            ctx.insert(key, value);
        }
        ctx
    }

    /// Convert to JSON value
    pub fn to_json(&self) -> Value {
        json!(self.data)
    }
}

impl From<HashMap<String, Value>> for Context {
    fn from(data: HashMap<String, Value>) -> Self {
        Self { data }
    }
}

impl From<Context> for tera::Context {
    fn from(ctx: Context) -> Self {
        ctx.to_tera()
    }
}

#[macro_export]
macro_rules! context {
    () => {
        $crate::Context::new()
    };
    ($($key:expr => $value:expr),* $(,)?) => {{
        let mut ctx = $crate::Context::new();
        $(
            ctx.insert($key, $value);
        )*
        ctx
    }};
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_new() {
        let ctx = Context::new();
        assert!(ctx.data.is_empty());
    }

    #[test]
    fn test_context_insert_get() {
        let mut ctx = Context::new();
        ctx.insert("name", "John");
        ctx.insert("age", 30);

        assert_eq!(ctx.get("name").unwrap().as_str().unwrap(), "John");
        assert_eq!(ctx.get("age").unwrap().as_i64().unwrap(), 30);
    }

    #[test]
    fn test_context_macro() {
        let ctx = context! {
            "name" => "Jane",
            "age" => 25,
        };

        assert_eq!(ctx.get("name").unwrap().as_str().unwrap(), "Jane");
        assert_eq!(ctx.get("age").unwrap().as_i64().unwrap(), 25);
    }

    #[test]
    fn test_context_merge() {
        let mut ctx1 = context! { "a" => 1 };
        let ctx2 = context! { "b" => 2 };

        ctx1.merge(ctx2);

        assert_eq!(ctx1.get("a").unwrap().as_i64().unwrap(), 1);
        assert_eq!(ctx1.get("b").unwrap().as_i64().unwrap(), 2);
    }

    #[test]
    fn test_context_from_value() {
        #[derive(Serialize)]
        struct User {
            name: String,
            age: u32,
        }

        let user = User {
            name: "Alice".to_string(),
            age: 28,
        };

        let ctx = Context::from_value(&user).unwrap();
        assert_eq!(ctx.get("name").unwrap().as_str().unwrap(), "Alice");
        assert_eq!(ctx.get("age").unwrap().as_u64().unwrap(), 28);
    }
}
