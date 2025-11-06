use crate::context::GraphQLContext;
use crate::error::{database_error, not_found, validation_error};
use crate::types::product::{Product, ProductEntity, ProductColumn, ProductActiveModel, ProductInput, UpdateProductInput};
use async_graphql::{Context, Object, Result};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect,
};
use std::str::FromStr;

#[derive(Default)]
pub struct ProductQuery;

#[Object]
impl ProductQuery {
    /// Get a product by ID
    async fn product(&self, ctx: &Context<'_>, id: i64) -> Result<Product> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let product = ProductEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Product with id {} not found", id)))?;

        Ok(product.into())
    }

    /// Get all products with optional pagination
    async fn products(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 0)] offset: u64,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<Product>> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let products = ProductEntity::find()
            .order_by_asc(ProductColumn::Id)
            .offset(offset)
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(products.into_iter().map(|p| p.into()).collect())
    }

    /// Count total products
    async fn products_count(&self, ctx: &Context<'_>) -> Result<u64> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        ProductEntity::find()
            .count(db)
            .await
            .map_err(database_error)
    }

    /// Search products by name
    async fn search_products(
        &self,
        ctx: &Context<'_>,
        query: String,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<Product>> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let products = ProductEntity::find()
            .filter(ProductColumn::Name.contains(&query))
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(products.into_iter().map(|p| p.into()).collect())
    }

    /// Get active products only
    async fn active_products(
        &self,
        ctx: &Context<'_>,
        #[graphql(default = 10)] limit: u64,
    ) -> Result<Vec<Product>> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let products = ProductEntity::find()
            .filter(ProductColumn::Active.eq(true))
            .limit(limit)
            .all(db)
            .await
            .map_err(database_error)?;

        Ok(products.into_iter().map(|p| p.into()).collect())
    }
}

#[derive(Default)]
pub struct ProductMutation;

#[Object]
impl ProductMutation {
    /// Create a new product
    async fn create_product(&self, ctx: &Context<'_>, input: ProductInput) -> Result<Product> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        // Validate price
        let price = rust_decimal::Decimal::from_str(&input.price)
            .map_err(|_| validation_error("Invalid price format"))?;

        // Check if SKU already exists
        let existing = ProductEntity::find()
            .filter(ProductColumn::Sku.eq(&input.sku))
            .one(db)
            .await
            .map_err(database_error)?;

        if existing.is_some() {
            return Err(validation_error(format!(
                "Product with SKU '{}' already exists",
                input.sku
            )));
        }

        let now = Utc::now().naive_utc();
        let product = ProductActiveModel {
            name: ActiveValue::Set(input.name),
            description: ActiveValue::Set(input.description),
            price: ActiveValue::Set(price),
            stock: ActiveValue::Set(input.stock),
            sku: ActiveValue::Set(input.sku),
            active: ActiveValue::Set(input.active.unwrap_or(true)),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };

        let result = product.insert(db).await.map_err(database_error)?;
        Ok(result.into())
    }

    /// Update an existing product
    async fn update_product(
        &self,
        ctx: &Context<'_>,
        id: i64,
        input: UpdateProductInput,
    ) -> Result<Product> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        // Find existing product
        let product = ProductEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Product with id {} not found", id)))?;

        let mut active: ProductActiveModel = product.into();

        // Update fields if provided
        if let Some(name) = input.name {
            active.name = ActiveValue::Set(name);
        }
        if let Some(description) = input.description {
            active.description = ActiveValue::Set(Some(description));
        }
        if let Some(price_str) = input.price {
            let price = rust_decimal::Decimal::from_str(&price_str)
                .map_err(|_| validation_error("Invalid price format"))?;
            active.price = ActiveValue::Set(price);
        }
        if let Some(stock) = input.stock {
            active.stock = ActiveValue::Set(stock);
        }
        if let Some(sku) = input.sku {
            // Check if new SKU already exists
            let existing = ProductEntity::find()
                .filter(ProductColumn::Sku.eq(&sku))
                .filter(ProductColumn::Id.ne(id))
                .one(db)
                .await
                .map_err(database_error)?;

            if existing.is_some() {
                return Err(validation_error(format!(
                    "Product with SKU '{}' already exists",
                    sku
                )));
            }
            active.sku = ActiveValue::Set(sku);
        }
        if let Some(active_flag) = input.active {
            active.active = ActiveValue::Set(active_flag);
        }

        active.updated_at = ActiveValue::Set(Utc::now().naive_utc());

        let result = active.update(db).await.map_err(database_error)?;
        Ok(result.into())
    }

    /// Delete a product
    async fn delete_product(&self, ctx: &Context<'_>, id: i64) -> Result<bool> {
        let context = ctx.data::<GraphQLContext>()?;
        let db = context.db();

        let product = ProductEntity::find_by_id(id)
            .one(db)
            .await
            .map_err(database_error)?
            .ok_or_else(|| not_found(format!("Product with id {} not found", id)))?;

        let active: ProductActiveModel = product.into();
        active.delete(db).await.map_err(database_error)?;

        Ok(true)
    }
}
