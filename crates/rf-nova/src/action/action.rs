//! Actions for Nova resources
//!
//! Actions allow you to perform tasks on one or more resources.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Result type for actions
pub type ActionResult = Result<ActionResponse, ActionError>;

/// Action errors
#[derive(Debug, thiserror::Error)]
pub enum ActionError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Action failed: {0}")]
    Failed(String),
}

impl From<sea_orm::DbErr> for ActionError {
    fn from(err: sea_orm::DbErr) -> Self {
        ActionError::Database(err.to_string())
    }
}

/// Action response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResponse {
    pub success: bool,
    pub message: String,
    pub data: Option<Value>,
    #[serde(rename = "type")]
    pub response_type: ActionResponseType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionResponseType {
    Success,
    Danger,
    Warning,
    Info,
}

impl ActionResponse {
    pub fn success(message: impl Into<String>) -> ActionResult {
        Ok(Self {
            success: true,
            message: message.into(),
            data: None,
            response_type: ActionResponseType::Success,
        })
    }

    pub fn danger(message: impl Into<String>) -> ActionResult {
        Ok(Self {
            success: false,
            message: message.into(),
            data: None,
            response_type: ActionResponseType::Danger,
        })
    }

    pub fn warning(message: impl Into<String>) -> ActionResult {
        Ok(Self {
            success: true,
            message: message.into(),
            data: None,
            response_type: ActionResponseType::Warning,
        })
    }

    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }
}

/// Fields that can be added to actions for user input
#[derive(Debug, Clone)]
pub struct ActionField {
    pub name: String,
    pub label: String,
    pub field_type: ActionFieldType,
    pub rules: Vec<String>,
    pub help: Option<String>,
    pub default: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActionFieldType {
    Text,
    Textarea,
    Select,
    Boolean,
    Number,
    Date,
    DateTime,
}

impl ActionField {
    pub fn text(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            field_type: ActionFieldType::Text,
            rules: vec![],
            help: None,
            default: None,
        }
    }

    pub fn textarea(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            field_type: ActionFieldType::Textarea,
            rules: vec![],
            help: None,
            default: None,
        }
    }

    pub fn select(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            field_type: ActionFieldType::Select,
            rules: vec![],
            help: None,
            default: None,
        }
    }

    pub fn boolean(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            field_type: ActionFieldType::Boolean,
            rules: vec![],
            help: None,
            default: None,
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn rules(mut self, rules: impl Into<String>) -> Self {
        self.rules = rules.into().split('|').map(String::from).collect();
        self
    }

    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    pub fn default(mut self, value: Value) -> Self {
        self.default = Some(value);
        self
    }

    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name,
            "label": self.label,
            "type": self.field_type,
            "rules": self.rules,
            "help": self.help,
            "default": self.default,
        })
    }
}

/// Container for action field values submitted by the user
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ActionFields {
    fields: HashMap<String, Value>,
}

impl ActionFields {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
        }
    }

    pub fn from_map(fields: HashMap<String, Value>) -> Self {
        Self { fields }
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.fields.get(name)
    }

    pub fn get_string(&self, name: &str) -> Option<String> {
        self.get(name)?.as_str().map(String::from)
    }

    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.get(name)?.as_bool()
    }

    pub fn get_i64(&self, name: &str) -> Option<i64> {
        self.get(name)?.as_i64()
    }

    pub fn get_f64(&self, name: &str) -> Option<f64> {
        self.get(name)?.as_f64()
    }
}

/// Core Action trait
#[async_trait]
pub trait Action: Send + Sync {
    /// Get action name
    fn name(&self) -> &str;

    /// Get action URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Whether this action is destructive (shows red button)
    fn destructive(&self) -> bool {
        false
    }

    /// Whether this action can be run on the index page
    fn show_on_index(&self) -> bool {
        true
    }

    /// Whether this action can be run on the detail page
    fn show_on_detail(&self) -> bool {
        true
    }

