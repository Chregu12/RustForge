//! Infrastruktur-Schicht für Foundry Core.

pub mod artifacts;
pub mod audit;
pub mod config;
pub mod migrations;
pub mod seeds;

pub mod cache;
pub mod db;
pub mod events;
pub mod queue;
pub mod storage;
pub mod validation;
pub mod package_manager;
pub mod metrics;

pub use artifacts::LocalArtifactPort;
pub use audit::{AuditOutcome, AuditRecord, JsonlAuditLogger};
pub use cache::InMemoryCacheStore;
pub use config::{ConfigError, ConfigProvider, DatabaseConfig, DatabaseDriver, DotenvProvider};
pub use db::{connect as connect_db, ConnectionError};
pub use events::InMemoryEventBus;
pub use migrations::SeaOrmMigrationService;
pub use queue::InMemoryQueue;
pub use seeds::SeaOrmSeedService;
pub use storage::{FileStorageAdapter, InMemoryStorage};
pub use validation::SimpleValidationService;
pub use package_manager::{PackageManager, Package, PackageInfo, SearchResult, OutdatedPackage};
pub use metrics::{
    Metric, MetricAggregate, MetricsCollector, PerformanceMonitor, PerformanceReport,
    SystemMetrics, Timer,
};
