//! Dashboard and widget system

use crate::config::AdminConfig;
use crate::widgets::{Widget, WidgetType};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

/// Dashboard with customizable widgets
pub struct Dashboard {
    config: AdminConfig,
    widgets: RwLock<Vec<Arc<dyn Widget>>>,
}

impl Dashboard {
    pub fn new(config: AdminConfig) -> Self {
        Self {
            config,
            widgets: RwLock::new(Vec::new()),
        }
    }

    pub fn add_widget(&self, widget: Arc<dyn Widget>) {
        let mut widgets = self.widgets.write().unwrap();
        widgets.push(widget);
    }

    pub async fn render(&self) -> anyhow::Result<DashboardData> {
        let widgets = {
            let widgets_lock = self.widgets.read().unwrap();
            widgets_lock.clone()
        };
        let mut rendered = Vec::new();

        for widget in widgets.iter() {
            let data = widget.render().await?;
            rendered.push(data);
        }

        Ok(DashboardData {
            title: self.config.title.clone(),
            widgets: rendered,
        })
    }

    pub fn config(&self) -> &AdminConfig {
        &self.config
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub title: String,
    pub widgets: Vec<WidgetData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WidgetData {
    pub id: String,
    pub title: String,
    pub widget_type: WidgetType,
    pub data: serde_json::Value,
}