    /// Whether this action can be run standalone (without selecting resources)
    fn standalone(&self) -> bool {
        false
    }

    /// Get fields that the action needs from the user
    fn fields(&self) -> Vec<ActionField> {
        vec![]
    }

    /// Handle the action
    async fn handle(&self, models: Vec<Value>, fields: ActionFields) -> ActionResult;

    /// Serialize action for JSON API
    fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "uri_key": self.uri_key(),
            "destructive": self.destructive(),
            "show_on_index": self.show_on_index(),
            "show_on_detail": self.show_on_detail(),
            "standalone": self.standalone(),
            "fields": self.fields().iter().map(|f| f.to_json()).collect::<Vec<_>>(),
        })
    }
}

/// Helper macro to create simple actions
#[macro_export]
macro_rules! action {
    (
        name: $name:expr,
        $(destructive: $destructive:expr,)?
        handle: |$models:ident, $fields:ident| $body:expr
    ) => {
        {
            struct CustomAction;

            #[async_trait::async_trait]
            impl $crate::action::Action for CustomAction {
                fn name(&self) -> &str {
                    $name
                }

                $(
                fn destructive(&self) -> bool {
                    $destructive
                }
                )?

                async fn handle(
                    &self,
                    $models: Vec<serde_json::Value>,
                    $fields: $crate::action::ActionFields,
                ) -> $crate::action::ActionResult {
                    $body
                }
            }

            Box::new(CustomAction) as Box<dyn $crate::action::Action>
        }
    };
}

/// Standard actions that come with Nova

/// Export action - exports selected resources
pub struct ExportAction {
    pub name: String,
    pub format: ExportFormat,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    Json,
    Csv,
}

impl ExportAction {
    pub fn json() -> Self {
        Self {
            name: "Export as JSON".to_string(),
            format: ExportFormat::Json,
        }
    }

    pub fn csv() -> Self {
        Self {
            name: "Export as CSV".to_string(),
            format: ExportFormat::Csv,
        }
    }
}

#[async_trait]
impl Action for ExportAction {
    fn name(&self) -> &str {
        &self.name
    }

    fn show_on_index(&self) -> bool {
        true
    }

    fn show_on_detail(&self) -> bool {
        false
    }

    async fn handle(&self, models: Vec<Value>, _fields: ActionFields) -> ActionResult {
        match self.format {
            ExportFormat::Json => {
                let json = serde_json::to_string_pretty(&models)
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                ActionResponse::success("Exported successfully").map(|r| r.with_data(
                    serde_json::json!({
                        "content": json,
                        "filename": "export.json",
                        "mime_type": "application/json",
                    })
                ))
            }
            ExportFormat::Csv => {
                // Convert to CSV
                let mut wtr = csv::Writer::from_writer(vec![]);

                for model in models {
                    if let Value::Object(map) = model {
                        let values: Vec<String> = map.values().map(|v| format!("{}", v)).collect();
                        wtr.write_record(&values)
                            .map_err(|e| ActionError::Failed(e.to_string()))?;
                    }
                }

                let data = wtr
                    .into_inner()
                    .map_err(|e| ActionError::Failed(e.to_string()))?;
                let csv = String::from_utf8(data)
                    .map_err(|e| ActionError::Failed(e.to_string()))?;

                ActionResponse::success("Exported successfully").map(|r| r.with_data(
                    serde_json::json!({
                        "content": csv,
                        "filename": "export.csv",
                        "mime_type": "text/csv",
                    })
                ))
            }
        }
    }
}

/// Download action - triggers file download
pub struct DownloadAction {
    pub name: String,
}

impl DownloadAction {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

#[async_trait]
impl Action for DownloadAction {
    fn name(&self) -> &str {
        &self.name
    }

    async fn handle(&self, models: Vec<Value>, _fields: ActionFields) -> ActionResult {
        ActionResponse::success(format!("Downloaded {} items", models.len()))
    }
}
