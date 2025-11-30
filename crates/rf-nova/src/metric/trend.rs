//! Trend Metric - displays data over time as a line chart
//!
//! Shows how a value changes over time.

use async_trait::async_trait;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

use super::value::{MetricError, MetricWidth};

/// Trend metric result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendData {
    pub data: HashMap<String, f64>,
    pub trend: Option<TrendDirection>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Up,
    Down,
    Flat,
}

impl TrendData {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            trend: None,
        }
    }

    pub fn add(mut self, label: impl Into<String>, value: f64) -> Self {
        self.data.insert(label.into(), value);
        self
    }

    pub fn with_trend(mut self, trend: TrendDirection) -> Self {
        self.trend = Some(trend);
        self
    }

    pub fn calculate_trend(mut self) -> Self {
        let values: Vec<f64> = self.data.values().copied().collect();
        if values.len() >= 2 {
            let first = values[0];
            let last = values[values.len() - 1];
            let diff = last - first;

            self.trend = Some(if diff > 0.0 {
                TrendDirection::Up
            } else if diff < 0.0 {
                TrendDirection::Down
            } else {
                TrendDirection::Flat
            });
        }
        self
    }

    /// Helper to create trend data by days
    pub async fn by_days<F, Fut>(range: DateRange, mut f: F) -> Result<Self, MetricError>
    where
        F: FnMut(NaiveDate) -> Fut,
        Fut: std::future::Future<Output = Result<f64, MetricError>>,
    {
        let mut data = Self::new();
        let mut current = range.start;

        while current <= range.end {
            let value = f(current).await?;
            data = data.add(current.format("%Y-%m-%d").to_string(), value);
            current = current.succ_opt().unwrap();
        }

        Ok(data.calculate_trend())
    }

    /// Helper to create trend data by months
    pub async fn by_months<F, Fut>(range: DateRange, mut f: F) -> Result<Self, MetricError>
    where
        F: FnMut(NaiveDate) -> Fut,
        Fut: std::future::Future<Output = Result<f64, MetricError>>,
    {
        let mut data = Self::new();
        let mut current = range.start;

        while current <= range.end {
            let value = f(current).await?;
            data = data.add(current.format("%Y-%m").to_string(), value);

            // Move to next month using Duration
            current = current + Duration::days(30); // Approximate month
        }

        Ok(data.calculate_trend())
    }
}

impl Default for TrendData {
    fn default() -> Self {
        Self::new()
    }
}

/// Date range for trend calculations
#[derive(Debug, Clone)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Self {
        Self { start, end }
    }

    pub fn last_days(days: i64) -> Self {
        let end = Utc::now().date_naive();
        let start = end - Duration::days(days - 1);
        Self { start, end }
    }

    pub fn last_weeks(weeks: i64) -> Self {
        Self::last_days(weeks * 7)
    }

    pub fn last_months(months: i64) -> Self {
        let end = Utc::now().date_naive();
        let start = end - Duration::days(months * 30); // Approximate
        Self { start, end }
    }

    pub fn this_month() -> Self {
        let now = Utc::now().date_naive();
        let start = now - Duration::days(30); // Approximate month start
        Self { start, end: now }
    }

    pub fn this_year() -> Self {
        let now = Utc::now().date_naive();
        let start = now - Duration::days(365); // Approximate year start
        Self { start, end: now }
    }
}

/// Trend metric trait
#[async_trait]
pub trait TrendMetric: Send + Sync {
    /// Get metric name
    fn name(&self) -> &str;

    /// Get metric URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Calculate the trend data
    async fn calculate(&self, range: DateRange) -> Result<TrendData, MetricError>;

    /// Default date range
    fn default_range(&self) -> DateRange {
        DateRange::last_days(30)
    }

    /// Width of the metric card
    fn width(&self) -> MetricWidth {
        MetricWidth::OneHalf
    }

    /// Serialize metric for JSON API
    fn to_json(&self) -> JsonValue {
        serde_json::json!({
            "type": "trend",
            "name": self.name(),
            "uri_key": self.uri_key(),
            "width": self.width(),
        })
    }
}

/// Helper macro to create trend metrics
#[macro_export]
macro_rules! trend_metric {
    (
        name: $name:expr,
        calculate: |$range:ident| $body:expr
    ) => {
        {
            struct CustomTrendMetric;

            #[async_trait::async_trait]
            impl $crate::metric::TrendMetric for CustomTrendMetric {
                fn name(&self) -> &str {
                    $name
                }

                async fn calculate(
                    &self,
                    $range: $crate::metric::DateRange,
                ) -> Result<$crate::metric::TrendData, $crate::metric::MetricError> {
                    $body
                }
            }

            Box::new(CustomTrendMetric) as Box<dyn $crate::metric::TrendMetric>
        }
    };
}
