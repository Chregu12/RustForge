//! Field types for Nova resources
//!
//! Fields define how model attributes are displayed and edited in the admin panel.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Field visibility context
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldContext {
    /// Shown on the index (list) page
    Index,
    /// Shown on the detail (show) page
    Detail,
    /// Shown on the create form
    Create,
    /// Shown on the update form
    Update,
}

/// Base field trait that all field types implement
pub trait Field: Send + Sync {
    /// Get the field name (database column)
    fn name(&self) -> &str;

    /// Get the field label (human-readable)
    fn label(&self) -> &str;

    /// Check if field is visible in given context
    fn is_visible(&self, context: FieldContext) -> bool;

    /// Check if field is sortable
    fn is_sortable(&self) -> bool {
        false
    }

    /// Check if field is searchable
    fn is_searchable(&self) -> bool {
        false
    }

    /// Get validation rules
    fn validation_rules(&self) -> Vec<&str> {
        vec![]
    }

    /// Get help text
    fn help_text(&self) -> Option<&str> {
        None
    }

    /// Serialize field for JSON API
    fn to_json(&self) -> Value;

    /// Get the field type identifier
    fn field_type(&self) -> &str;
}

/// ID field - Primary key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ID {
    pub name: String,
    pub label: String,
    #[serde(skip)]
    pub visibility: Vec<FieldContext>,
}

impl ID {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            visibility: vec![FieldContext::Index, FieldContext::Detail],
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
}

impl Field for ID {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "id",
            "name": self.name,
            "label": self.label,
        })
    }

    fn field_type(&self) -> &str {
        "id"
    }
}

/// Text field
#[derive(Debug, Clone)]
pub struct Text {
    pub name: String,
    pub label: String,
    pub sortable: bool,
    pub searchable: bool,
    pub rules: Vec<String>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub visibility: Vec<FieldContext>,
}

impl Text {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            sortable: false,
            searchable: false,
            rules: vec![],
            help: None,
            placeholder: None,
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn searchable(mut self) -> Self {
        self.searchable = true;
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

    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    pub fn hide_on_index(mut self) -> Self {
        self.visibility.retain(|&ctx| ctx != FieldContext::Index);
        self
    }

    pub fn hide_on_detail(mut self) -> Self {
        self.visibility.retain(|&ctx| ctx != FieldContext::Detail);
        self
    }

    pub fn only_on_forms(mut self) -> Self {
        self.visibility = vec![FieldContext::Create, FieldContext::Update];
        self
    }
}

impl Field for Text {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn is_sortable(&self) -> bool {
        self.sortable
    }

    fn is_searchable(&self) -> bool {
        self.searchable
    }

    fn validation_rules(&self) -> Vec<&str> {
        self.rules.iter().map(|s| s.as_str()).collect()
    }

    fn help_text(&self) -> Option<&str> {
        self.help.as_deref()
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "text",
            "name": self.name,
            "label": self.label,
            "sortable": self.sortable,
            "searchable": self.searchable,
            "rules": self.rules,
            "help": self.help,
            "placeholder": self.placeholder,
        })
    }

    fn field_type(&self) -> &str {
        "text"
    }
}

/// Textarea field
#[derive(Debug, Clone)]
pub struct Textarea {
    pub name: String,
    pub label: String,
    pub rows: u32,
    pub rules: Vec<String>,
    pub help: Option<String>,
    pub placeholder: Option<String>,
    pub visibility: Vec<FieldContext>,
}

impl Textarea {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            rows: 4,
            rules: vec![],
            help: None,
            placeholder: None,
            visibility: vec![
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn rows(mut self, rows: u32) -> Self {
        self.rows = rows;
        self
    }

    pub fn rules(mut self, rules: impl Into<String>) -> Self {
        self.rules = rules.into().split('|').map(String::from).collect();
        self
    }
}

impl Field for Textarea {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn validation_rules(&self) -> Vec<&str> {
        self.rules.iter().map(|s| s.as_str()).collect()
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "textarea",
            "name": self.name,
            "label": self.label,
            "rows": self.rows,
            "rules": self.rules,
        })
    }

    fn field_type(&self) -> &str {
        "textarea"
    }
}

/// Password field
#[derive(Debug, Clone)]
pub struct Password {
    pub name: String,
    pub label: String,
    pub rules: Vec<String>,
    pub visibility: Vec<FieldContext>,
}

impl Password {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            rules: vec![],
            visibility: vec![FieldContext::Create, FieldContext::Update],
        }
    }

    pub fn rules(mut self, rules: impl Into<String>) -> Self {
        self.rules = rules.into().split('|').map(String::from).collect();
        self
    }
}

