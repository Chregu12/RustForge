//! Route facade module providing Laravel-style static routing API

pub mod builder;
pub mod group;
pub mod handler;
pub mod registry;
pub mod route;

pub use builder::FacadeRouteBuilder;
pub use group::GroupBuilder;
pub use registry::{global_router, GlobalRouter};
pub use route::{MiddlewareGroupBuilder, Route};
