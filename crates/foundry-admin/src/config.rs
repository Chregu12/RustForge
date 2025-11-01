//! Admin panel configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminConfig {
    /// URL prefix for admin routes (default: "/admin")
    pub prefix: String,

    /// Enable authentication middleware
    pub require_auth: bool,

    /// Admin panel title
    pub title: String,

    /// Brand logo URL
    pub logo_url: Option<String>,

    /// Theme settings
    pub theme: ThemeConfig,

    /// Pagination settings
    pub pagination: PaginationConfig,

    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub primary_color: String,
    pub sidebar_color: String,
    pub dark_mode: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationConfig {
    pub per_page: usize,
    pub max_per_page: usize,
}

impl Default for AdminConfig {
    fn default() -> Self {
        Self {
            prefix: "/admin".to_string(),
            require_auth: true,
            title: "Foundry Admin".to_string(),
            logo_url: None,
            theme: ThemeConfig {
                primary_color: "#3b82f6".to_string(),
                sidebar_color: "#1f2937".to_string(),
                dark_mode: false,
            },
            pagination: PaginationConfig {
                per_page: 25,
                max_per_page: 100,
            },
            metadata: HashMap::new(),
        }
    }
}

impl AdminConfig {
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    pub fn with_auth(mut self, enabled: bool) -> Self {
        self.require_auth = enabled;
        self
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn with_dark_mode(mut self, enabled: bool) -> Self {
        self.theme.dark_mode = enabled;
        self
    }
}
