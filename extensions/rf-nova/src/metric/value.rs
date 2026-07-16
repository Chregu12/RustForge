//! Value Metric - displays a single numeric value
//!
//! Shows a single number, optionally with comparison to previous period.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Value metric result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: f64,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub format: Option<String>,
    pub previous: Option<f64>,
    pub increase: Option<f64>,
    pub increase_percentage: Option<f64>,
}

impl MetricValue {
    pub fn new(value: f64) -> Self {
        Self {
            value,
            prefix: None,
            suffix: None,
            format: None,
            previous: None,
            increase: None,
            increase_percentage: None,
        }
    }

    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn format(mut self, format: impl Into<String>) -> Self {
        self.format = Some(format.into());
        self
    }

    pub fn previous(mut self, previous: f64) -> Self {
        let increase = self.value - previous;
        let increase_percentage = if previous != 0.0 {
            (increase / previous) * 100.0
        } else {
            0.0
        };

        self.previous = Some(previous);
        self.increase = Some(increase);
        self.increase_percentage = Some(increase_percentage);
        self
    }

    pub fn currency(mut self) -> Self {
        self.prefix = Some("$".to_string());
        self.format = Some("0,0.00".to_string());
        self
    }

    pub fn percentage(mut self) -> Self {
        self.suffix = Some("%".to_string());
        self.format = Some("0.00".to_string());
        self
    }
}

/// Value metric trait
#[async_trait]
pub trait ValueMetric: Send + Sync {
    /// Get metric name
    fn name(&self) -> &str;

    /// Get metric URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Calculate the metric value
    async fn calculate(&self) -> Result<MetricValue, MetricError>;

    /// Width of the metric card (1/3, 1/2, or full)
    fn width(&self) -> MetricWidth {
        MetricWidth::OneThird
    }

    /// Serialize metric for JSON API
    fn to_json(&self) -> JsonValue {
        serde_json::json!({
            "type": "value",
            "name": self.name(),
            "uri_key": self.uri_key(),
            "width": self.width(),
        })
    }
}

/// Metric errors
#[derive(Debug, thiserror::Error)]
pub enum MetricError {
    #[error("Database error: {0}")]
    Database(#[from] sea_orm::DbErr),

    #[error("Calculation error: {0}")]
    Calculation(String),
}

/// Metric card width
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricWidth {
    OneThird,
    OneHalf,
    Full,
}

/// Helper macro to create value metrics
#[macro_export]
macro_rules! value_metric {
    (
        name: $name:expr,
        calculate: || $body:expr
    ) => {
        {
            struct CustomValueMetric;

            #[async_trait::async_trait]
            impl $crate::metric::ValueMetric for CustomValueMetric {
                fn name(&self) -> &str {
                    $name
                }

                async fn calculate(&self) -> Result<$crate::metric::MetricValue, $crate::metric::MetricError> {
                    $body
                }
            }

            Box::new(CustomValueMetric) as Box<dyn $crate::metric::ValueMetric>
        }
    };
}