impl Field for Password {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn validation_rules(&self) -> Vec<&str> {
        self.rules.iter().map(|s| s.as_str()).collect()
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "password",
            "name": self.name,
            "label": self.label,
            "rules": self.rules,
        })
    }

    fn field_type(&self) -> &str {
        "password"
    }
}

/// Boolean field (checkbox/toggle)
#[derive(Debug, Clone)]
pub struct Boolean {
    pub name: String,
    pub label: String,
    pub sortable: bool,
    pub true_value: String,
    pub false_value: String,
    pub visibility: Vec<FieldContext>,
}

impl Boolean {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            sortable: false,
            true_value: "Yes".to_string(),
            false_value: "No".to_string(),
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn labels(mut self, true_label: impl Into<String>, false_label: impl Into<String>) -> Self {
        self.true_value = true_label.into();
        self.false_value = false_label.into();
        self
    }
}

impl Field for Boolean {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn is_sortable(&self) -> bool {
        self.sortable
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "boolean",
            "name": self.name,
            "label": self.label,
            "sortable": self.sortable,
            "true_value": self.true_value,
            "false_value": self.false_value,
        })
    }

    fn field_type(&self) -> &str {
        "boolean"
    }
}

/// DateTime field
#[derive(Debug, Clone)]
pub struct DateTime {
    pub name: String,
    pub label: String,
    pub sortable: bool,
    pub format: String,
    pub visibility: Vec<FieldContext>,
}

impl DateTime {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            sortable: false,
            format: "%Y-%m-%d %H:%M:%S".to_string(),
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = format.into();
        self
    }
}

impl Field for DateTime {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn is_sortable(&self) -> bool {
        self.sortable
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "datetime",
            "name": self.name,
            "label": self.label,
            "sortable": self.sortable,
            "format": self.format,
        })
    }

    fn field_type(&self) -> &str {
        "datetime"
    }
}

/// Number field
#[derive(Debug, Clone)]
pub struct Number {
    pub name: String,
    pub label: String,
    pub sortable: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub rules: Vec<String>,
    pub visibility: Vec<FieldContext>,
}

impl Number {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            sortable: false,
            min: None,
            max: None,
            step: None,
            rules: vec![],
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn min(mut self, min: f64) -> Self {
        self.min = Some(min);
        self
    }

    pub fn max(mut self, max: f64) -> Self {
        self.max = Some(max);
        self
    }

    pub fn step(mut self, step: f64) -> Self {
        self.step = Some(step);
        self
    }
}

impl Field for Number {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn is_sortable(&self) -> bool {
        self.sortable
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "number",
            "name": self.name,
            "label": self.label,
            "sortable": self.sortable,
            "min": self.min,
            "max": self.max,
            "step": self.step,
        })
    }

    fn field_type(&self) -> &str {
        "number"
    }
}

/// Select field (dropdown)
#[derive(Debug, Clone)]
pub struct Select {
    pub name: String,
    pub label: String,
    pub options: Vec<SelectOption>,
    pub searchable: bool,
    pub rules: Vec<String>,
    pub visibility: Vec<FieldContext>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

impl SelectOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

impl Select {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            options: vec![],
            searchable: false,
            rules: vec![],
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn options(mut self, options: Vec<SelectOption>) -> Self {
        self.options = options;
        self
    }

    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }
}

impl Field for Select {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "select",
            "name": self.name,
            "label": self.label,
            "options": self.options,
            "searchable": self.searchable,
        })
    }

    fn field_type(&self) -> &str {
        "select"
    }
}

/// BelongsTo relationship field
#[derive(Debug, Clone)]
pub struct BelongsTo {
    pub name: String,
    pub label: String,
    pub resource: String,
    pub foreign_key: String,
    pub display_field: String,
    pub searchable: bool,
    pub visibility: Vec<FieldContext>,
}

impl BelongsTo {
    pub fn new(name: impl Into<String>, resource: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            foreign_key: format!("{}_id", name),
            name,
            resource: resource.into(),
            display_field: "name".to_string(),
            searchable: false,
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn foreign_key(mut self, key: impl Into<String>) -> Self {
        self.foreign_key = key.into();
        self
    }

    pub fn display(mut self, field: impl Into<String>) -> Self {
        self.display_field = field.into();
        self
    }

    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }
}

impl Field for BelongsTo {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "belongsTo",
            "name": self.name,
            "label": self.label,
            "resource": self.resource,
            "foreign_key": self.foreign_key,
            "display_field": self.display_field,
            "searchable": self.searchable,
        })
    }

    fn field_type(&self) -> &str {
        "belongsTo"
    }
}

