//! Filters for Nova resources
//!
//! Filters allow users to narrow down the list of resources.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Filter option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterOption {
    pub value: String,
    pub label: String,
}

impl FilterOption {
    pub fn new(value: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            label: label.into(),
        }
    }
}

/// Filter trait
pub trait Filter: Send + Sync {
    /// Get filter name
    fn name(&self) -> &str;

    /// Get filter component type
    fn component(&self) -> FilterComponent {
        FilterComponent::Select
    }

    /// Get available options for select filters
    fn options(&self) -> Vec<FilterOption> {
        vec![]
    }

    /// Apply filter to a query builder
    /// Note: This returns a generic representation that will be converted to actual SQL
    fn apply(&self, value: &str) -> FilterCondition;

    /// Serialize filter for JSON API
    fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "component": self.component(),
            "options": self.options(),
        })
    }
}

/// Filter component types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterComponent {
    Select,
    Boolean,
    Date,
    DateRange,
}

/// Filter condition that can be applied to queries
#[derive(Debug, Clone)]
pub enum FilterCondition {
    Equals { field: String, value: String },
    In { field: String, values: Vec<String> },
    Between { field: String, start: String, end: String },
    GreaterThan { field: String, value: String },
    LessThan { field: String, value: String },
    Like { field: String, pattern: String },
    Custom { sql: String, bindings: Vec<Value> },
}

/// Standard filter implementations

/// Select filter - filter by specific values
pub struct SelectFilter {
    pub name: String,
    pub field: String,
    pub options: Vec<FilterOption>,
}

impl SelectFilter {
    pub fn new(name: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field: field.into(),
            options: vec![],
        }
    }

    pub fn options(mut self, options: Vec<FilterOption>) -> Self {
        self.options = options;
        self
    }

    pub fn option(mut self, value: impl Into<String>, label: impl Into<String>) -> Self {
        self.options.push(FilterOption::new(value, label));
        self
    }
}

impl Filter for SelectFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> FilterComponent {
        FilterComponent::Select
    }

    fn options(&self) -> Vec<FilterOption> {
        self.options.clone()
    }

    fn apply(&self, value: &str) -> FilterCondition {
        FilterCondition::Equals {
            field: self.field.clone(),
            value: value.to_string(),
        }
    }
}

/// Boolean filter - filter by true/false
pub struct BooleanFilter {
    pub name: String,
    pub field: String,
    pub true_label: String,
    pub false_label: String,
}

impl BooleanFilter {
    pub fn new(name: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field: field.into(),
            true_label: "Yes".to_string(),
            false_label: "No".to_string(),
        }
    }

    pub fn labels(mut self, true_label: impl Into<String>, false_label: impl Into<String>) -> Self {
        self.true_label = true_label.into();
        self.false_label = false_label.into();
        self
    }
}

impl Filter for BooleanFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> FilterComponent {
        FilterComponent::Boolean
    }

    fn options(&self) -> Vec<FilterOption> {
        vec![
            FilterOption::new("true", &self.true_label),
            FilterOption::new("false", &self.false_label),
        ]
    }

    fn apply(&self, value: &str) -> FilterCondition {
        FilterCondition::Equals {
            field: self.field.clone(),
            value: value.to_string(),
        }
    }
}

/// Date filter - filter by specific date
pub struct DateFilter {
    pub name: String,
    pub field: String,
}

impl DateFilter {
    pub fn new(name: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field: field.into(),
        }
    }
}

impl Filter for DateFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> FilterComponent {
        FilterComponent::Date
    }

    fn apply(&self, value: &str) -> FilterCondition {
        FilterCondition::Equals {
            field: self.field.clone(),
            value: value.to_string(),
        }
    }
}

/// Date range filter - filter by date range
pub struct DateRangeFilter {
    pub name: String,
    pub field: String,
}

impl DateRangeFilter {
    pub fn new(name: impl Into<String>, field: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field: field.into(),
        }
    }
}

impl Filter for DateRangeFilter {
    fn name(&self) -> &str {
        &self.name
    }

    fn component(&self) -> FilterComponent {
        FilterComponent::DateRange
    }

    fn apply(&self, value: &str) -> FilterCondition {
        // Parse date range (format: "start,end")
        let parts: Vec<&str> = value.split(',').collect();
        if parts.len() == 2 {
            FilterCondition::Between {
                field: self.field.clone(),
                start: parts[0].to_string(),
                end: parts[1].to_string(),
            }
        } else {
            FilterCondition::Equals {
                field: self.field.clone(),
                value: value.to_string(),
            }
        }
    }
}

/// Preset filters that can be applied quickly

/// Active/Inactive filter
pub struct ActiveFilter;

impl Filter for ActiveFilter {
    fn name(&self) -> &str {
        "Status"
    }

    fn component(&self) -> FilterComponent {
        FilterComponent::Select
    }

    fn options(&self) -> Vec<FilterOption> {
        vec![
            FilterOption::new("active", "Active"),
            FilterOption::new("inactive", "Inactive"),
        ]
    }

    fn apply(&self, value: &str) -> FilterCondition {
        let is_active = value == "active";
        FilterCondition::Equals {
            field: "active".to_string(),
            value: is_active.to_string(),
        }
    }
}

/// Trashed filter (for soft deletes)
pub struct TrashedFilter;

impl Filter for TrashedFilter {
    fn name(&self) -> &str {
        "Trashed"
    }

    fn component(&self) -> FilterComponent {
        FilterComponent::Select
    }

    fn options(&self) -> Vec<FilterOption> {
        vec![
            FilterOption::new("with", "With Trashed"),
            FilterOption::new("only", "Only Trashed"),
            FilterOption::new("without", "Without Trashed"),
        ]
    }

    fn apply(&self, value: &str) -> FilterCondition {
        match value {
            "only" => FilterCondition::Custom {
                sql: "deleted_at IS NOT NULL".to_string(),
                bindings: vec![],
            },
            "without" => FilterCondition::Custom {
                sql: "deleted_at IS NULL".to_string(),
                bindings: vec![],
            },
            _ => FilterCondition::Custom {
                sql: "1=1".to_string(), // Include all
                bindings: vec![],
            },
        }
    }
}

/// Helper to create filters easily
#[macro_export]
macro_rules! filter {
    (
        name: $name:expr,
        field: $field:expr,
        options: [$($value:expr => $label:expr),* $(,)?]
    ) => {
        {
            $crate::filter::SelectFilter::new($name, $field)
                .options(vec![
                    $(
                        $crate::filter::FilterOption::new($value, $label),
                    )*
                ])
        }
    };
}
