//! Performance Monitoring & Metrics
//!
//! Sammelt und verarbeitet Metriken für Performance-Monitoring.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Eine Performance-Metrik
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Metrik-Name
    pub name: String,
    /// Metrik-Wert
    pub value: f64,
    /// Einheit (z.B. "ms", "bytes", "count")
    pub unit: String,
    /// Zeitstempel
    pub timestamp: i64,
    /// Tags/Labels
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

impl Metric {
    /// Erstellt eine neue Metrik
    pub fn new(name: impl Into<String>, value: f64, unit: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value,
            unit: unit.into(),
            timestamp: chrono::Utc::now().timestamp(),
            tags: HashMap::new(),
        }
    }

    /// Fügt einen Tag hinzu
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Fügt mehrere Tags hinzu
    pub fn with_tags(mut self, tags: HashMap<String, String>) -> Self {
        self.tags.extend(tags);
        self
    }
}

/// Timer für Performance-Messungen
pub struct Timer {
    /// Start-Zeitpunkt
    start: Instant,
    /// Name der Operation
    name: String,
    /// Tags
    tags: HashMap<String, String>,
}

impl Timer {
    /// Startet einen neuen Timer
    pub fn start(name: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
            tags: HashMap::new(),
        }
    }

    /// Fügt einen Tag hinzu
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Stoppt den Timer und gibt die Duration zurück
    pub fn stop(self) -> Duration {
        self.start.elapsed()
    }

    /// Stoppt den Timer und gibt eine Metrik zurück
    pub fn stop_as_metric(self) -> Metric {
        let elapsed = self.start.elapsed();
        Metric::new(&self.name, elapsed.as_millis() as f64, "ms")
            .with_tags(self.tags)
    }
}

/// Metrik-Aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricAggregate {
    /// Durchschnitt
    pub average: f64,
    /// Minimum
    pub min: f64,
    /// Maximum
    pub max: f64,
    /// Anzahl der Samples
    pub count: usize,
    /// Summe
    pub sum: f64,
    /// Perzentile (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p99: Option<f64>,
}

impl MetricAggregate {
    /// Erstellt eine neue Aggregation aus Metriken
    pub fn from_metrics(metrics: &[f64]) -> Self {
        if metrics.is_empty() {
            return Self {
                average: 0.0,
                min: 0.0,
                max: 0.0,
                count: 0,
                sum: 0.0,
                p50: None,
                p95: None,
                p99: None,
            };
        }

        let mut sorted = metrics.to_vec();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let sum: f64 = metrics.iter().sum();
        let count = metrics.len();
        let average = sum / count as f64;
        let min = sorted[0];
        let max = sorted[count - 1];

        // Berechne Perzentile
        let p50 = Self::percentile(&sorted, 50);
        let p95 = Self::percentile(&sorted, 95);
        let p99 = Self::percentile(&sorted, 99);

        Self {
            average,
            min,
            max,
            count,
            sum,
            p50: Some(p50),
            p95: Some(p95),
            p99: Some(p99),
        }
    }

    fn percentile(sorted: &[f64], p: usize) -> f64 {
        let index = (p as f64 / 100.0 * sorted.len() as f64) as usize;
        let index = index.min(sorted.len() - 1);
        sorted[index]
    }
}

/// Metrics Collector
pub struct MetricsCollector {
    /// Gespeicherte Metriken
    metrics: Arc<RwLock<Vec<Metric>>>,
    /// Metrik-Historie (Name -> Werte)
    history: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    /// Max Historie-Größe pro Metrik
    max_history_size: usize,
}

impl MetricsCollector {
    /// Erstellt einen neuen MetricsCollector
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(Vec::new())),
            history: Arc::new(RwLock::new(HashMap::new())),
            max_history_size: 1000,
        }
    }

    /// Setzt die maximale Historie-Größe
    pub fn with_max_history_size(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }

    /// Sammelt eine Metrik
    pub async fn collect(&self, metric: Metric) {
        debug!("Collecting metric: {} = {} {}", metric.name, metric.value, metric.unit);

        // Füge zur Historie hinzu
        let mut history = self.history.write().await;
        let values = history.entry(metric.name.clone()).or_insert_with(Vec::new);
        values.push(metric.value);

        // Begrenze Historie-Größe
        if values.len() > self.max_history_size {
            values.remove(0);
        }

        // Speichere Metrik
        let mut metrics = self.metrics.write().await;
        metrics.push(metric);

        // Begrenze Metrik-Speicher
        if metrics.len() > self.max_history_size * 10 {
            metrics.drain(0..self.max_history_size);
        }
    }

    /// Sammelt mehrere Metriken
    pub async fn collect_batch(&self, metrics: Vec<Metric>) {
        for metric in metrics {
            self.collect(metric).await;
        }
    }

    /// Gibt alle Metriken zurück
    pub async fn get_all(&self) -> Vec<Metric> {
        let metrics = self.metrics.read().await;
        metrics.clone()
    }

    /// Gibt Metriken für einen Namen zurück
    pub async fn get_by_name(&self, name: &str) -> Vec<Metric> {
        let metrics = self.metrics.read().await;
        metrics
            .iter()
            .filter(|m| m.name == name)
            .cloned()
            .collect()
    }

    /// Gibt Aggregate-Statistiken für eine Metrik zurück
    pub async fn get_aggregate(&self, name: &str) -> Option<MetricAggregate> {
        let history = self.history.read().await;
        history.get(name).map(|values| MetricAggregate::from_metrics(values))
    }

    /// Gibt alle verfügbaren Metrik-Namen zurück
    pub async fn get_metric_names(&self) -> Vec<String> {
        let history = self.history.read().await;
        history.keys().cloned().collect()
    }

    /// Löscht alle Metriken
    pub async fn clear(&self) {
        let mut metrics = self.metrics.write().await;
        metrics.clear();

        let mut history = self.history.write().await;
        history.clear();

        info!("All metrics cleared");
    }

    /// Löscht Metriken für einen Namen
    pub async fn clear_metric(&self, name: &str) {
        let mut metrics = self.metrics.write().await;
        metrics.retain(|m| m.name != name);

        let mut history = self.history.write().await;
        history.remove(name);

        info!("Metrics cleared for: {}", name);
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

/// System-Metriken
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// CPU-Auslastung (Prozent)
    pub cpu_usage_percent: f64,
    /// Memory-Auslastung (MB)
    pub memory_usage_mb: f64,
    /// Verfügbarer Speicher (MB)
    pub memory_available_mb: f64,
    /// Aktive Verbindungen
    pub active_connections: usize,
    /// Requests pro Sekunde
    pub requests_per_second: f64,
    /// Durchschnittliche Response-Zeit (ms)
    pub avg_response_time_ms: f64,
}

