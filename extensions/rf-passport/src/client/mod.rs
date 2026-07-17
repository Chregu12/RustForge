//! OAuth Client management

pub mod model;
pub mod repository;

pub use model::{ActiveModel, Column, Entity, Model, Relation};
pub use repository::ClientRepository;
