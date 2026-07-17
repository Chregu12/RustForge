#![allow(dead_code)] // fields/methods retained for planned functionality, not read internally yet
//! View Composers
//!
//! Share data across multiple views using composers and creators

use crate::context::Context;
use crate::error::{ViewError, ViewResult};
use globset::{Glob, GlobSet, GlobSetBuilder};
use std::sync::{Arc, RwLock};

/// View composer trait
///
/// Implement this to create view composers that add data to views
pub trait ViewComposer: Send + Sync {
    /// Compose the view by adding data to the context
    fn compose(&self, view_name: &str, context: &mut Context) -> ViewResult<()>;

    /// Optional: Called when the composer is registered
    fn registered(&self) {}
}

/// Closure-based view composer
pub struct ClosureComposer<F>
where
    F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync,
{
    closure: F,
}

impl<F> ClosureComposer<F>
where
    F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync,
{
    /// Create a new closure composer
    pub fn new(closure: F) -> Self {
        Self { closure }
    }
}

impl<F> ViewComposer for ClosureComposer<F>
where
    F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync,
{
    fn compose(&self, view_name: &str, context: &mut Context) -> ViewResult<()> {
        (self.closure)(view_name, context)
    }
}

/// Composer entry with pattern matching
struct ComposerEntry {
    pattern: String,
    glob_set: GlobSet,
    composer: Arc<dyn ViewComposer>,
}

/// Registry for view composers
///
/// Manages view composers and applies them based on pattern matching
pub struct ComposerRegistry {
    composers: RwLock<Vec<ComposerEntry>>,
    creators: RwLock<Vec<ComposerEntry>>,
}

impl ComposerRegistry {
    /// Create a new composer registry
    pub fn new() -> Self {
        Self {
            composers: RwLock::new(Vec::new()),
            creators: RwLock::new(Vec::new()),
        }
    }

    /// Register a view composer
    ///
    /// # Example
    ///
    /// ```ignore
    /// registry.composer("posts.*", |view_name, context| {
    ///     context.insert("categories", get_categories());
    ///     Ok(())
    /// });
    /// ```
    pub fn composer<C>(&self, pattern: &str, composer: C) -> ViewResult<()>
    where
        C: ViewComposer + 'static,
    {
        let glob_set = self.build_glob_set(pattern)?;

        // Call registered before moving into Arc
        composer.registered();

        let entry = ComposerEntry {
            pattern: pattern.to_string(),
            glob_set,
            composer: Arc::new(composer),
        };

        self.composers.write().unwrap().push(entry);
        Ok(())
    }

    /// Register a view composer using a closure
    pub fn composer_fn<F>(&self, pattern: &str, closure: F) -> ViewResult<()>
    where
        F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync + 'static,
    {
        self.composer(pattern, ClosureComposer::new(closure))
    }

    /// Register a view creator (runs before composers)
    ///
    /// Creators run before composers and are useful for setting up initial data
    pub fn creator<C>(&self, pattern: &str, creator: C) -> ViewResult<()>
    where
        C: ViewComposer + 'static,
    {
        let glob_set = self.build_glob_set(pattern)?;

        // Call registered before moving into Arc
        creator.registered();

        let entry = ComposerEntry {
            pattern: pattern.to_string(),
            glob_set,
            composer: Arc::new(creator),
        };

        self.creators.write().unwrap().push(entry);
        Ok(())
    }

    /// Register a view creator using a closure
    pub fn creator_fn<F>(&self, pattern: &str, closure: F) -> ViewResult<()>
    where
        F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync + 'static,
    {
        self.creator(pattern, ClosureComposer::new(closure))
    }

    /// Compose a view
    ///
    /// Runs all matching creators and composers for the given view
    pub fn compose(&self, view_name: &str, context: &mut Context) -> ViewResult<()> {
        // Normalize view name: convert dots to slashes
        let normalized_name = view_name.replace('/', ".");

        // Run creators first
        let creators = self.creators.read().unwrap();
        for entry in creators.iter() {
            if self.matches(&entry.glob_set, &normalized_name) {
                entry.composer.compose(view_name, context)?;
            }
        }

        // Run composers
        let composers = self.composers.read().unwrap();
        for entry in composers.iter() {
            if self.matches(&entry.glob_set, &normalized_name) {
                entry.composer.compose(view_name, context)?;
            }
        }

        Ok(())
    }

    /// Build a glob set from pattern
    fn build_glob_set(&self, pattern: &str) -> ViewResult<GlobSet> {
        let mut builder = GlobSetBuilder::new();

        // Support both / and . as separators
        let normalized = pattern.replace('/', ".");

        // Add the pattern
        let glob = Glob::new(&normalized)
            .map_err(|e| ViewError::FilterError(format!("Invalid pattern: {}", e)))?;
        builder.add(glob);

        builder
            .build()
            .map_err(|e| ViewError::FilterError(format!("Failed to build glob set: {}", e)))
    }

    /// Check if view name matches glob set
    fn matches(&self, glob_set: &GlobSet, view_name: &str) -> bool {
        glob_set.is_match(view_name)
    }

    /// Clear all composers
    pub fn clear(&self) {
        self.composers.write().unwrap().clear();
        self.creators.write().unwrap().clear();
    }

