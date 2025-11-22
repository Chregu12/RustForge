//! Post Entity Model
//!
//! Demonstrates SeaORM entity definition with:
//! - Auto-increment primary key
//! - Foreign key relationship to User
//! - Optional fields
//! - Timestamps

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "posts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i32,

    pub title: String,
    pub content: String,

    #[sea_orm(nullable)]
    pub published: Option<bool>,

    pub user_id: i32,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::user::Entity",
        from = "Column::UserId",
        to = "super::user::Column::Id"
    )]
    User,
}

impl Related<super::user::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

// Helper methods for the Post model
impl Model {
    /// Check if post is published
    pub fn is_published(&self) -> bool {
        self.published.unwrap_or(false)
    }

    /// Get excerpt of content (first 100 characters)
    pub fn excerpt(&self, max_length: usize) -> String {
        if self.content.len() <= max_length {
            self.content.clone()
        } else {
            format!("{}...", &self.content[..max_length])
        }
    }
}
