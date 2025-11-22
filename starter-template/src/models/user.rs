//! User Entity Model
//!
//! Demonstrates SeaORM entity definition with:
//! - Auto-increment primary key
//! - Unique email constraint
//! - Password hashing
//! - Timestamps
//! - Soft deletes

use sea_orm::entity::prelude::*;
use sea_orm::Set;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    #[sea_orm(unique)]
    pub email: String,

    pub name: String,

    #[serde(skip_serializing)]
    pub password_hash: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,

    #[sea_orm(nullable)]
    pub deleted_at: Option<chrono::NaiveDateTime>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::post::Entity")]
    Posts,
}

impl Related<super::post::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Posts.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Helper methods for the User model
impl Model {
    /// Display user information (safe for logging)
    pub fn display(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }

    /// Check if user is soft-deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }
}

// Soft delete helper for ActiveModel
impl ActiveModel {
    /// Mark user as soft-deleted
    pub fn soft_delete(&mut self) {
        self.deleted_at = Set(Some(chrono::Utc::now().naive_utc()));
        self.updated_at = Set(chrono::Utc::now().naive_utc());
    }

    /// Restore soft-deleted user
    pub fn restore(&mut self) {
        self.deleted_at = Set(None);
        self.updated_at = Set(chrono::Utc::now().naive_utc());
    }
}
