use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;

use super::guard::Guard;

pub struct GuardRegistry {
    guards: HashMap<String, Arc<dyn Any + Send + Sync>>,
}

impl GuardRegistry {
    pub fn new() -> Self {
        Self {
            guards: HashMap::new(),
        }
    }

    pub fn register<G: Guard + 'static>(&mut self, name: &str, guard: G) {
        self.guards.insert(
            name.to_string(),
            Arc::new(guard) as Arc<dyn Any + Send + Sync>,
        );
    }

    pub fn guard<G: Guard + 'static>(&self, name: &str) -> Option<Arc<G>> {
        self.guards
            .get(name)
            .and_then(|guard| guard.clone().downcast::<G>().ok())
    }

    pub fn remove(&mut self, name: &str) {
        self.guards.remove(name);
    }

    pub fn is_registered(&self, name: &str) -> bool {
        self.guards.contains_key(name)
    }
}

impl Default for GuardRegistry {
    fn default() -> Self {
        Self::new()
    }
}
