//! Partition Metric - displays data as a pie/donut chart
//!
//! Shows how data is distributed across categories.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::value::{MetricError, MetricWidth};

/// Partition metric result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionData {
    pub segments: Vec<PartitionSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionSegment {
    pub label: String,
    pub value: f64,
    pub color: String,
}

impl PartitionData {
    pub fn new() -> Self {
        Self { segments: vec![] }
    }

    pub fn add(mut self, label: impl Into<String>, value: f64, color: impl Into<String>) -> Self {
        self.segments.push(PartitionSegment {
            label: label.into(),
            value,
            color: color.into(),
        });
        self
    }

    pub fn segment(mut self, segment: PartitionSegment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Get total value of all segments
    pub fn total(&self) -> f64 {
        self.segments.iter().map(|s| s.value).sum()
    }

    /// Get percentage for each segment
    pub fn percentages(&self) -> HashMap<String, f64> {
        let total = self.total();
        self.segments
            .iter()
            .map(|s| {
                let percentage = if total > 0.0 {
                    (s.value / total) * 100.0
                } else {
                    0.0
                };
                (s.label.clone(), percentage)
            })
            .collect()
    }
}

impl Default for PartitionData {
    fn default() -> Self {
        Self::new()
    }
}

impl PartitionSegment {
    pub fn new(label: impl Into<String>, value: f64, color: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value,
            color: color.into(),
        }
    }
}

/// Partition metric trait
#[async_trait]
pub trait PartitionMetric: Send + Sync {
    /// Get metric name
    fn name(&self) -> &str;

    /// Get metric URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Calculate the partition data
    async fn calculate(&self) -> Result<PartitionData, MetricError>;

    /// Width of the metric card
    fn width(&self) -> MetricWidth {
        MetricWidth::OneThird
    }

    /// Chart type (pie or donut)
    fn chart_type(&self) -> PartitionChartType {
        PartitionChartType::Donut
    }

    /// Serialize metric for JSON API
    fn to_json(&self) -> JsonValue {
        serde_json::json!({
            "type": "partition",
            "name": self.name(),
            "uri_key": self.uri_key(),
            "width": self.width(),
            "chart_type": self.chart_type(),
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PartitionChartType {
    Pie,
    Donut,
}

/// Preset colors for partition charts
pub struct Colors;

impl Colors {
    pub const BLUE: &'static str = "#4299E1";
    pub const GREEN: &'static str = "#48BB78";
    pub const RED: &'static str = "#F56565";
    pub const YELLOW: &'static str = "#ECC94B";
    pub const PURPLE: &'static str = "#9F7AEA";
    pub const PINK: &'static str = "#ED64A6";
    pub const ORANGE: &'static str = "#ED8936";
    pub const TEAL: &'static str = "#38B2AC";
    pub const CYAN: &'static str = "#0BC5EA";
    pub const GRAY: &'static str = "#A0AEC0";

    /// Get a color from the palette by index
    pub fn palette(index: usize) -> &'static str {
        const PALETTE: &[&str] = &[
            "#4299E1", // BLUE
            "#48BB78", // GREEN
            "#ECC94B", // YELLOW
            "#9F7AEA", // PURPLE
            "#ED64A6", // PINK
            "#ED8936", // ORANGE
            "#38B2AC", // TEAL
            "#0BC5EA", // CYAN
            "#F56565", // RED
            "#A0AEC0", // GRAY
        ];
        PALETTE[index % PALETTE.len()]
    }

    /// Get all colors in the palette
    pub fn all() -> Vec<&'static str> {
        vec![
            Colors::BLUE,
            Colors::GREEN,
            Colors::YELLOW,
            Colors::PURPLE,
            Colors::PINK,
            Colors::ORANGE,
            Colors::TEAL,
            Colors::CYAN,
            Colors::RED,
            Colors::GRAY,
        ]
    }
}

/// Helper macro to create partition metrics
#[macro_export]
macro_rules! partition_metric {
    (
        name: $name:expr,
        calculate: || $body:expr
    ) => {
        {
            struct CustomPartitionMetric;

            #[async_trait::async_trait]
            impl $crate::metric::PartitionMetric for CustomPartitionMetric {
                fn name(&self) -> &str {
                    $name
                }

                async fn calculate(&self) -> Result<$crate::metric::PartitionData, $crate::metric::MetricError> {
                    $body
                }
            }

            Box::new(CustomPartitionMetric) as Box<dyn $crate::metric::PartitionMetric>
        }
    };
}
