//! Nova main struct and configuration
//!
//! The Nova instance that manages resources, dashboards, and routing.

use crate::dashboard::Dashboard;
use crate::resource::Resource;
use axum::Router;
use sea_orm::DatabaseConnection;
use std::collections::HashMap;
use std::sync::Arc;

/// Nova configuration
#[derive(Debug, Clone)]
pub struct NovaConfig {
    /// Base path for Nova routes (e.g., "/admin")
    pub path: String,

    /// Application name
    pub name: String,

    /// Brand logo URL
    pub logo: Option<String>,

    /// Enable global search
    pub global_search: bool,

    /// Items per page options
    pub per_page_options: Vec<u64>,

    /// Default items per page
    pub default_per_page: u64,

    /// Theme (light, dark, auto)
    pub theme: String,

    /// Primary color (hex)
    pub primary_color: String,
}

impl Default for NovaConfig {
    fn default() -> Self {
        Self {
            path: "/nova".to_string(),
            name: "Nova".to_string(),
            logo: None,
            global_search: true,
            per_page_options: vec![15, 25, 50, 100],
            default_per_page: 15,
            theme: "auto".to_string(),
            primary_color: "#4299E1".to_string(),
        }
    }
}

/// Resource registration
pub struct ResourceRegistration {
    pub name: String,
    pub uri_key: String,
    pub group: Option<String>,
}

/// Dashboard registration
pub struct DashboardRegistration {
    pub name: String,
    pub uri_key: String,
}

/// Nova instance
pub struct Nova {
    config: NovaConfig,
    db: Option<DatabaseConnection>,
    resources: HashMap<String, ResourceRegistration>,
    dashboards: Vec<DashboardRegistration>,
}

impl Nova {
    /// Create a new Nova instance
    pub fn new() -> Self {
        Self {
            config: NovaConfig::default(),
            db: None,
            resources: HashMap::new(),
            dashboards: vec![],
        }
    }

    /// Set the base path for Nova
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.config.path = path.into();
        self
    }

    /// Set the application name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.config.name = name.into();
        self
    }

    /// Set the logo URL
    pub fn with_logo(mut self, logo: impl Into<String>) -> Self {
        self.config.logo = Some(logo.into());
        self
    }

    /// Set the primary color
    pub fn with_primary_color(mut self, color: impl Into<String>) -> Self {
        self.config.primary_color = color.into();
        self
    }

    /// Set the theme
    pub fn with_theme(mut self, theme: impl Into<String>) -> Self {
        self.config.theme = theme.into();
        self
    }

    /// Attach database connection
    pub fn with_db(mut self, db: DatabaseConnection) -> Self {
        self.db = Some(db);
        self
    }

    /// Register a resource
    pub fn register_resource<R: Resource + 'static>(mut self) -> Self {
        let name = R::name().to_string();
        let registration = ResourceRegistration {
            name: name.clone(),
            uri_key: R::plural().to_string(),
            group: R::group().map(String::from),
        };
        self.resources.insert(name, registration);
        self
    }

    /// Register multiple resources
    pub fn register_resources<R: Resource + 'static>(self, resources: Vec<Box<dyn Fn() -> R>>) -> Self {
        resources.into_iter().fold(self, |nova, _| nova.register_resource::<R>())
    }

    /// Register a dashboard
    pub fn register_dashboard<D: Dashboard + 'static>(mut self, dashboard: D) -> Self {
        self.dashboards.push(DashboardRegistration {
            name: dashboard.name().to_string(),
            uri_key: dashboard.uri_key(),
        });
        self
    }

    /// Get configuration
    pub fn config(&self) -> &NovaConfig {
        &self.config
    }

    /// Get database connection
    pub fn db(&self) -> Option<&DatabaseConnection> {
        self.db.as_ref()
    }

    /// Get registered resources
    pub fn resources(&self) -> &HashMap<String, ResourceRegistration> {
        &self.resources
    }

    /// Get registered dashboards
    pub fn dashboards(&self) -> &[DashboardRegistration] {
        &self.dashboards
    }

    /// Build the Nova routes
    pub fn routes(self) -> Router {
        crate::routes::build_routes(Arc::new(self))
    }

    /// Get resource groups for sidebar navigation
    pub fn resource_groups(&self) -> HashMap<String, Vec<&ResourceRegistration>> {
        let mut groups: HashMap<String, Vec<&ResourceRegistration>> = HashMap::new();

        for resource in self.resources.values() {
            let group_name = resource.group.clone().unwrap_or_else(|| "Resources".to_string());
            groups.entry(group_name).or_insert_with(Vec::new).push(resource);
        }

        groups
    }
}

impl Default for Nova {
    fn default() -> Self {
        Self::new()
    }
}

/// Nova builder for fluent configuration
pub struct NovaBuilder {
    nova: Nova,
}

impl NovaBuilder {
    pub fn new() -> Self {
        Self {
            nova: Nova::new(),
        }
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.nova = self.nova.with_path(path);
        self
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.nova = self.nova.with_name(name);
        self
    }

    pub fn logo(mut self, logo: impl Into<String>) -> Self {
        self.nova = self.nova.with_logo(logo);
        self
    }

    pub fn primary_color(mut self, color: impl Into<String>) -> Self {
        self.nova = self.nova.with_primary_color(color);
        self
    }

    pub fn db(mut self, db: DatabaseConnection) -> Self {
        self.nova = self.nova.with_db(db);
        self
    }

    pub fn resource<R: Resource + 'static>(mut self) -> Self {
        self.nova = self.nova.register_resource::<R>();
        self
    }

    pub fn dashboard<D: Dashboard + 'static>(mut self, dashboard: D) -> Self {
        self.nova = self.nova.register_dashboard(dashboard);
        self
    }

    pub fn build(self) -> Nova {
        self.nova
    }
}

impl Default for NovaBuilder {
    fn default() -> Self {
        Self::new()
    }
}
