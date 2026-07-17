use std::path::PathBuf;

/// Configuration for the view engine
#[derive(Debug, Clone)]
pub struct ViewConfig {
    /// Path to the views directory
    pub views_path: PathBuf,

    /// Whether to cache compiled templates
    pub cache_enabled: bool,

    /// Whether to automatically reload templates when they change
    pub auto_reload: bool,

    /// Whether to enable strict mode (fail on missing variables)
    pub strict_mode: bool,

    /// Default file extension for templates
    pub extension: String,
}

impl Default for ViewConfig {
    /// The default extension is **`.tera`**.  Call `.extension("html")` (or use
    /// [`ViewEngine::html`](crate::engine::ViewEngine::html)) when your templates
    /// use the `.html` extension that the umbrella `rf::view()` helper expects.
    fn default() -> Self {
        Self {
            views_path: PathBuf::from("resources/views"),
            cache_enabled: true,
            auto_reload: cfg!(debug_assertions),
            strict_mode: false,
            extension: "tera".to_string(),
        }
    }
}

impl ViewConfig {
    /// Create a new view configuration.
    ///
    /// The default template extension is `.tera`.  To use `.html` templates
    /// (consistent with the `rf::view()` / `rf_response::view` helper) call
    /// `.extension("html")` on the returned config, or use
    /// [`ViewEngine::html`](crate::engine::ViewEngine::html) directly.
    pub fn new(views_path: impl Into<PathBuf>) -> Self {
        Self {
            views_path: views_path.into(),
            ..Default::default()
        }
    }

    /// Set whether to enable caching
    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.cache_enabled = enabled;
        self
    }

    /// Set whether to enable auto-reload
    pub fn auto_reload(mut self, enabled: bool) -> Self {
        self.auto_reload = enabled;
        self
    }

    /// Set whether to enable strict mode
    pub fn strict_mode(mut self, enabled: bool) -> Self {
        self.strict_mode = enabled;
        self
    }

    /// Set the default template extension
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        self.extension = extension.into();
        self
    }

    /// Get the glob pattern for loading templates
    pub fn glob_pattern(&self) -> String {
        format!("{}/**/*.{}", self.views_path.display(), self.extension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ViewConfig::default();
        assert_eq!(config.views_path, PathBuf::from("resources/views"));
        assert!(config.cache_enabled);
        assert_eq!(config.extension, "tera");
    }

    #[test]
    fn test_config_builder() {
        let config = ViewConfig::new("templates")
            .cache_enabled(false)
            .auto_reload(true)
            .strict_mode(true)
            .extension("html");

        assert_eq!(config.views_path, PathBuf::from("templates"));
        assert!(!config.cache_enabled);
        assert!(config.auto_reload);
        assert!(config.strict_mode);
        assert_eq!(config.extension, "html");
    }

    #[test]
    fn test_glob_pattern() {
        let config = ViewConfig::new("views").extension("html");
        assert_eq!(config.glob_pattern(), "views/**/*.html");
    }
}
