//! Blade stacks implementation for @push and @stack directives
//!
//! Stacks allow you to push content onto named stacks which can be rendered
//! somewhere else in another view or layout.
//!
//! # Example
//!
//! ```blade
//! <!-- Layout -->
//! <head>
//!     @stack('scripts')
//! </head>
//!
//! <!-- Page -->
//! @push('scripts')
//!     <script src="/js/app.js"></script>
//! @endpush
//!
//! @push('scripts')
//!     <script src="/js/another.js"></script>
//! @endpush
//! ```
//!
//! This will output:
//!
//! ```html
//! <head>
//!     <script src="/js/app.js"></script>
//!     <script src="/js/another.js"></script>
//! </head>
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Stack manager for managing @push/@stack directives
#[derive(Debug, Clone)]
pub struct StackManager {
    stacks: Arc<Mutex<HashMap<String, Vec<String>>>>,
    prepend_stacks: Arc<Mutex<HashMap<String, Vec<String>>>>,
}

impl StackManager {
    /// Create a new stack manager
    pub fn new() -> Self {
        Self {
            stacks: Arc::new(Mutex::new(HashMap::new())),
            prepend_stacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Push content onto a stack
    pub fn push(&self, name: &str, content: String) {
        let mut stacks = self.stacks.lock().unwrap();
        stacks
            .entry(name.to_string())
            .or_default()
            .push(content);
    }

    /// Prepend content to a stack
    pub fn prepend(&self, name: &str, content: String) {
        let mut stacks = self.prepend_stacks.lock().unwrap();
        stacks
            .entry(name.to_string())
            .or_default()
            .push(content);
    }

    /// Render a stack's contents
    pub fn render(&self, name: &str) -> String {
        let mut output = Vec::new();

        // Get prepended items (in reverse order since they were prepended)
        if let Ok(prepend_stacks) = self.prepend_stacks.lock() {
            if let Some(items) = prepend_stacks.get(name) {
                output.extend(items.iter().rev().cloned());
            }
        }

        // Get normal pushed items
        if let Ok(stacks) = self.stacks.lock() {
            if let Some(items) = stacks.get(name) {
                output.extend(items.iter().cloned());
            }
        }

        output.join("\n")
    }

    /// Check if a stack has content
    pub fn has(&self, name: &str) -> bool {
        let has_normal = self
            .stacks
            .lock()
            .unwrap()
            .get(name)
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        let has_prepend = self
            .prepend_stacks
            .lock()
            .unwrap()
            .get(name)
            .map(|v| !v.is_empty())
            .unwrap_or(false);

        has_normal || has_prepend
    }

    /// Clear a specific stack
    pub fn clear(&self, name: &str) {
        if let Ok(mut stacks) = self.stacks.lock() {
            stacks.remove(name);
        }
        if let Ok(mut prepend_stacks) = self.prepend_stacks.lock() {
            prepend_stacks.remove(name);
        }
    }

    /// Clear all stacks
    pub fn clear_all(&self) {
        if let Ok(mut stacks) = self.stacks.lock() {
            stacks.clear();
        }
        if let Ok(mut prepend_stacks) = self.prepend_stacks.lock() {
            prepend_stacks.clear();
        }
    }

    /// Get all stack names
    pub fn stack_names(&self) -> Vec<String> {
        let mut names = Vec::new();

        if let Ok(stacks) = self.stacks.lock() {
            names.extend(stacks.keys().cloned());
        }

        if let Ok(prepend_stacks) = self.prepend_stacks.lock() {
            for key in prepend_stacks.keys() {
                if !names.contains(key) {
                    names.push(key.clone());
                }
            }
        }

        names
    }
}

impl Default for StackManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global stack instance for convenience
static GLOBAL_STACK: once_cell::sync::Lazy<StackManager> =
    once_cell::sync::Lazy::new(StackManager::new);

/// Get the global stack manager
pub fn global() -> &'static StackManager {
    &GLOBAL_STACK
}

/// Push content onto a stack (convenience function)
pub fn push(name: &str, content: String) {
    global().push(name, content);
}

/// Prepend content to a stack (convenience function)
pub fn prepend(name: &str, content: String) {
    global().prepend(name, content);
}

/// Render a stack (convenience function)
pub fn render(name: &str) -> String {
    global().render(name)
}

/// Check if a stack has content (convenience function)
pub fn has(name: &str) -> bool {
    global().has(name)
}

/// Clear a specific stack (convenience function)
pub fn clear(name: &str) {
    global().clear(name);
}

/// Clear all stacks (convenience function)
pub fn clear_all() {
    global().clear_all();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_push_and_render() {
        let manager = StackManager::new();

        manager.push("scripts", "<script src=\"app.js\"></script>".to_string());
        manager.push("scripts", "<script src=\"vendor.js\"></script>".to_string());

        let output = manager.render("scripts");
        assert!(output.contains("app.js"));
        assert!(output.contains("vendor.js"));

        // Verify order (app.js should come before vendor.js)
        let app_pos = output.find("app.js").unwrap();
        let vendor_pos = output.find("vendor.js").unwrap();
        assert!(app_pos < vendor_pos);
    }

    #[test]
    fn test_prepend() {
        let manager = StackManager::new();

        manager.push("scripts", "<script src=\"app.js\"></script>".to_string());
        manager.prepend("scripts", "<script src=\"vendor.js\"></script>".to_string());
        manager.prepend("scripts", "<script src=\"lib.js\"></script>".to_string());

        let output = manager.render("scripts");

        // Verify order: lib.js, vendor.js, app.js
        let lib_pos = output.find("lib.js").unwrap();
        let vendor_pos = output.find("vendor.js").unwrap();
        let app_pos = output.find("app.js").unwrap();

        assert!(lib_pos < vendor_pos);
        assert!(vendor_pos < app_pos);
    }

    #[test]
    fn test_has() {
        let manager = StackManager::new();

        assert!(!manager.has("scripts"));

        manager.push("scripts", "<script></script>".to_string());
        assert!(manager.has("scripts"));
    }

    #[test]
    fn test_clear() {
        let manager = StackManager::new();

        manager.push("scripts", "<script></script>".to_string());
        manager.push("styles", "<style></style>".to_string());

        assert!(manager.has("scripts"));
        assert!(manager.has("styles"));

        manager.clear("scripts");
        assert!(!manager.has("scripts"));
        assert!(manager.has("styles"));
    }

    #[test]
    fn test_clear_all() {
        let manager = StackManager::new();

        manager.push("scripts", "<script></script>".to_string());
        manager.push("styles", "<style></style>".to_string());

        manager.clear_all();

        assert!(!manager.has("scripts"));
        assert!(!manager.has("styles"));
    }

    #[test]
    fn test_multiple_stacks() {
        let manager = StackManager::new();

        manager.push("scripts", "<script src=\"app.js\"></script>".to_string());
        manager.push("styles", "<link href=\"app.css\">".to_string());

        let scripts = manager.render("scripts");
        let styles = manager.render("styles");

        assert!(scripts.contains("app.js"));
        assert!(!scripts.contains("app.css"));

        assert!(styles.contains("app.css"));
        assert!(!styles.contains("app.js"));
    }

    #[test]
    fn test_empty_stack() {
        let manager = StackManager::new();
        let output = manager.render("nonexistent");
        assert_eq!(output, "");
    }
}