    /// Get count of registered composers
    pub fn composer_count(&self) -> usize {
        self.composers.read().unwrap().len()
    }

    /// Get count of registered creators
    pub fn creator_count(&self) -> usize {
        self.creators.read().unwrap().len()
    }
}

impl Default for ComposerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global composer registry
static GLOBAL_REGISTRY: once_cell::sync::Lazy<Arc<ComposerRegistry>> =
    once_cell::sync::Lazy::new(|| Arc::new(ComposerRegistry::new()));

/// Get the global composer registry
pub fn global() -> Arc<ComposerRegistry> {
    Arc::clone(&GLOBAL_REGISTRY)
}

/// Register a global view composer
///
/// # Example
///
/// ```ignore
/// use rf_views::composers;
///
/// composers::composer("*", |_, context| {
///     context.insert("app_name", "MyApp");
///     Ok(())
/// });
/// ```
pub fn composer<C>(pattern: &str, composer: C) -> ViewResult<()>
where
    C: ViewComposer + 'static,
{
    global().composer(pattern, composer)
}

/// Register a global view composer using a closure
pub fn composer_fn<F>(pattern: &str, closure: F) -> ViewResult<()>
where
    F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync + 'static,
{
    global().composer_fn(pattern, closure)
}

/// Register a global view creator
pub fn creator<C>(pattern: &str, creator: C) -> ViewResult<()>
where
    C: ViewComposer + 'static,
{
    global().creator(pattern, creator)
}

/// Register a global view creator using a closure
pub fn creator_fn<F>(pattern: &str, closure: F) -> ViewResult<()>
where
    F: Fn(&str, &mut Context) -> ViewResult<()> + Send + Sync + 'static,
{
    global().creator_fn(pattern, closure)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_closure_composer() {
        let composer = ClosureComposer::new(|_, context| {
            context.insert("test", "value");
            Ok(())
        });

        let mut context = Context::new();
        composer.compose("test.view", &mut context).unwrap();

        assert_eq!(context.get("test").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn test_composer_registry() {
        let registry = ComposerRegistry::new();

        registry
            .composer_fn("test.*", |_, context| {
                context.insert("shared", "data");
                Ok(())
            })
            .unwrap();

        let mut context = Context::new();
        registry.compose("test.view", &mut context).unwrap();

        assert_eq!(context.get("shared").unwrap(), &serde_json::json!("data"));
    }

    #[test]
    fn test_composer_pattern_matching() {
        let registry = ComposerRegistry::new();

        registry
            .composer_fn("posts.*", |_, context| {
                context.insert("type", "post");
                Ok(())
            })
            .unwrap();

        registry
            .composer_fn("admin.*", |_, context| {
                context.insert("type", "admin");
                Ok(())
            })
            .unwrap();

        // Test posts pattern
        let mut context1 = Context::new();
        registry.compose("posts.index", &mut context1).unwrap();
        assert_eq!(context1.get("type").unwrap(), &serde_json::json!("post"));

        // Test admin pattern
        let mut context2 = Context::new();
        registry.compose("admin.dashboard", &mut context2).unwrap();
        assert_eq!(context2.get("type").unwrap(), &serde_json::json!("admin"));
    }

    #[test]
    fn test_wildcard_composer() {
        let registry = ComposerRegistry::new();

        registry
            .composer_fn("*", |_, context| {
                context.insert("global", "value");
                Ok(())
            })
            .unwrap();

        let mut context = Context::new();
        registry.compose("any.view", &mut context).unwrap();

        assert_eq!(context.get("global").unwrap(), &serde_json::json!("value"));
    }

    #[test]
    fn test_multiple_composers() {
        let registry = ComposerRegistry::new();

        registry
            .composer_fn("posts.*", |_, context| {
                context.insert("section", "posts");
                Ok(())
            })
            .unwrap();

        registry
            .composer_fn("posts.index", |_, context| {
                context.insert("page", "index");
                Ok(())
            })
            .unwrap();

        let mut context = Context::new();
        registry.compose("posts.index", &mut context).unwrap();

        // Both composers should run
        assert_eq!(context.get("section").unwrap(), &serde_json::json!("posts"));
        assert_eq!(context.get("page").unwrap(), &serde_json::json!("index"));
    }

    #[test]
    fn test_creator_runs_before_composer() {
        let registry = ComposerRegistry::new();

        registry
            .creator_fn("test", |_, context| {
                context.insert("value", "creator");
                Ok(())
            })
            .unwrap();

        registry
            .composer_fn("test", |_, context| {
                // Composer can access data from creator
                let value = context.get("value").unwrap();
                assert_eq!(value, &serde_json::json!("creator"));
                context.insert("value", "composer");
                Ok(())
            })
            .unwrap();

        let mut context = Context::new();
        registry.compose("test", &mut context).unwrap();

        // Composer should override creator
        assert_eq!(
            context.get("value").unwrap(),
            &serde_json::json!("composer")
        );
    }

    #[test]
    fn test_clear() {
        let registry = ComposerRegistry::new();

        registry.composer_fn("test", |_, _| Ok(())).unwrap();

        assert_eq!(registry.composer_count(), 1);

        registry.clear();

        assert_eq!(registry.composer_count(), 0);
    }
}