impl SystemMetrics {
    /// Erstellt neue System-Metriken
    pub fn new() -> Self {
        Self {
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            memory_available_mb: 0.0,
            active_connections: 0,
            requests_per_second: 0.0,
            avg_response_time_ms: 0.0,
        }
    }

    /// Konvertiert zu Metrik-Liste
    pub fn to_metrics(&self) -> Vec<Metric> {
        vec![
            Metric::new("system.cpu.usage", self.cpu_usage_percent, "percent"),
            Metric::new("system.memory.usage", self.memory_usage_mb, "mb"),
            Metric::new("system.memory.available", self.memory_available_mb, "mb"),
            Metric::new("system.connections.active", self.active_connections as f64, "count"),
            Metric::new("system.requests.per_second", self.requests_per_second, "rps"),
            Metric::new("system.response_time.avg", self.avg_response_time_ms, "ms"),
        ]
    }
}

impl Default for SystemMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance Monitor Service
pub struct PerformanceMonitor {
    /// Metrics Collector
    collector: Arc<MetricsCollector>,
}

impl PerformanceMonitor {
    /// Erstellt einen neuen Performance Monitor
    pub fn new() -> Self {
        Self {
            collector: Arc::new(MetricsCollector::new()),
        }
    }

    /// Gibt den Collector zurück
    pub fn collector(&self) -> Arc<MetricsCollector> {
        self.collector.clone()
    }

    /// Startet einen Timer
    pub fn timer(&self, name: impl Into<String>) -> Timer {
        Timer::start(name)
    }

    /// Sammelt eine Metrik
    pub async fn collect(&self, metric: Metric) {
        self.collector.collect(metric).await;
    }

    /// Sammelt System-Metriken
    pub async fn collect_system_metrics(&self, metrics: SystemMetrics) {
        self.collector.collect_batch(metrics.to_metrics()).await;
    }

    /// Gibt einen Report zurück
    pub async fn report(&self) -> PerformanceReport {
        let metric_names = self.collector.get_metric_names().await;
        let mut aggregates = HashMap::new();

        for name in &metric_names {
            if let Some(agg) = self.collector.get_aggregate(name).await {
                aggregates.insert(name.clone(), agg);
            }
        }

        PerformanceReport {
            total_metrics: self.collector.get_all().await.len(),
            metric_names,
            aggregates,
            generated_at: chrono::Utc::now().timestamp(),
        }
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance Report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Gesamtanzahl der Metriken
    pub total_metrics: usize,
    /// Metrik-Namen
    pub metric_names: Vec<String>,
    /// Aggregierte Statistiken
    pub aggregates: HashMap<String, MetricAggregate>,
    /// Zeitstempel der Generierung
    pub generated_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let collector = MetricsCollector::new();

        let metric = Metric::new("test.metric", 42.0, "count");
        collector.collect(metric).await;

        let all = collector.get_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "test.metric");
        assert_eq!(all[0].value, 42.0);
    }

    #[tokio::test]
    async fn test_metric_aggregate() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let agg = MetricAggregate::from_metrics(&values);

        assert_eq!(agg.count, 5);
        assert_eq!(agg.min, 1.0);
        assert_eq!(agg.max, 5.0);
        assert_eq!(agg.average, 3.0);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::start("test.operation");
        std::thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();

        assert!(elapsed.as_millis() >= 10);
    }

    #[test]
    fn test_system_metrics_to_metrics() {
        let sys = SystemMetrics {
            cpu_usage_percent: 45.5,
            memory_usage_mb: 256.0,
            memory_available_mb: 1024.0,
            active_connections: 42,
            requests_per_second: 123.45,
            avg_response_time_ms: 25.5,
        };

        let metrics = sys.to_metrics();
        assert_eq!(metrics.len(), 6);
        assert!(metrics.iter().any(|m| m.name == "system.cpu.usage"));
    }
}
