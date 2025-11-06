use async_graphql::{InputObject, SimpleObject};
use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// Type aliases for SeaORM
pub type Decimal = rust_decimal::Decimal;
pub type DateTimeUtc = chrono::NaiveDateTime;

/// Product Entity for Sea-ORM
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "products")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub stock: i32,
    pub sku: String,
    pub active: bool,
    pub created_at: DateTimeUtc,
    pub updated_at: DateTimeUtc,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

/// GraphQL Product Type
#[derive(SimpleObject, Clone, Debug)]
pub struct Product {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub price: String,
    pub stock: i32,
    pub sku: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Model> for Product {
    fn from(model: Model) -> Self {
        Self {
            id: model.id,
            name: model.name,
            description: model.description,
            price: model.price.to_string(),
            stock: model.stock,
            sku: model.sku,
            active: model.active,
            created_at: model.created_at.and_utc(),
            updated_at: model.updated_at.and_utc(),
        }
    }
}

/// Input for creating a new product
#[derive(InputObject, Debug)]
pub struct ProductInput {
    pub name: String,
    pub description: Option<String>,
    pub price: String,
    pub stock: i32,
    pub sku: String,
    pub active: Option<bool>,
}

/// Input for updating a product
#[derive(InputObject, Debug)]
pub struct UpdateProductInput {
    pub name: Option<String>,
    pub description: Option<String>,
    pub price: Option<String>,
    pub stock: Option<i32>,
    pub sku: Option<String>,
    pub active: Option<bool>,
}

// Type aliases for the generated SeaORM types
pub type ProductEntity = Entity;
pub type ProductColumn = Column;
pub type ProductActiveModel = ActiveModel;
