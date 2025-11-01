//! Foundry Tenancy - Multi-tenancy support
//!
//! # Features
//!
//! - **Tenant Isolation**: Separate data by tenant
//! - **Domain Routing**: Route requests based on domain
//! - **Tenant Context**: Request-scoped tenant information
//! - **Query Scoping**: Automatic tenant filtering
//!
//! # Example
//!
//! ```no_run
//! use foundry_tenancy::prelude::*;
//!
//! # async fn example() {
//! let tenant = Tenant::new("acme-corp", "Acme Corp");
//! let manager = TenantManager::new();
//!
//! manager.register(tenant).await;
//! # }
//! ```

pub mod tenant;
pub mod middleware;
pub mod scopes;
pub mod manager;

pub use tenant::{Tenant, TenantId, TenantError};
pub use middleware::TenantMiddleware;
pub use scopes::TenantScope;
pub use manager::TenantManager;

pub mod prelude {
    pub use crate::tenant::{Tenant, TenantId};
    pub use crate::middleware::TenantMiddleware;
    pub use crate::manager::TenantManager;
    pub use crate::scopes::TenantScope;
}
