use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// Type alias for SeaORM
pub type DateTimeUtc = chrono::NaiveDateTime;

/// Account Entity for Sea-ORM
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "accounts")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub email: String,
    pub name: String,
    pub role: String,
    pub active: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// GraphQL Account Type
#[derive(SimpleObject, Clone, Debug)]
pub struct Account {
    pub id: i64,
    pub email: String,
    pub name: String,
    pub role: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Model> for Account {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            email: model.email,
            name: model.name,
            role: model.role,
            active: model.active,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.and_utc(),
        }
    }
}

/// Input for creating a new account
#[derive(InputObject, Debug)]
pub struct AccountInput {
    pub email: String,
    pub name: String,
    pub role: String,
    pub active: Option<bool>,
}

/// Input for updating an account
#[derive(InputObject, Debug)]
pub struct UpdateAccountInput {
    pub email: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
    pub active: Option<bool>,
}

// Type aliases for the generated SeaORM types
pub type AccountEntity = Entity;
pub type AccountColumn = Column;
pub type AccountActiveModel = ActiveModel;
