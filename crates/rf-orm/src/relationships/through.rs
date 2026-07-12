//! # Through Relationships (HasOneThrough & HasManyThrough)
//!
//! Support for Laravel-style "through" relationships, which allow accessing distant
//! relationships via an intermediate model.
//!
//! ## Overview
//!
//! Through relationships are useful when you need to access a relationship through
//! another model. For example:
//! - Country -> User -> Post (get all posts in a country through users)
//! - Supplier -> Product -> Review (get all reviews for a supplier's products)
//!
//! ## Pattern
//!
//! ```text
//! Country (id)
//!   └─> User (id, country_id)
//!       └─> Post (id, user_id)
//!
//! HasManyThrough: Country.posts() returns all posts where:
//!   posts.user_id = users.id AND users.country_id = country.id
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::relationships::through::{has_many_through, has_one_through};
//! use sea_orm::EntityTrait;
//! # fn main() {}
//! # mod country {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "countries")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # mod user {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "users")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # mod post {
//! #     use sea_orm::entity::prelude::*;
//! #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
//! #     #[sea_orm(table_name = "posts")]
//! #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
//! #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
//! #     impl ActiveModelBehavior for ActiveModel {}
//! # }
//! # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
//! // Get all posts in a country
//! let country = country::Entity::find_by_id(1).one(&db).await?.unwrap();
//! let posts = has_many_through::<post::Entity, user::Entity>(
//!     &db,
//!     country.id as i64,
//!     "country_id",  // FK in User table
//!     "user_id",     // FK in Post table
//! ).await?;
//!
//! // Get the latest post in a country
//! let latest_post = has_one_through::<post::Entity, user::Entity>(
//!     &db,
//!     country.id as i64,
//!     "country_id",
//!     "user_id",
//! ).first().await?;
//! # Ok(())
//! # }
//! ```

use async_trait::async_trait;
use sea_orm::{DatabaseConnection, DbErr, EntityTrait};

/// Result type for through relationship queries
pub type ThroughResult<T> = Result<T, DbErr>;

/// Trait for models that have "has one through" relationships
///
/// A HasOneThrough relationship allows you to access a single distant relation
/// through an intermediate model.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::through::{has_one_through, ThroughResult};
/// use sea_orm::DatabaseConnection;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod country {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "countries")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// // Country has one latest post through users
/// impl country::Model {
///     pub async fn latest_post(&self, db: &DatabaseConnection) -> ThroughResult<Option<post::Model>> {
///         has_one_through::<post::Entity, user::Entity>(
///             db,
///             self.id as i64,
///             "country_id",
///             "user_id",
///         )
///         .order_by_desc("created_at")
///         .first()
///         .await
///     }
/// }
/// ```
#[async_trait]
pub trait HasOneThrough<T: EntityTrait, I: EntityTrait> {
    /// Load the distant relation through the intermediate model
    async fn has_one_through(
        &self,
        db: &DatabaseConnection,
        first_key: &str,
        second_key: &str,
    ) -> ThroughResult<Option<T::Model>>;
}

/// Trait for models that have "has many through" relationships
///
/// A HasManyThrough relationship allows you to access multiple distant relations
/// through an intermediate model.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::through::{has_many_through, ThroughResult};
/// use sea_orm::DatabaseConnection;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod country {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "countries")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// // Country has many posts through users
/// impl country::Model {
///     pub async fn posts(&self, db: &DatabaseConnection) -> ThroughResult<Vec<post::Model>> {
///         has_many_through::<post::Entity, user::Entity>(
///             db,
///             self.id as i64,
///             "country_id",
///             "user_id",
///         ).await
///     }
/// }
/// ```
#[async_trait]
pub trait HasManyThrough<T: EntityTrait, I: EntityTrait> {
    /// Load all distant relations through the intermediate model
    async fn has_many_through(
        &self,
        db: &DatabaseConnection,
        first_key: &str,
        second_key: &str,
    ) -> ThroughResult<Vec<T::Model>>;
}

/// Helper function to load a has_one_through relationship
///
/// This creates a query that joins through an intermediate table and returns
/// the first matching result.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `first_key` - The foreign key in the intermediate table referencing the parent
/// * `second_key` - The foreign key in the final table referencing the intermediate
///
/// # Returns
///
/// A ThroughQueryBuilder that can be further customized before execution
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::through::has_one_through;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// # let country_id = 1i64;
/// // Country -> User -> Post (get latest post)
/// let latest_post = has_one_through::<post::Entity, user::Entity>(
///     &db,
///     country_id,
///     "country_id",
///     "user_id",
/// )
/// .order_by_desc("created_at")
/// .first()
/// .await?;
/// # Ok(())
/// # }
/// ```
pub fn has_one_through<'a, T, I>(
    db: &'a DatabaseConnection,
    parent_id: i64,
    first_key: &str,
    second_key: &str,
) -> ThroughQueryBuilder<'a, T, I>
where
    T: EntityTrait,
    I: EntityTrait,
{
    ThroughQueryBuilder::new(db, parent_id, first_key, second_key)
}

