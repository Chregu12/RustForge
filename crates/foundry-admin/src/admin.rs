//! Core admin panel implementation

use crate::config::AdminConfig;
use crate::dashboard::Dashboard;
use crate::resource::AdminResource;
use crate::templates::TemplateEngine;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Main admin panel
pub struct AdminPanel {
    config: AdminConfig,
    resources: RwLock<HashMap<String, Arc<dyn AdminResource>>>,
    dashboard: Dashboard,
    templates: TemplateEngine,
}

impl AdminPanel {
    pub fn new(config: AdminConfig) -> Self {
        Self {
            config: config.clone(),
            resources: RwLock::new(HashMap::new()),
            dashboard: Dashboard::new(config.clone()),
            templates: TemplateEngine::new(config),
        }
    }

    pub fn builder() -> AdminPanelBuilder {
        AdminPanelBuilder::default()
    }

    pub fn config(&self) -> &AdminConfig {
        &self.config
    }

    pub fn dashboard(&self) -> &Dashboard {
        &self.dashboard
    }

    pub fn templates(&self) -> &TemplateEngine {
        &self.templates
    }

    pub fn register_resource(&self, name: impl Into<String>, resource: Arc<dyn AdminResource>) {
        let mut resources = self.resources.write().unwrap();
        resources.insert(name.into(), resource);
    }

    pub fn get_resource(&self, name: &str) -> Option<Arc<dyn AdminResource>> {
        let resources = self.resources.read().unwrap();
        resources.get(name).cloned()
    }

    pub fn list_resources(&self) -> Vec<String> {
        let resources = self.resources.read().unwrap();
        resources.keys().cloned().collect()
    }
}

/// Builder for AdminPanel
#[derive(Default)]
pub struct AdminPanelBuilder {
    config: Option<AdminConfig>,
    resources: Vec<(String, Arc<dyn AdminResource>)>,
}

impl AdminPanelBuilder {
    pub fn config(mut self, config: AdminConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn resource(mut self, name: impl Into<String>, resource: Arc<dyn AdminResource>) -> Self {
        self.resources.push((name.into(), resource));
        self
    }

    pub fn build(self) -> AdminPanel {
        let config = self.config.unwrap_or_default();
        let panel = AdminPanel::new(config);

        for (name, resource) in self.resources {
            panel.register_resource(name, resource);
        }

        panel
    }
}
