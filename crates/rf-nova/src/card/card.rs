//! Cards for Nova dashboards
//!
//! Cards are custom widgets that can be displayed on dashboards.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::metric::value::MetricWidth;

/// Card trait
#[async_trait]
pub trait Card: Send + Sync {
    /// Get card name
    fn name(&self) -> &str;

    /// Get card URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Get the Vue component name for this card
    fn component(&self) -> &str;

    /// Get data to pass to the component
    async fn data(&self) -> Result<Value, CardError>;

    /// Width of the card
    fn width(&self) -> MetricWidth {
        MetricWidth::OneThird
    }

    /// Whether the card should refresh automatically
    fn refresh_interval(&self) -> Option<u32> {
        None
    }

    /// Serialize card for JSON API
    fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "uri_key": self.uri_key(),
            "component": self.component(),
            "width": self.width(),
            "refresh_interval": self.refresh_interval(),
        })
    }
}

/// Card errors
#[derive(Debug, thiserror::Error)]
pub enum CardError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Data error: {0}")]
    Data(String),
}

/// Help card - displays helpful information
pub struct HelpCard {
    pub name: String,
    pub title: String,
    pub content: String,
    pub width: MetricWidth,
}

impl HelpCard {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            title: name.clone(),
            name,
            content: String::new(),
            width: MetricWidth::OneThird,
        }
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    pub fn width(mut self, width: MetricWidth) -> Self {
        self.width = width;
        self
    }
}

#[async_trait]
impl Card for HelpCard {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> &str {
        "help-card"
    }

    async fn data(&self) -> Result<Value, CardError> {
        Ok(serde_json::json!({
            "title": self.title,
            "content": self.content,
        }))
    }

    fn width(&self) -> MetricWidth {
        self.width
    }
}

/// Table card - displays data in a table
pub struct TableCard {
    pub name: String,
    pub columns: Vec<TableColumn>,
    pub rows: Vec<Value>,
    pub width: MetricWidth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableColumn {
    pub key: String,
    pub label: String,
    pub sortable: bool,
}

impl TableColumn {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            sortable: false,
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }
}

impl TableCard {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            columns: vec![],
            rows: vec![],
            width: MetricWidth::Full,
        }
    }

    pub fn columns(mut self, columns: Vec<TableColumn>) -> Self {
        self.columns = columns;
        self
    }

    pub fn rows(mut self, rows: Vec<Value>) -> Self {
        self.rows = rows;
        self
    }

    pub fn width(mut self, width: MetricWidth) -> Self {
        self.width = width;
        self
    }
}

#[async_trait]
impl Card for TableCard {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> &str {
        "table-card"
    }

    async fn data(&self) -> Result<Value, CardError> {
        Ok(serde_json::json!({
            "columns": self.columns,
            "rows": self.rows,
        }))
    }

    fn width(&self) -> MetricWidth {
        self.width
    }
}

/// Progress card - shows progress toward a goal
pub struct ProgressCard {
    pub name: String,
    pub current: f64,
    pub goal: f64,
    pub label: String,
    pub width: MetricWidth,
}

impl ProgressCard {
    pub fn new(name: impl Into<String>, current: f64, goal: f64) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            current,
            goal,
            width: MetricWidth::OneThird,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn width(mut self, width: MetricWidth) -> Self {
        self.width = width;
        self
    }

    pub fn percentage(&self) -> f64 {
        if self.goal > 0.0 {
            (self.current / self.goal) * 100.0
        } else {
            0.0
        }
    }
}

#[async_trait]
impl Card for ProgressCard {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> &str {
        "progress-card"
    }

    async fn data(&self) -> Result<Value, CardError> {
        Ok(serde_json::json!({
            "label": self.label,
            "current": self.current,
            "goal": self.goal,
            "percentage": self.percentage(),
        }))
    }

    fn width(&self) -> MetricWidth {
        self.width
    }
}

/// Helper macro to create custom cards
#[macro_export]
macro_rules! card {
    (
        name: $name:expr,
        component: $component:expr,
        data: || $body:expr
    ) => {
        {
            struct CustomCard;

            #[async_trait::async_trait]
            impl $crate::card::Card for CustomCard {
                fn name(&self) -> &str {
                    $name
                }

                fn component(&self) -> &str {
                    $component
                }

                async fn data(&self) -> Result<serde_json::Value, $crate::card::CardError> {
                    $body
                }
            }

            Box::new(CustomCard) as Box<dyn $crate::card::Card>
        }
    };
}
