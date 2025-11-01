//! Configuration cache

use std::collections::HashMap;

pub struct ConfigCache {
    data: HashMap<String, serde_json::Value>,
}

impl ConfigCache {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    pub fn set(&mut self, key: String, value: serde_json::Value) {
        self.data.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl Default for ConfigCache {
    fn default() -> Self {
        Self::new()
    }
}
