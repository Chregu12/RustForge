//! Configuration for Inertia.js

use crate::version::AssetVersion;

/// Inertia configuration
#[derive(Clone)]
pub struct InertiaConfig {
    /// Root view template name (e.g., "app" for app.html)
    pub root_view: String,

    /// Asset version for cache busting
    pub asset_version: AssetVersion,

    /// URL for the root template
    pub ssr_url: Option<String>,

    /// Enable Server-Side Rendering
    pub ssr_enabled: bool,
}

impl InertiaConfig {
    /// Create a new Inertia configuration with defaults
    pub fn new() -> Self {
        Self {
            root_view: "app".to_string(),
            asset_version: AssetVersion::default(),
            ssr_url: None,
            ssr_enabled: false,
        }
    }

    /// Set the root view template name
    pub fn root_view(mut self, name: impl Into<String>) -> Self {
        self.root_view = name.into();
        self
    }

    /// Set the asset version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.asset_version = AssetVersion::Fixed(version.into());
        self
    }

    /// Use a dynamic asset version
    pub fn version_fn<F>(mut self, f: F) -> Self
    where
        F: Fn() -> String + Send + Sync + 'static,
    {
        self.asset_version = AssetVersion::Dynamic(std::sync::Arc::new(f));
        self
    }

    /// Enable SSR with the given URL
    pub fn with_ssr(mut self, url: impl Into<String>) -> Self {
        self.ssr_url = Some(url.into());
        self.ssr_enabled = true;
        self
    }

    /// Get the current asset version
    pub fn get_version(&self) -> String {
        self.asset_version.get()
    }
}

impl Default for InertiaConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = InertiaConfig::new();
        assert_eq!(config.root_view, "app");
        assert!(!config.ssr_enabled);
    }

    #[test]
    fn test_config_builder() {
        let config = InertiaConfig::new()
            .root_view("main")
            .version("v1.0.0")
            .with_ssr("http://localhost:13714");

        assert_eq!(config.root_view, "main");
        assert_eq!(config.get_version(), "v1.0.0");
        assert!(config.ssr_enabled);
        assert_eq!(config.ssr_url, Some("http://localhost:13714".to_string()));
    }

    #[test]
    fn test_dynamic_version() {
        let config = InertiaConfig::new().version_fn(|| {
            // Dynamic version that returns a timestamp
            use std::time::{SystemTime, UNIX_EPOCH};
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
            format!("v{}", now)
        });

        // Verify version is generated
        let version = config.get_version();
        assert!(version.starts_with("v"));
        assert!(version.len() > 2);
    }
}
