//! **EXPERIMENTAL — not part of the RustForge 1.0 supported surface; API may change without a SemVer bump.**
//!
//! # rf-nova: Laravel Nova-Inspired Admin Panel Framework
//!
//! A powerful, flexible admin panel framework for RustForge, inspired by Laravel Nova.
//!
//! ## Features
//!
//! - **Resources**: Map your models to beautiful CRUD interfaces
//! - **Actions**: Perform tasks on one or more resources
//! - **Filters**: Filter resources with custom criteria
//! - **Lenses**: Create custom query views
//! - **Metrics**: Display value, trend, and partition metrics
//! - **Cards**: Custom dashboard widgets
//! - **Dashboards**: Aggregate cards and metrics
//! - **Authorization**: Policy-based access control
//! - **Export**: Export data to CSV or JSON
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use rf_nova::prelude::*;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create Nova instance
//! let nova = Nova::new()
//!     .with_path("/admin")
//!     .with_name("My Admin Panel")
//!     .register_dashboard(MainDashboard::new());
//!
//! // Add to your Axum app
//! let app = axum::Router::new()
//!     .merge(nova.routes());
//! # Ok(())
//! # }
//! ```
//!
//! ## Creating Resources
//!
//! ```rust,ignore
//! use rf_nova::prelude::*;
//!
//! pub struct UserResource;
//!
//! impl Resource for UserResource {
//!     type Entity = user::Entity;
//!     type Model = user::Model;
//!
//!     fn name() -> &'static str {
//!         "User"
//!     }
//!
//!     fn fields() -> Vec<Box<dyn Field>> {
//!         vec![
//!             Box::new(ID::new("id")),
//!             Box::new(Text::new("name").sortable().searchable()),
//!             Box::new(Email::new("email").sortable().searchable()),
//!             Box::new(Boolean::new("is_admin")),
//!             Box::new(DateTime::new("created_at").sortable()),
//!         ]
//!     }
//! }
//! ```
//!
//! ## Creating Actions
//!
//! ```rust,ignore
//! use rf_nova::prelude::*;
//!
//! pub struct DeactivateUser;
//!
//! #[async_trait]
//! impl Action for DeactivateUser {
//!     fn name(&self) -> &str {
//!         "Deactivate User"
//!     }
//!
//!     fn destructive(&self) -> bool {
//!         true
//!     }
//!
//!     async fn handle(&self, models: Vec<Value>, fields: ActionFields) -> ActionResult {
//!         // Perform action
//!         ActionResponse::success("Users deactivated")
//!     }
//! }
//! ```
//!
//! ## Creating Metrics
//!
//! ```rust,ignore
//! use rf_nova::prelude::*;
//!
//! pub struct TotalUsers;
//!
//! #[async_trait]
//! impl ValueMetric for TotalUsers {
//!     fn name(&self) -> &str {
//!         "Total Users"
//!     }
//!
//!     async fn calculate(&self) -> Result<MetricValue, MetricError> {
//!         let count = User::count().await?;
//!         Ok(MetricValue::new(count as f64).prefix("👥"))
//!     }
//! }
//! ```

pub mod action;
pub mod authorization;
pub mod card;
pub mod dashboard;
pub mod filter;
pub mod lens;
pub mod metric;
pub mod nova;
pub mod resource;
pub mod resource_router;
pub mod routes;

// Re-export main types
pub use action::{Action, ActionError, ActionField, ActionFields, ActionResponse, ActionResult, ExportAction, ExportFormat};
pub use authorization::{AdminOnlyPolicy, AllowAllPolicy, OwnerPolicy, ReadOnlyPolicy, ResourcePolicy};
pub use card::{Card, CardError, HelpCard, ProgressCard, TableCard, TableColumn};
pub use dashboard::{Dashboard, DashboardCards, MainDashboard};
pub use filter::{
    ActiveFilter, BooleanFilter, DateFilter, DateRangeFilter, Filter, FilterComponent,
    FilterCondition, FilterOption, SelectFilter, TrashedFilter,
};
pub use lens::{
    Lens, LensHaving, LensJoin, LensOrderBy, LensQuery, LensWhere, MostRecentLens, OrderDirection,
    TopItemsLens,
};
pub use metric::{
    Colors, DateRange, MetricError, MetricValue, MetricWidth, PartitionChartType, PartitionData,
    PartitionMetric, PartitionSegment, TrendData, TrendDirection, TrendMetric, ValueMetric,
};
pub use nova::{Nova, NovaBuilder, NovaConfig};
pub use resource_router::resource_router;
pub use resource::{
    crud::{create, destroy, export, index, show, update},
    field::{
        BelongsTo, Boolean, DateTime, Email, Field, FieldContext, File, HasMany, Image, Number,
        Password, Select, SelectOption, Text, Textarea, ID,
    },
    resource::{
        PaginatedResponse, PaginationMeta, Resource, ResourceError, ResourceQuery,
        ResourceResult, ResourceSchema,
    },
};

// Re-export derive macros
pub use rf_nova_macros::{
    Action as DeriveAction, Card as DeriveCard, Filter as DeriveFilter, Lens as DeriveLens,
    PartitionMetric as DerivePartitionMetric, Resource as DeriveResource,
    TrendMetric as DeriveTrendMetric, ValueMetric as DeriveValueMetric,
};

/// Prelude module for convenient imports
pub mod prelude {
    pub use super::{
        action::*, authorization::*, card::*, dashboard::*, filter::*, lens::*, metric::*, nova::*,
        resource::{crud::*, field::*, resource::*},
    };

    // `ExportFormat` is defined in both `action` and `resource::crud`. Prefer the
    // `action` variant here to match the crate-root re-export above; the explicit
    // re-export shadows the globs and resolves the ambiguous-glob-reexport warning.
    pub use super::action::ExportFormat;

    // Re-export derive macros
    pub use rf_nova_macros::{
        Action as DeriveAction, Card as DeriveCard, Filter as DeriveFilter, Lens as DeriveLens,
        PartitionMetric as DerivePartitionMetric, Resource as DeriveResource,
        TrendMetric as DeriveTrendMetric, ValueMetric as DeriveValueMetric,
    };

    // Re-export commonly used external types
    pub use async_trait::async_trait;
    pub use sea_orm::{DatabaseConnection, EntityTrait, ModelTrait};
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value;
}