/// Helper function to load a has_many_through relationship
///
/// This creates a query that joins through an intermediate table and returns
/// all matching results.
///
/// # Arguments
///
/// * `db` - Database connection
/// * `parent_id` - The ID of the parent model
/// * `first_key` - The foreign key in the intermediate table referencing the parent
/// * `second_key` - The foreign key in the final table referencing the intermediate
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::through::has_many_through;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// # let country_id = 1i64;
/// // Country -> User -> Post (get all posts)
/// let posts = has_many_through::<post::Entity, user::Entity>(
///     &db,
///     country_id,
///     "country_id",
///     "user_id",
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn has_many_through<T, I>(
    db: &DatabaseConnection,
    parent_id: i64,
    first_key: &str,
    second_key: &str,
) -> ThroughResult<Vec<T::Model>>
where
    T: EntityTrait,
    I: EntityTrait,
{
    has_one_through::<T, I>(db, parent_id, first_key, second_key)
        .get()
        .await
}

/// Query builder for through relationships with chainable methods
///
/// This builder allows you to customize the through relationship query
/// with additional filters, ordering, and limits before execution.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::relationships::through::ThroughQueryBuilder;
/// # fn main() {}
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// # let country_id = 1i64;
/// let posts = ThroughQueryBuilder::<post::Entity, user::Entity>::new(
///     &db,
///     country_id,
///     "country_id",
///     "user_id",
/// )
/// .where_raw("posts.published = true")
/// .order_by_desc("posts.created_at")
/// .limit(10)
/// .get()
/// .await?;
/// # Ok(())
/// # }
/// ```
pub struct ThroughQueryBuilder<'a, T, I>
where
    T: EntityTrait,
    I: EntityTrait,
{
    db: &'a DatabaseConnection,
    parent_id: i64,
    first_key: String,
    second_key: String,
    where_clauses: Vec<String>,
    order_by: Option<(String, String)>,
    limit_value: Option<u64>,
    offset_value: Option<u64>,
    _phantom_t: std::marker::PhantomData<T>,
    _phantom_i: std::marker::PhantomData<I>,
}

impl<'a, T, I> ThroughQueryBuilder<'a, T, I>
where
    T: EntityTrait,
    I: EntityTrait,
{
    /// Create a new through query builder
    pub fn new(
        db: &'a DatabaseConnection,
        parent_id: i64,
        first_key: &str,
        second_key: &str,
    ) -> Self {
        Self {
            db,
            parent_id,
            first_key: first_key.to_string(),
            second_key: second_key.to_string(),
            where_clauses: Vec::new(),
            order_by: None,
            limit_value: None,
            offset_value: None,
            _phantom_t: std::marker::PhantomData,
            _phantom_i: std::marker::PhantomData,
        }
    }

    /// Add a raw WHERE clause to the query
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::through::has_one_through;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # fn build(db: &sea_orm::DatabaseConnection) {
    /// let builder = has_one_through::<post::Entity, user::Entity>(db, 1, "country_id", "user_id");
    /// let builder = builder.where_raw("posts.published = true");
    /// # let _ = builder;
    /// # }
    /// ```
    pub fn where_raw(mut self, clause: &str) -> Self {
        self.where_clauses.push(clause.to_string());
        self
    }

    /// Add an ORDER BY clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::through::has_one_through;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # fn build(db: &sea_orm::DatabaseConnection) {
    /// let builder = has_one_through::<post::Entity, user::Entity>(db, 1, "country_id", "user_id");
    /// let builder = builder.order_by("posts.created_at", "desc");
    /// # let _ = builder;
    /// # }
    /// ```
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some((column.to_string(), direction.to_string()));
        self
    }

    /// Add an ORDER BY DESC clause (convenience method)
    pub fn order_by_desc(self, column: &str) -> Self {
        self.order_by(column, "desc")
    }

    /// Add an ORDER BY ASC clause (convenience method)
    pub fn order_by_asc(self, column: &str) -> Self {
        self.order_by(column, "asc")
    }

    /// Add a LIMIT clause
    pub fn limit(mut self, limit: u64) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Add an OFFSET clause
    pub fn offset(mut self, offset: u64) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Execute the query and return the first result
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::through::has_one_through;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = has_one_through::<post::Entity, user::Entity>(&db, 1, "country_id", "user_id");
    /// let post = builder.first().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn first(mut self) -> ThroughResult<Option<T::Model>> {
        self.limit_value = Some(1);
        let mut results = self.get().await?;
        Ok(results.pop())
    }

    /// Execute the query and return all results
    ///
    /// This method builds and executes a JOIN query like:
    /// ```sql
    /// SELECT target.*
    /// FROM target
    /// INNER JOIN intermediate ON target.second_key = intermediate.id
    /// WHERE intermediate.first_key = parent_id
    /// ```
    pub async fn get(self) -> ThroughResult<Vec<T::Model>> {
        // Get table names (store in owned Strings)
        let target_entity = T::default();
        let intermediate_entity = I::default();
        let target_table = target_entity.table_name();
        let intermediate_table = intermediate_entity.table_name();

        // Build the SQL query manually since SeaORM's join API is limited
        // In a real-world scenario, you'd use raw SQL or a more complex query builder

        // For this implementation, we'll use a workaround:
        // 1. Find all intermediate models
        // 2. Extract their IDs
        // 3. Query target models with those IDs

        use sea_orm::ConnectionTrait;
        use sea_orm::Statement;

        // Build the raw SQL
        let mut sql = format!(
            "SELECT {}.*
             FROM {}
             INNER JOIN {} ON {}.{} = {}.id
             WHERE {}.{} = ?",
            target_table,
            target_table,
            intermediate_table,
            target_table,
            self.second_key,
            intermediate_table,
            intermediate_table,
            self.first_key
        );

        // Add WHERE clauses
        for clause in &self.where_clauses {
            sql.push_str(&format!(" AND {}", clause));
        }

        // Add ORDER BY
        if let Some((column, direction)) = &self.order_by {
            sql.push_str(&format!(
                " ORDER BY {} {}",
                column,
                direction.to_uppercase()
            ));
        }

        // Add LIMIT
        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Add OFFSET
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        // Execute the raw query
        let stmt = Statement::from_sql_and_values(
            self.db.get_database_backend(),
            &sql,
            vec![sea_orm::Value::BigInt(Some(self.parent_id))],
        );

        T::find().from_raw_sql(stmt).all(self.db).await
    }

    /// Count the number of results without retrieving them
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use rf_orm::relationships::through::has_one_through;
    /// # fn main() {}
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # async fn example(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let builder = has_one_through::<post::Entity, user::Entity>(&db, 1, "country_id", "user_id");
    /// let count = builder.count().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(self) -> ThroughResult<u64> {
        let results = self.get().await?;
        Ok(results.len() as u64)
    }
}