/// HasMany relationship field
#[derive(Debug, Clone)]
pub struct HasMany {
    pub name: String,
    pub label: String,
    pub resource: String,
    pub foreign_key: String,
    pub visibility: Vec<FieldContext>,
}

impl HasMany {
    pub fn new(name: impl Into<String>, resource: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            resource: resource.into(),
            foreign_key: "id".to_string(),
            visibility: vec![FieldContext::Detail],
        }
    }

    pub fn foreign_key(mut self, key: impl Into<String>) -> Self {
        self.foreign_key = key.into();
        self
    }
}

impl Field for HasMany {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "hasMany",
            "name": self.name,
            "label": self.label,
            "resource": self.resource,
            "foreign_key": self.foreign_key,
        })
    }

    fn field_type(&self) -> &str {
        "hasMany"
    }
}

/// File upload field
#[derive(Debug, Clone)]
pub struct File {
    pub name: String,
    pub label: String,
    pub disk: String,
    pub path: Option<String>,
    pub accept: Vec<String>,
    pub max_size: Option<usize>,
    pub visibility: Vec<FieldContext>,
}

impl File {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            disk: "local".to_string(),
            path: None,
            accept: vec![],
            max_size: None,
            visibility: vec![
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn disk(mut self, disk: impl Into<String>) -> Self {
        self.disk = disk.into();
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn accept(mut self, types: Vec<String>) -> Self {
        self.accept = types;
        self
    }

    pub fn max_size(mut self, bytes: usize) -> Self {
        self.max_size = Some(bytes);
        self
    }
}

impl Field for File {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "file",
            "name": self.name,
            "label": self.label,
            "disk": self.disk,
            "path": self.path,
            "accept": self.accept,
            "max_size": self.max_size,
        })
    }

    fn field_type(&self) -> &str {
        "file"
    }
}

/// Image upload field (extends File with preview)
#[derive(Debug, Clone)]
pub struct Image {
    pub name: String,
    pub label: String,
    pub disk: String,
    pub path: Option<String>,
    pub max_size: Option<usize>,
    pub preview_width: u32,
    pub preview_height: u32,
    pub visibility: Vec<FieldContext>,
}

impl Image {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            disk: "local".to_string(),
            path: None,
            max_size: None,
            preview_width: 200,
            preview_height: 200,
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn preview(mut self, width: u32, height: u32) -> Self {
        self.preview_width = width;
        self.preview_height = height;
        self
    }
}

impl Field for Image {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "image",
            "name": self.name,
            "label": self.label,
            "disk": self.disk,
            "path": self.path,
            "max_size": self.max_size,
            "preview_width": self.preview_width,
            "preview_height": self.preview_height,
        })
    }

    fn field_type(&self) -> &str {
        "image"
    }
}

/// Email field (specialized text field)
#[derive(Debug, Clone)]
pub struct Email {
    pub name: String,
    pub label: String,
    pub sortable: bool,
    pub searchable: bool,
    pub rules: Vec<String>,
    pub visibility: Vec<FieldContext>,
}

impl Email {
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        Self {
            label: name.clone(),
            name,
            sortable: false,
            searchable: false,
            rules: vec!["email".to_string()],
            visibility: vec![
                FieldContext::Index,
                FieldContext::Detail,
                FieldContext::Create,
                FieldContext::Update,
            ],
        }
    }

    pub fn sortable(mut self) -> Self {
        self.sortable = true;
        self
    }

    pub fn searchable(mut self) -> Self {
        self.searchable = true;
        self
    }

    pub fn rules(mut self, rules: impl Into<String>) -> Self {
        self.rules = rules.into().split('|').map(String::from).collect();
        if !self.rules.contains(&"email".to_string()) {
            self.rules.push("email".to_string());
        }
        self
    }
}

impl Field for Email {
    fn name(&self) -> &str {
        &self.name
    }

    fn label(&self) -> &str {
        &self.label
    }

    fn is_visible(&self, context: FieldContext) -> bool {
        self.visibility.contains(&context)
    }

    fn is_sortable(&self) -> bool {
        self.sortable
    }

    fn is_searchable(&self) -> bool {
        self.searchable
    }

    fn validation_rules(&self) -> Vec<&str> {
        self.rules.iter().map(|s| s.as_str()).collect()
    }

    fn to_json(&self) -> Value {
        serde_json::json!({
            "type": "email",
            "name": self.name,
            "label": self.label,
            "sortable": self.sortable,
            "searchable": self.searchable,
            "rules": self.rules,
        })
    }

    fn field_type(&self) -> &str {
        "email"
    }
}
