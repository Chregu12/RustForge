//! Health check system for RustForge
//!
//! Provides comprehensive health checking for production deployments, including:
//! - Database connectivity checks
//! - Redis connectivity checks
//! - Disk space monitoring
//! - Memory usage monitoring
//! - Custom health checks
//! - Kubernetes liveness/readiness probes
//!
//! # Features
//!
//! - Multiple built-in health checks
//! - Axum endpoint integration (`/health`, `/health/live`, `/health/ready`)
//! - Thresholds for warning and critical states
//! - Optional database and Redis support via feature flags
//! - Detailed metadata in responses
//!
//! # Quick Start
//!
//! ```no_run
//! use rf_health::{health_router, HealthChecker};
//! use rf_health::checks::{MemoryCheck, DiskCheck};
//! use axum::Router;
//!
//! # async fn example() {
//! // Create health checker
//! let checker = HealthChecker::new()
//!     .add_check(MemoryCheck::default())
//!     .add_check(DiskCheck::default());
//!
//! // Add to router
//! let app = Router::new()
//!     .merge(health_router(checker));
//!
//! // Now accessible at:
//! // GET /health - All health checks
//! // GET /health/live - Liveness probe (Kubernetes)
//! // GET /health/ready - Readiness probe (Kubernetes)
//! # }
//! ```
//!
//! # Custom Health Checks
//!
//! ```no_run
//! use rf_health::{HealthCheck, CheckResult};
//! use async_trait::async_trait;
//!
//! struct MyCheck;
//!
//! #[async_trait]
//! impl HealthCheck for MyCheck {
//!     fn name(&self) -> &str {
//!         "my_check"
//!     }
//!
//!     async fn check(&self) -> CheckResult {
//!         // Perform your check
//!         CheckResult::healthy(self.name())
//!     }
//! }
//! ```
//!
//! # Database Check (Feature: `database`)
//!
//! ```no_run
//! # #[cfg(feature = "database")]
//! # async fn example() {
//! use rf_health::{HealthChecker, health_router};
//! use rf_health::checks::DatabaseCheck;
//! use sqlx::PgPool;
//!
//! let pool = PgPool::connect("postgres://localhost/mydb").await.unwrap();
//!
//! let checker = HealthChecker::new()
//!     .add_check(DatabaseCheck::new(pool));
//!
//! let app = axum::Router::new()
//!     .merge(health_router(checker));
//! # }
//! ```
//!
//! # Redis Check (Feature: `redis-check`)
//!
//! ```no_run
//! # #[cfg(feature = "redis-check")]
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! use rf_health::{HealthChecker, health_router};
//! use rf_health::checks::RedisCheck;
//!
//! let redis_check = RedisCheck::from_url("redis://localhost").await?;
//!
//! let checker = HealthChecker::new()
//!     .add_check(redis_check);
//!
//! let app = axum::Router::new()
//!     .merge(health_router(checker));
//! # Ok(())
//! # }
//! ```
//!
//! # Response Format
//!
//! Health endpoints return JSON:
//!
//! ```json
//! {
//!   "status": "healthy",
//!   "checks": [
//!     {
//!       "name": "memory",
//!       "status": "healthy",
//!       "metadata": {
//!         "total_bytes": 17179869184,
//!         "used_bytes": 8589934592,
//!         "usage_percent": 50.0
//!       },
//!       "timestamp": "2024-01-15T10:30:00Z"
//!     }
//!   ],
//!   "timestamp": "2024-01-15T10:30:00Z"
//! }
//! ```
//!
//! # Kubernetes Integration
//!
//! ```yaml
//! livenessProbe:
//!   httpGet:
//!     path: /health/live
//!     port: 8080
//!   initialDelaySeconds: 10
//!   periodSeconds: 30
//!
//! readinessProbe:
//!   httpGet:
//!     path: /health/ready
//!     port: 8080
//!   initialDelaySeconds: 5
//!   periodSeconds: 10
//! ```

mod checker;
mod endpoint;
mod error;

/// Built-in health checks
pub mod checks;

pub use checker::{CheckResult, HealthCheck, HealthResponse, HealthStatus};
pub use endpoint::{health_router, HealthChecker};
pub use error::{HealthError, HealthResult};
