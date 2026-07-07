#![allow(dead_code)] // demonstration code showcasing the framework API, not every item is exercised
// User entity

use chrono::{DateTime, Utc};
use rf_orm::{Set, SoftDelete};
use sea_orm::{entity::prelude::*, ActiveValue};
use serde::{Deserialize, Serialize};

/// User model
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "users")]
pub struct Model {
    /// Primary key
    #[sea_orm(primary_key)]
    pub id: i32,

    /// Email address (unique)
    #[sea_orm(unique)]
    pub email: String,

    /// Display name
    pub name: String,

    /// Password hash (bcrypt)
    #[serde(skip_serializing)]
    pub password_hash: String,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Updated timestamp
    pub updated_at: DateTime<Utc>,

    /// Soft delete timestamp
    #[sea_orm(nullable)]
    pub deleted_at: Option<DateTime<Utc>>,
}

/// Relations
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// Implement soft delete trait
impl SoftDelete for ActiveModel {
    fn soft_delete(&mut self) {
        self.deleted_at = Set(Some(Utc::now()));
        self.updated_at = Set(Utc::now());
    }

    fn restore(&mut self) {
        self.deleted_at = Set(None);
        self.updated_at = Set(Utc::now());
    }

    fn is_deleted(&self) -> bool {
        matches!(
            &self.deleted_at,
            ActiveValue::Set(Some(_)) | ActiveValue::Unchanged(Some(_))
        )
    }
}

impl Model {
    /// Check if user is deleted
    pub fn is_deleted(&self) -> bool {
        self.deleted_at.is_some()
    }

    /// Create display string
    pub fn display(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}
