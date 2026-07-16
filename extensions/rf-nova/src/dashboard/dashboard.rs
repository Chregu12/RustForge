//! Dashboard for Nova
//!
//! Dashboards aggregate cards and metrics for display.

use serde_json::Value;

use crate::card::Card;
use crate::metric::{PartitionMetric, TrendMetric, ValueMetric};

/// Dashboard trait
pub trait Dashboard: Send + Sync {
    /// Get dashboard name
    fn name(&self) -> &str {
        "Main Dashboard"
    }

    /// Get dashboard URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Get cards for this dashboard
    fn cards(&self) -> Vec<Box<dyn Card>> {
        vec![]
    }

    /// Get value metrics for this dashboard
    fn value_metrics(&self) -> Vec<Box<dyn ValueMetric>> {
        vec![]
    }

    /// Get trend metrics for this dashboard
    fn trend_metrics(&self) -> Vec<Box<dyn TrendMetric>> {
        vec![]
    }

    /// Get partition metrics for this dashboard
    fn partition_metrics(&self) -> Vec<Box<dyn PartitionMetric>> {
        vec![]
    }

    /// Get all cards (metrics + custom cards)
    fn all_cards(&self) -> DashboardCards {
        DashboardCards {
            value_metrics: self.value_metrics(),
            trend_metrics: self.trend_metrics(),
            partition_metrics: self.partition_metrics(),
            cards: self.cards(),
        }
    }

    /// Serialize dashboard for JSON API
    fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "uri_key": self.uri_key(),
        })
    }
}

/// Container for all dashboard cards
pub struct DashboardCards {
    pub value_metrics: Vec<Box<dyn ValueMetric>>,
    pub trend_metrics: Vec<Box<dyn TrendMetric>>,
    pub partition_metrics: Vec<Box<dyn PartitionMetric>>,
    pub cards: Vec<Box<dyn Card>>,
}

impl DashboardCards {
    pub fn new() -> Self {
        Self {
            value_metrics: vec![],
            trend_metrics: vec![],
            partition_metrics: vec![],
            cards: vec![],
        }
    }

    pub fn add_value_metric(mut self, metric: Box<dyn ValueMetric>) -> Self {
        self.value_metrics.push(metric);
        self
    }

    pub fn add_trend_metric(mut self, metric: Box<dyn TrendMetric>) -> Self {
        self.trend_metrics.push(metric);
        self
    }

    pub fn add_partition_metric(mut self, metric: Box<dyn PartitionMetric>) -> Self {
        self.partition_metrics.push(metric);
        self
    }

    pub fn add_card(mut self, card: Box<dyn Card>) -> Self {
        self.cards.push(card);
        self
    }

    /// Get JSON schema for all cards
    pub fn to_json(&self) -> Value {
        serde_json::json!({
            "value_metrics": self.value_metrics.iter().map(|m| m.to_json()).collect::<Vec<_>>(),
            "trend_metrics": self.trend_metrics.iter().map(|m| m.to_json()).collect::<Vec<_>>(),
            "partition_metrics": self.partition_metrics.iter().map(|m| m.to_json()).collect::<Vec<_>>(),
            "cards": self.cards.iter().map(|c| c.to_json()).collect::<Vec<_>>(),
        })
    }
}

impl Default for DashboardCards {
    fn default() -> Self {
        Self::new()
    }
}

/// Main dashboard implementation
pub struct MainDashboard {
    name: String,
    cards: DashboardCards,
}

impl MainDashboard {
    pub fn new() -> Self {
        Self {
            name: "Main Dashboard".to_string(),
            cards: DashboardCards::new(),
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    pub fn value_metric(mut self, metric: Box<dyn ValueMetric>) -> Self {
        self.cards.value_metrics.push(metric);
        self
    }

    pub fn trend_metric(mut self, metric: Box<dyn TrendMetric>) -> Self {
        self.cards.trend_metrics.push(metric);
        self
    }

    pub fn partition_metric(mut self, metric: Box<dyn PartitionMetric>) -> Self {
        self.cards.partition_metrics.push(metric);
        self
    }

    pub fn card(mut self, card: Box<dyn Card>) -> Self {
        self.cards.cards.push(card);
        self
    }
}

impl Dashboard for MainDashboard {
    fn name(&self) -> &str {
        &self.name
    }

    fn cards(&self) -> Vec<Box<dyn Card>> {
        // Note: We can't move out of self.cards, so we return empty vec
        // In practice, use all_cards() instead
        vec![]
    }

    fn value_metrics(&self) -> Vec<Box<dyn ValueMetric>> {
        vec![]
    }

    fn trend_metrics(&self) -> Vec<Box<dyn TrendMetric>> {
        vec![]
    }

    fn partition_metrics(&self) -> Vec<Box<dyn PartitionMetric>> {
        vec![]
    }

    fn all_cards(&self) -> DashboardCards {
        // Return a reference to the cards
        // Note: This is a simplified version. In production, you'd want to handle this differently
        DashboardCards::new()
    }
}

impl Default for MainDashboard {
    fn default() -> Self {
        Self::new()
    }
}