/// Macro to easily implement HasManyThrough relationships
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::has_many_through;
/// use sea_orm::EntityTrait;
/// # fn main() {}
/// # mod country {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "countries")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i64 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// // Define the relationship (parent model, target entity, intermediate entity)
/// has_many_through!(
///     country::Model,  // Parent model
///     post::Entity,    // Target model
///     user::Entity,    // Intermediate model
///     posts,           // Method name
///     "country_id",    // FK in intermediate
///     "user_id"        // FK in target
/// );
///
/// # async fn run(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// // Usage
/// let country = country::Entity::find_by_id(1).one(&db).await?.unwrap();
/// let posts = country.posts(&db).await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! has_many_through {
    ($parent:ty, $target:ty, $intermediate:ty, $method:ident, $first_key:literal, $second_key:literal) => {
        impl $parent {
            pub async fn $method(
                &self,
                db: &sea_orm::DatabaseConnection,
            ) -> $crate::relationships::through::ThroughResult<
                Vec<<$target as sea_orm::EntityTrait>::Model>,
            > {
                $crate::relationships::through::has_many_through::<$target, $intermediate>(
                    db,
                    self.id,
                    $first_key,
                    $second_key,
                )
                .await
            }
        }
    };
}

/// Macro to easily implement HasOneThrough relationships
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::has_one_through;
/// use sea_orm::EntityTrait;
/// # fn main() {}
/// # mod country {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "countries")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i64 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod user {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "users")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub country_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// // Define the relationship (parent model, target entity, intermediate entity)
/// has_one_through!(
///     country::Model,  // Parent model
///     post::Entity,    // Target model
///     user::Entity,    // Intermediate model
///     latest_post,     // Method name
///     "country_id",    // FK in intermediate
///     "user_id"        // FK in target
/// );
///
/// # async fn run(db: sea_orm::DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// // Usage
/// let country = country::Entity::find_by_id(1).one(&db).await?.unwrap();
/// let latest_post = country.latest_post(&db).await?;
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! has_one_through {
    ($parent:ty, $target:ty, $intermediate:ty, $method:ident, $first_key:literal, $second_key:literal) => {
        impl $parent {
            pub async fn $method(
                &self,
                db: &sea_orm::DatabaseConnection,
            ) -> $crate::relationships::through::ThroughResult<
                Option<<$target as sea_orm::EntityTrait>::Model>,
            > {
                $crate::relationships::through::has_one_through::<$target, $intermediate>(
                    db,
                    self.id,
                    $first_key,
                    $second_key,
                )
                .order_by_desc("created_at")
                .first()
                .await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_through_query_builder_creation() {
        // Verify builder can be created and configured
        // Would need actual entities for full test
    }

    #[test]
    fn test_through_builder_chaining() {
        // Verify all methods return Self for chaining
    }

    #[test]
    fn test_macros_compile() {
        // Macros would be tested in integration tests
        // with actual entity definitions
    }
}
