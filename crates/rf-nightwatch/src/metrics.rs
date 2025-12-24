//! Metrics collection

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};

static METRICS_REGISTRY: OnceLock<Arc<MetricsRegistry>> = OnceLock::new();

/// Counter metric
#[derive(Clone)]
pub struct Counter {
    name: String,
    value: Arc<AtomicU64>,
}

impl Counter {
    /// Create a new counter
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Increment the counter by 1
    pub fn increment(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment the counter by a value
    pub fn increment_by(&self, value: u64) {
        self.value.fetch_add(value, Ordering::Relaxed);
    }

    /// Get the current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }

    /// Reset the counter
    pub fn reset(&self) {
        self.value.store(0, Ordering::Relaxed);
    }

    /// Get the counter name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Gauge metric
#[derive(Clone)]
pub struct Gauge {
    name: String,
    value: Arc<RwLock<f64>>,
}

impl Gauge {
    /// Create a new gauge
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            value: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Set the gauge value
    pub fn set(&self, value: f64) {
        let mut v = self.value.write().unwrap();
        *v = value;
    }

    /// Increment the gauge
    pub fn increment(&self) {
        let mut v = self.value.write().unwrap();
        *v += 1.0;
    }

    /// Decrement the gauge
    pub fn decrement(&self) {
        let mut v = self.value.write().unwrap();
        *v -= 1.0;
    }

    /// Get the current value
    pub fn get(&self) -> f64 {
        *self.value.read().unwrap()
    }

    /// Get the gauge name
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// Histogram metric
#[derive(Clone)]
pub struct Histogram {
    name: String,
    values: Arc<RwLock<Vec<f64>>>,
}

impl Histogram {
    /// Create a new histogram
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            values: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Record a value
    pub fn record(&self, value: f64) {
        let mut values = self.values.write().unwrap();
        values.push(value);
    }

    /// Get the count of recorded values
    pub fn count(&self) -> usize {
        self.values.read().unwrap().len()
    }

    /// Get the sum of all values
    pub fn sum(&self) -> f64 {
        self.values.read().unwrap().iter().sum()
    }

    /// Get the average
    pub fn avg(&self) -> Option<f64> {
        let values = self.values.read().unwrap();
        if values.is_empty() {
            None
        } else {
            Some(values.iter().sum::<f64>() / values.len() as f64)
        }
    }

    /// Get the minimum value
    pub fn min(&self) -> Option<f64> {
        let values = self.values.read().unwrap();
        values.iter().cloned().min_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Get the maximum value
    pub fn max(&self) -> Option<f64> {
        let values = self.values.read().unwrap();
        values.iter().cloned().max_by(|a, b| a.partial_cmp(b).unwrap())
    }

    /// Get a percentile
    pub fn percentile(&self, p: f64) -> Option<f64> {
        let mut values = self.values.read().unwrap().clone();
        if values.is_empty() {
            return None;
        }
        values.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let idx = ((p / 100.0) * (values.len() - 1) as f64) as usize;
        Some(values[idx])
    }

    /// Get the histogram name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Reset the histogram
    pub fn reset(&self) {
        let mut values = self.values.write().unwrap();
        values.clear();
    }
}

/// Metrics snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub counters: HashMap<String, u64>,
    pub gauges: HashMap<String, f64>,
    pub histograms: HashMap<String, HistogramSnapshot>,
}

/// Histogram snapshot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramSnapshot {
    pub count: usize,
    pub sum: f64,
    pub avg: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub p50: Option<f64>,
    pub p90: Option<f64>,
    pub p99: Option<f64>,
}

/// Metrics registry
pub struct MetricsRegistry {
    counters: RwLock<HashMap<String, Counter>>,
    gauges: RwLock<HashMap<String, Gauge>>,
    histograms: RwLock<HashMap<String, Histogram>>,
}

impl MetricsRegistry {
    /// Create a new metrics registry
    pub fn new() -> Self {
        Self {
            counters: RwLock::new(HashMap::new()),
            gauges: RwLock::new(HashMap::new()),
            histograms: RwLock::new(HashMap::new()),
        }
    }

    /// Get the global metrics registry
    pub fn global() -> Arc<Self> {
        METRICS_REGISTRY
            .get_or_init(|| Arc::new(Self::new()))
            .clone()
    }

    /// Get or create a counter
    pub fn counter(&self, name: &str) -> Counter {
        let mut counters = self.counters.write().unwrap();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Counter::new(name))
            .clone()
    }

    /// Get or create a gauge
    pub fn gauge(&self, name: &str) -> Gauge {
        let mut gauges = self.gauges.write().unwrap();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Gauge::new(name))
            .clone()
    }

    /// Get or create a histogram
    pub fn histogram(&self, name: &str) -> Histogram {
        let mut histograms = self.histograms.write().unwrap();
        histograms
            .entry(name.to_string())
            .or_insert_with(|| Histogram::new(name))
            .clone()
    }

    /// Get a snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        let counters = self.counters.read().unwrap();
        let gauges = self.gauges.read().unwrap();
        let histograms = self.histograms.read().unwrap();

        MetricsSnapshot {
            counters: counters.iter().map(|(k, v)| (k.clone(), v.get())).collect(),
            gauges: gauges.iter().map(|(k, v)| (k.clone(), v.get())).collect(),
            histograms: histograms
                .iter()
                .map(|(k, v)| {
                    (
                        k.clone(),
                        HistogramSnapshot {
                            count: v.count(),
                            sum: v.sum(),
                            avg: v.avg(),
                            min: v.min(),
                            max: v.max(),
                            p50: v.percentile(50.0),
                            p90: v.percentile(90.0),
                            p99: v.percentile(99.0),
                        },
                    )
                })
                .collect(),
        }
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
