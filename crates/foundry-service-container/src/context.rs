use std::any::TypeId;
use std::collections::HashMap;

/// Contextual binding allows different implementations based on context
pub struct ContextualBinding {
    /// The service that needs the dependency
    pub when: TypeId,
    /// The interface being requested
    pub needs: String,
    /// The implementation to provide
    pub give: String,
}

/// Store for contextual bindings
#[allow(dead_code)]
pub struct ContextualBindingStore {
    bindings: HashMap<(TypeId, String), String>,
}

#[allow(dead_code)]
impl ContextualBindingStore {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }

    /// Add a contextual binding
    pub fn add(&mut self, when: TypeId, needs: String, give: String) {
        self.bindings.insert((when, needs), give);
    }

    /// Get the implementation for a given context
    pub fn get(&self, when: TypeId, needs: &str) -> Option<&String> {
        self.bindings.get(&(when, needs.to_string()))
    }
}

impl Default for ContextualBindingStore {
    fn default() -> Self {
        Self::new()
    }
}
