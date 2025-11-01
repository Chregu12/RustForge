//! Widget implementations for the dashboard

use crate::dashboard::WidgetData;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WidgetType {
    Metric,
    Chart,
    Table,
    List,
    Card,
}

/// Base trait for dashboard widgets
#[async_trait]
pub trait Widget: Send + Sync {
    async fn render(&self) -> anyhow::Result<WidgetData>;
}

/// Metric widget showing a single value with optional comparison
pub struct MetricWidget {
    id: String,
    title: String,
    value_fn: Box<dyn Fn() -> anyhow::Result<MetricValue> + Send + Sync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricValue {
    pub value: String,
    pub label: String,
    pub trend: Option<Trend>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trend {
    pub direction: TrendDirection,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendDirection {
    Up,
    Down,
    Neutral,
}

impl MetricWidget {
    pub fn new<F>(id: impl Into<String>, title: impl Into<String>, value_fn: F) -> Self
    where
        F: Fn() -> anyhow::Result<MetricValue> + Send + Sync + 'static,
    {
        Self {
            id: id.into(),
            title: title.into(),
            value_fn: Box::new(value_fn),
        }
    }
}

#[async_trait]
impl Widget for MetricWidget {
    async fn render(&self) -> anyhow::Result<WidgetData> {
        let value = (self.value_fn)()?;
        Ok(WidgetData {
            id: self.id.clone(),
            title: self.title.clone(),
            widget_type: WidgetType::Metric,
            data: json!(value),
        })
    }
}

/// Chart widget for displaying data visualizations
pub struct ChartWidget {
    id: String,
    title: String,
    chart_type: ChartType,
    data_fn: Box<dyn Fn() -> anyhow::Result<ChartData> + Send + Sync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChartType {
    Line,
    Bar,
    Pie,
    Doughnut,
    Area,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChartData {
    pub labels: Vec<String>,
    pub datasets: Vec<Dataset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dataset {
    pub label: String,
    pub data: Vec<f64>,
    pub color: Option<String>,
}

impl ChartWidget {
    pub fn new<F>(
        id: impl Into<String>,
        title: impl Into<String>,
        chart_type: ChartType,
        data_fn: F,
    ) -> Self
    where
        F: Fn() -> anyhow::Result<ChartData> + Send + Sync + 'static,
    {
        Self {
            id: id.into(),
            title: title.into(),
            chart_type,
            data_fn: Box::new(data_fn),
        }
    }
}

#[async_trait]
impl Widget for ChartWidget {
    async fn render(&self) -> anyhow::Result<WidgetData> {
        let data = (self.data_fn)()?;
        Ok(WidgetData {
            id: self.id.clone(),
            title: self.title.clone(),
            widget_type: WidgetType::Chart,
            data: json!({
                "type": self.chart_type,
                "data": data
            }),
        })
    }
}

/// Table widget for displaying tabular data
pub struct TableWidget {
    id: String,
    title: String,
    data_fn: Box<dyn Fn() -> anyhow::Result<TableData> + Send + Sync>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableData {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl TableWidget {
    pub fn new<F>(id: impl Into<String>, title: impl Into<String>, data_fn: F) -> Self
    where
        F: Fn() -> anyhow::Result<TableData> + Send + Sync + 'static,
    {
        Self {
            id: id.into(),
            title: title.into(),
            data_fn: Box::new(data_fn),
        }
    }
}

#[async_trait]
impl Widget for TableWidget {
    async fn render(&self) -> anyhow::Result<WidgetData> {
        let data = (self.data_fn)()?;
        Ok(WidgetData {
            id: self.id.clone(),
            title: self.title.clone(),
            widget_type: WidgetType::Table,
            data: json!(data),
        })
    }
}
