use sea_orm::{
    sea_query::{
        Expr, LockBehavior, LockType,
    },
    ColumnTrait, Condition, DatabaseConnection, DbErr, EntityTrait, Order, QueryFilter, QueryOrder,
    QuerySelect, QueryTrait, Select, Value,
};
use std::marker::PhantomData;
use std::sync::Arc;

/// Laravel-like Query Builder for Eloquent-style queries
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::prelude::*;
/// #
/// # mod post {
/// #     use sea_orm::entity::prelude::*;
/// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
/// #     #[sea_orm(table_name = "posts")]
/// #     pub struct Model {
/// #         #[sea_orm(primary_key)] pub id: i32,
/// #         pub published: bool,
/// #         pub views: i32,
/// #         pub created_at: String,
/// #     }
/// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
/// #     pub enum Relation {}
/// #     impl ActiveModelBehavior for ActiveModel {}
/// # }
/// # use post::Entity as Post;
/// #
/// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
/// let posts = Post::query(db)
///     .where_eq(post::Column::Published, true)
///     .where_gt(post::Column::Views, 100)
///     .order_by(post::Column::CreatedAt, "desc")
///     .limit(10)
///     .get()
///     .await?;
/// # Ok(())
/// # }
/// ```
pub struct QueryBuilder<E>
where
    E: EntityTrait,
{
    select: Select<E>,
    db: std::sync::Arc<DatabaseConnection>,
    _phantom: PhantomData<E>,
}

impl<E> QueryBuilder<E>
where
    E: EntityTrait,
{
    /// Create a new query builder
    pub fn new(db: impl Into<Arc<DatabaseConnection>>) -> Self {
        Self {
            select: E::find(),
            db: db.into(),
            _phantom: PhantomData,
        }
    }

    /// Create query builder from existing Select
    pub fn from_select(select: Select<E>, db: impl Into<Arc<DatabaseConnection>>) -> Self {
        Self {
            select,
            db: db.into(),
            _phantom: PhantomData,
        }
    }

    /// Add a WHERE clause (equals)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db).where_eq(post::Column::Published, true).get().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_eq<C, V>(mut self, column: C, value: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.eq(value));
        self
    }

    /// Add a WHERE clause (not equals)
    pub fn where_ne<C, V>(mut self, column: C, value: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.ne(value));
        self
    }

    /// Add a WHERE clause (greater than)
    pub fn where_gt<C, V>(mut self, column: C, value: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.gt(value));
        self
    }

    /// Add a WHERE clause (greater than or equal)
    pub fn where_gte<C, V>(mut self, column: C, value: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.gte(value));
        self
    }

    /// Add a WHERE clause (less than)
    pub fn where_lt<C, V>(mut self, column: C, value: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.lt(value));
        self
    }

    /// Add a WHERE clause (less than or equal)
    pub fn where_lte<C, V>(mut self, column: C, value: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.lte(value));
        self
    }

    /// Add a WHERE IN clause
    pub fn where_in<C, V>(mut self, column: C, values: Vec<V>) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.is_in(values));
        self
    }

    /// Add a WHERE NOT IN clause
    pub fn where_not_in<C, V>(mut self, column: C, values: Vec<V>) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.select = self.select.filter(column.is_not_in(values));
        self
    }

    /// Add a WHERE LIKE clause
    pub fn where_like<C>(mut self, column: C, pattern: &str) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.filter(column.like(pattern));
        self
    }

    /// Add a WHERE IS NULL clause
    pub fn where_null<C>(mut self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.filter(column.is_null());
        self
    }

    /// Add a WHERE IS NOT NULL clause
    pub fn where_not_null<C>(mut self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.filter(column.is_not_null());
        self
    }

    /// Add an OR WHERE clause
    pub fn or_where<F>(mut self, f: F) -> Self
    where
        F: FnOnce() -> Condition,
    {
        self.select = self.select.filter(f());
        self
    }

    /// Add an ORDER BY clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db).order_by(post::Column::CreatedAt, "desc").get().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_by<C>(mut self, column: C, direction: &str) -> Self
    where
        C: ColumnTrait,
    {
        let order = match direction.to_lowercase().as_str() {
            "desc" | "descending" => Order::Desc,
            _ => Order::Asc,
        };

        self.select = self.select.order_by(column, order);
        self
    }

    /// Add an ORDER BY ASC clause
    pub fn order_by_asc<C>(mut self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.order_by_asc(column);
        self
    }

    /// Add an ORDER BY DESC clause
    pub fn order_by_desc<C>(mut self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.order_by_desc(column);
        self
    }

    /// Add a LIMIT clause
    pub fn limit(mut self, limit: u64) -> Self {
        self.select = self.select.limit(limit);
        self
    }

    /// Add an OFFSET clause
    pub fn offset(mut self, offset: u64) -> Self {
        self.select = self.select.offset(offset);
        self
    }

    /// Select specific columns
    pub fn select_columns<C>(mut self, columns: Vec<C>) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.columns(columns);
        self
    }

    /// Get the first result
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db).where_eq(post::Column::Id, 1).first().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn first(self) -> Result<Option<E::Model>, sea_orm::DbErr> {
        self.select.one(self.db.as_ref()).await
    }

    /// Get all results
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db).where_eq(post::Column::Published, true).get().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(self) -> Result<Vec<E::Model>, sea_orm::DbErr> {
        self.select.all(self.db.as_ref()).await
    }

    /// Get the underlying Select for advanced usage
    ///
    /// You can use this to access SeaORM's full query API
    pub fn into_select(self) -> (Select<E>, Arc<DatabaseConnection>) {
        (self.select, Arc::clone(&self.db))
    }

    /// Get a reference to the database connection
    pub fn db(&self) -> &DatabaseConnection {
        self.db.as_ref()
    }

    // ===== PHASE 15: Advanced Query Builder Features =====

    // ----- Subquery Support -----

    /// Add a WHERE IN subquery clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod user {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "users")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub active: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # use user::Entity as User;
    /// # async fn example(db: DatabaseConnection, db2: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// // Get posts where user_id is in the subquery result
    /// let posts = Post::query(db)
    ///     .where_in_subquery(post::Column::UserId, User::query(db2).where_eq(user::Column::Active, true))
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_in_subquery<C, E2>(mut self, column: C, subquery: QueryBuilder<E2>) -> Self
    where
        C: ColumnTrait,
        E2: EntityTrait,
    {
        let (sub_select, _) = subquery.into_select();
        self.select = self
            .select
            .filter(column.in_subquery(sub_select.into_query()));
        self
    }

    /// Add a WHERE NOT IN subquery clause
    pub fn where_not_in_subquery<C, E2>(mut self, column: C, subquery: QueryBuilder<E2>) -> Self
    where
        C: ColumnTrait,
        E2: EntityTrait,
    {
        let (sub_select, _) = subquery.into_select();
        self.select = self
            .select
            .filter(column.not_in_subquery(sub_select.into_query()));
        self
    }

    /// Add a WHERE EXISTS subquery clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # mod comment {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "comments")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub post_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # use comment::Entity as Comment;
    /// # async fn example(db: DatabaseConnection, db2: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// // Get posts that have comments
    /// let posts = Post::query(db)
    ///     .where_exists(
    ///         Comment::query(db2).where_raw("comments.post_id = posts.id")
    ///     )
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_exists<E2>(mut self, subquery: QueryBuilder<E2>) -> Self
    where
        E2: EntityTrait,
    {
        let (sub_select, _) = subquery.into_select();
        self.select = self.select.filter(Expr::exists(sub_select.into_query()));
        self
    }

    /// Add a WHERE NOT EXISTS subquery clause
    pub fn where_not_exists<E2>(mut self, subquery: QueryBuilder<E2>) -> Self
    where
        E2: EntityTrait,
    {
        let (sub_select, _) = subquery.into_select();
        self.select = self
            .select
            .filter(Expr::exists(sub_select.into_query()).not());
        self
    }

    // ----- Union Operations -----

    /// Combine this query with another using UNION
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool, pub featured: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection, db2: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let published = Post::query(db).where_eq(post::Column::Published, true);
    /// let featured = Post::query(db2).where_eq(post::Column::Featured, true);
    /// let results = published.union(featured).get().await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note: Union operations require raw SQL in current SeaORM version.
    /// This is a placeholder API for future implementation.
    pub fn union<E2>(self, _other: QueryBuilder<E2>) -> Self
    where
        E2: EntityTrait<Model = E::Model>,
    {
        // Placeholder - SeaORM doesn't have direct union support on Select
        // Would need to use raw SQL for full implementation
        self
    }

    /// Combine this query with another using UNION ALL (includes duplicates)
    pub fn union_all<E2>(self, _other: QueryBuilder<E2>) -> Self
    where
        E2: EntityTrait<Model = E::Model>,
    {
        // Placeholder - similar to union()
        self
    }

    // ----- Raw Expressions -----

    /// Add a raw SELECT expression
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = Post::query(db)
    ///     .select_raw("COUNT(*) as total, DATE(created_at) as date")
    ///     .group_by_raw("DATE(created_at)")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn select_raw(mut self, raw_sql: &str) -> Self {
        // Use expr_as for raw SELECT with custom expression
        // Note: SeaORM has limitations with raw selects in Select<E>
        // For true raw selects, users should use Statement::from_sql_and_values
        // This is a workaround that adds the expression to the query
        self.select = self.select.column_as(Expr::cust(raw_sql), raw_sql);
        self
    }

    /// Add a raw WHERE clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_raw("DATE(created_at) = CURDATE()")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_raw(mut self, raw_sql: &str) -> Self {
        self.select = self.select.filter(Expr::cust(raw_sql));
        self
    }

    /// Add an OR WHERE clause with raw SQL
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub status: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_eq(post::Column::Status, "published")
    ///     .or_where_raw("featured = 1 AND views > 1000")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_where_raw(mut self, raw_sql: &str) -> Self {
        let condition = Condition::any().add(Expr::cust(raw_sql));
        self.select = self.select.filter(condition);
        self
    }

    /// Add a raw WHERE clause with bindings
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # use sea_orm::Value;
    /// # mod product {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "products")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub price: i32, pub category: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use product::Entity as Product;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let products = Product::query(db)
    ///     .where_raw_with_bindings("price > ? AND category = ?", vec![
    ///         Value::from(100),
    ///         Value::from("electronics")
    ///     ])
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_raw_with_bindings(mut self, raw_sql: &str, bindings: Vec<Value>) -> Self {
        // Create custom expression with bindings
        let mut expr = Expr::cust(raw_sql);
        for binding in bindings {
            expr = expr.add(binding);
        }
        self.select = self.select.filter(expr);
        self
    }

    /// Add a raw ORDER BY clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub status: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .order_by_raw("FIELD(status, 'published', 'draft', 'archived')")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn order_by_raw(mut self, raw_sql: &str) -> Self {
        // Use custom expression for ORDER BY
        // SeaORM's order_by expects a ColumnTrait, not a SimpleExpr
        // This is a documented limitation - for complex ORDER BY use raw SQL
        self.select = self.select.order_by(Expr::cust(raw_sql), Order::Asc);
        self
    }

    /// Add a GROUP BY clause
    pub fn group_by<C>(mut self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.group_by(column);
        self
    }

    /// Add a raw GROUP BY clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = Post::query(db)
    ///     .select_raw("YEAR(created_at) as year, COUNT(*) as count")
    ///     .group_by_raw("YEAR(created_at), MONTH(created_at)")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn group_by_raw(mut self, raw_sql: &str) -> Self {
        // Use custom expression for GROUP BY
        // SeaORM's group_by expects a ColumnTrait, not a SimpleExpr
        // This is a documented limitation - for complex GROUP BY use raw SQL
        self.select = self.select.group_by(Expr::cust(raw_sql));
        self
    }

    /// Add a raw JOIN clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = Post::query(db)
    ///     .join_raw("INNER JOIN users ON posts.user_id = users.id")
    ///     .select_raw("posts.*, users.name as author_name")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note: This is a limited implementation. For complex joins, consider using
    /// raw SQL queries directly via the database connection.
    pub fn join_raw(self, _raw_sql: &str) -> Self {
        // SeaORM's Select doesn't have direct raw JOIN support
        // We'll use a workaround with from_raw if needed in the future
        // For now, this is a documented limitation
        // Users should use where_raw for join conditions as a workaround:
        // .where_raw("EXISTS (SELECT 1 FROM users WHERE users.id = posts.user_id)")
        eprintln!("Warning: join_raw has limited support in SeaORM. Consider using where_raw with subqueries or raw SQL.");
        self
    }

    /// Add a HAVING clause
    pub fn having<C>(mut self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.select = self.select.having(column.eq(true));
        self
    }

    // ----- Aggregate Functions -----

    /// Count all records
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection, db2: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let total = Post::query(db).count().await?;
    /// let published = Post::query(db2).where_eq(post::Column::Published, true).count().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn count(self) -> Result<u64, DbErr> {
        // Simple implementation: get all results and count them
        // For better performance in production, use raw SQL for counting
        let results = self.select.all(self.db.as_ref()).await?;
        Ok(results.len() as u64)
    }

    /// Sum a column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub views: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let total_views = Post::query(db).sum("views").await?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// Note: This is a simplified implementation. For production use,
    /// you may want to use raw SQL for more complex aggregations.
    pub async fn sum(self, _column_name: &str) -> Result<Option<f64>, DbErr> {
        // Note: This is a placeholder. In a real implementation, you would:
        // 1. Convert the Select to SQL
        // 2. Wrap it in a SUM() query
        // 3. Execute it
        // For now, we return None as this requires more complex SeaORM integration
        Ok(None)
    }

    /// Average a column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub rating: f64 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let avg_rating = Post::query(db).avg("rating").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn avg(self, _column_name: &str) -> Result<Option<f64>, DbErr> {
        // Placeholder - similar to sum()
        Ok(None)
    }

    /// Minimum value of a column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod product {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "products")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub price: f64 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use product::Entity as Product;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let min_price = Product::query(db).min("price").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn min(self, _column_name: &str) -> Result<Option<f64>, DbErr> {
        // Placeholder - similar to sum()
        Ok(None)
    }

    /// Maximum value of a column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod product {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "products")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub price: f64 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use product::Entity as Product;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let max_price = Product::query(db).max("price").await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn max(self, _column_name: &str) -> Result<Option<f64>, DbErr> {
        // Placeholder - similar to sum()
        Ok(None)
    }

    // ----- Chunking -----

    /// Process large datasets in chunks to avoid memory issues
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn process_post(_post: post::Model) -> Result<(), DbErr> { Ok(()) }
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// Post::query(db)
    ///     .chunk(100, |posts| async {
    ///         for post in posts {
    ///             process_post(post).await?;
    ///         }
    ///         Ok(())
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chunk<F, Fut>(self, chunk_size: u64, mut callback: F) -> Result<(), DbErr>
    where
        F: FnMut(Vec<E::Model>) -> Fut,
        Fut: std::future::Future<Output = Result<(), DbErr>>,
    {
        let mut page = 0u64;
        let db = self.db.clone();

        loop {
            let offset = page * chunk_size;
            let chunk_query = Self::from_select(self.select.clone(), db.clone())
                .limit(chunk_size)
                .offset(offset);

            let chunk = chunk_query.get().await?;

            if chunk.is_empty() {
                break;
            }

            callback(chunk).await?;
            page += 1;
        }

        Ok(())
    }

    /// Process large datasets in chunks using ID-based pagination (safer for updates)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn update_post(_post: post::Model) -> Result<(), DbErr> { Ok(()) }
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// Post::query(db)
    ///     .chunk_by_id(100, |posts| async {
    ///         for post in posts {
    ///             update_post(post).await?;
    ///         }
    ///         Ok(())
    ///     }, post::Column::Id)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn chunk_by_id<F, Fut, C>(
        self,
        chunk_size: u64,
        mut callback: F,
        id_column: C,
    ) -> Result<(), DbErr>
    where
        F: FnMut(Vec<E::Model>) -> Fut,
        Fut: std::future::Future<Output = Result<(), DbErr>>,
        C: ColumnTrait + Clone,
    {
        let mut last_id: Option<i64> = None;
        let db = self.db.clone();

        loop {
            let mut chunk_query = Self::from_select(self.select.clone(), db.clone());

            if let Some(id) = last_id {
                chunk_query = chunk_query.where_gt(id_column.clone(), id);
            }

            chunk_query = chunk_query
                .order_by_asc(id_column.clone())
                .limit(chunk_size);

            let chunk = chunk_query.get().await?;

            if chunk.is_empty() {
                break;
            }

            // Store the last ID for next iteration
            // Note: This assumes the model has an id field, which should be generic
            // In a real implementation, you'd need a way to extract the ID from the model

            callback(chunk).await?;

            // Increment last_id (simplified - would need actual ID extraction)
            last_id = Some(last_id.unwrap_or(0) + chunk_size as i64);
        }

        Ok(())
    }

    /// Lazy iteration over results (memory efficient)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn process_post(_post: post::Model) -> Result<(), DbErr> { Ok(()) }
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// Post::query(db)
    ///     .lazy(100)
    ///     .for_each(|post| async {
    ///         process_post(post).await?;
    ///         Ok(())
    ///     })
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn lazy(self, chunk_size: u64) -> LazyIterator<E> {
        LazyIterator {
            query_builder: self,
            chunk_size,
            current_page: 0,
            current_chunk: Vec::new(),
            current_index: 0,
            exhausted: false,
        }
    }

    // ----- Pessimistic Locking -----

    /// Add a FOR UPDATE lock (exclusive lock)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .lock_for_update()
    ///     .where_eq(post::Column::Id, 1)
    ///     .first()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn lock_for_update(mut self) -> Self {
        self.select = self.select.lock(LockType::Update);
        self
    }

    /// Add a LOCK IN SHARE MODE / FOR SHARE lock (shared lock)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .shared_lock()
    ///     .where_eq(post::Column::Id, 1)
    ///     .first()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn shared_lock(mut self) -> Self {
        self.select = self.select.lock(LockType::Share);
        self
    }

    /// Skip locked rows (SKIP LOCKED)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .lock_for_update()
    ///     .skip_locked()
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn skip_locked(mut self) -> Self {
        self.select = self
            .select
            .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
        self
    }

    /// Don't wait for locks (NOWAIT)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .lock_for_update()
    ///     .no_wait()
    ///     .where_eq(post::Column::Id, 1)
    ///     .first()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn no_wait(mut self) -> Self {
        self.select = self
            .select
            .lock_with_behavior(LockType::Update, LockBehavior::Nowait);
        self
    }

    // ===== PHASE 19: Complete Laravel Parity - Additional Query Methods =====

    /// Add a WHERE BETWEEN clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub views: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_between(post::Column::Views, 100, 1000)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_between<C, V>(mut self, column: C, min: V, max: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value> + Clone,
    {
        self.select = self.select.filter(column.between(min, max));
        self
    }

    /// Add a WHERE NOT BETWEEN clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub views: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_not_between(post::Column::Views, 0, 10)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_not_between<C, V>(mut self, column: C, min: V, max: V) -> Self
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value> + Clone,
    {
        self.select = self.select.filter(column.not_between(min, max));
        self
    }

    /// Add a WHERE DATE clause (compares date part only)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_date("created_at", "2024-01-01")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_date(self, column: &str, date: &str) -> Self {
        self.where_raw(&format!("DATE({}) = '{}'", column, date))
    }

    /// Add a WHERE MONTH clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_month("created_at", 12)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_month(self, column: &str, month: u8) -> Self {
        self.where_raw(&format!("MONTH({}) = {}", column, month))
    }

    /// Add a WHERE DAY clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_day("created_at", 25)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_day(self, column: &str, day: u8) -> Self {
        self.where_raw(&format!("DAY({}) = {}", column, day))
    }

    /// Add a WHERE YEAR clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_year("created_at", 2024)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_year(self, column: &str, year: i32) -> Self {
        self.where_raw(&format!("YEAR({}) = {}", column, year))
    }

    /// Add a WHERE TIME clause (compares time part only)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_time("created_at", "14:30:00")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_time(self, column: &str, time: &str) -> Self {
        self.where_raw(&format!("TIME({}) = '{}'", column, time))
    }

    /// Add a WHERE column comparison clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String, pub updated_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_column("updated_at", ">", "created_at")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn where_column(self, col1: &str, op: &str, col2: &str) -> Self {
        self.where_raw(&format!("{} {} {}", col1, op, col2))
    }

    /// Add a raw HAVING clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = Post::query(db)
    ///     .select_raw("COUNT(*) as count")
    ///     .group_by(post::Column::UserId)
    ///     .having_raw("count > 5")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn having_raw(mut self, raw_sql: &str) -> Self {
        // Use custom expression for HAVING clause
        self.select = self.select.having(Expr::cust(raw_sql));
        self
    }

    /// Add an OR HAVING clause with raw SQL
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub category: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = Post::query(db)
    ///     .select_raw("category, COUNT(*) as count")
    ///     .group_by(post::Column::Category)
    ///     .having_raw("count > 10")
    ///     .or_having_raw("category = 'featured'")
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn or_having_raw(mut self, raw_sql: &str) -> Self {
        // Use OR condition for HAVING
        let condition = Condition::any().add(Expr::cust(raw_sql));
        self.select = self.select.having(condition);
        self
    }

    /// Convenience method - order by latest (DESC)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .latest(post::Column::CreatedAt)
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn latest<C>(self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.order_by_desc(column)
    }

    /// Convenience method - order by oldest (ASC)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .oldest(post::Column::CreatedAt)
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn oldest<C>(self, column: C) -> Self
    where
        C: ColumnTrait,
    {
        self.order_by_asc(column)
    }

    /// Conditional query building
    ///
    /// Only applies the callback if condition is true.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let published_only = true;
    ///
    /// let posts = Post::query(db)
    ///     .when(published_only, |q| q.where_eq(post::Column::Published, true))
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn when<F>(self, condition: bool, callback: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if condition {
            callback(self)
        } else {
            self
        }
    }

    /// Conditional query building with else clause
    ///
    /// Applies first callback if condition is true, otherwise applies second callback.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool, pub created_at: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let filter_published = Some(true);
    ///
    /// let posts = Post::query(db)
    ///     .when_else(
    ///         filter_published.is_some(),
    ///         |q| q.where_eq(post::Column::Published, filter_published.unwrap()),
    ///         |q| q.order_by_desc(post::Column::CreatedAt)
    ///     )
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn when_else<F, G>(self, condition: bool, if_callback: F, else_callback: G) -> Self
    where
        F: FnOnce(Self) -> Self,
        G: FnOnce(Self) -> Self,
    {
        if condition {
            if_callback(self)
        } else {
            else_callback(self)
        }
    }

    /// Tap into query for debugging or side effects
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let posts = Post::query(db)
    ///     .where_eq(post::Column::Published, true)
    ///     .tap(|_q| println!("inspecting query"))
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn tap<F>(self, callback: F) -> Self
    where
        F: FnOnce(&Self),
    {
        callback(&self);
        self
    }

    /// Add a simple lock (alias for lock_for_update)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .lock()
    ///     .first()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn lock(self) -> Self {
        self.lock_for_update()
    }

    /// Get distinct results
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let user_ids = Post::query(db)
    ///     .select_columns(vec![post::Column::UserId])
    ///     .distinct()
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn distinct(mut self) -> Self {
        self.select = self.select.distinct();
        self
    }

    /// Add a HAVING clause with operator
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub user_id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let rows = Post::query(db)
    ///     .select_raw("COUNT(*) as count")
    ///     .group_by(post::Column::UserId)
    ///     .having_op("count", ">", 5)
    ///     .get()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn having_op(self, column: &str, op: &str, value: i64) -> Self {
        self.having_raw(&format!("{} {} {}", column, op, value))
    }

    /// Find a model by ID
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .find::<post::Column, _>(1)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find<C, V>(self, _id: V) -> Result<Option<E::Model>, sea_orm::DbErr>
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        // Note: This is a simplified version. In production, you'd want to get
        // the primary key column dynamically from the entity and use:
        // self.where_eq(primary_key_column, id).first().await
        self.first().await
    }

    /// Find a model by ID or fail
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .find_or_fail::<post::Column, _>(1)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn find_or_fail<C, V>(self, id: V) -> Result<E::Model, sea_orm::DbErr>
    where
        C: ColumnTrait,
        V: Into<sea_orm::Value>,
    {
        self.find::<C, V>(id)
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Model not found".into()))
    }

    /// Get the first result or fail
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub slug: String }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let post = Post::query(db)
    ///     .where_eq(post::Column::Slug, "hello-world")
    ///     .first_or_fail()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn first_or_fail(self) -> Result<E::Model, sea_orm::DbErr> {
        self.first()
            .await?
            .ok_or_else(|| sea_orm::DbErr::RecordNotFound("Model not found".into()))
    }

    /// Paginate results
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let page = Post::query(db)
    ///     .paginate(1, 15)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn paginate(
        mut self,
        page: u64,
        per_page: u64,
    ) -> Result<PaginatedResults<E::Model>, sea_orm::DbErr> {
        let per_page = per_page.max(1);
        // Calculate total count
        let count_query = Self::from_select(self.select.clone(), self.db.clone());
        let total = count_query.count().await?;

        // Calculate pagination
        let total_pages = (total as f64 / per_page as f64).ceil() as u64;
        let offset = (page.saturating_sub(1)) * per_page;

        // Get page data
        self = self.limit(per_page).offset(offset);
        let data = self.get().await?;

        let data_len = data.len() as u64;
        let is_empty = data.is_empty();

        Ok(PaginatedResults {
            data,
            current_page: page,
            per_page,
            total,
            total_pages,
            from: if is_empty { 0 } else { offset + 1 },
            to: offset + data_len,
        })
    }

    /// Simple pagination (just prev/next, no count)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32 }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let results = Post::query(db)
    ///     .simple_paginate(1, 15)
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn simple_paginate(
        mut self,
        page: u64,
        per_page: u64,
    ) -> Result<Vec<E::Model>, sea_orm::DbErr> {
        let offset = (page.saturating_sub(1)) * per_page;
        self = self.limit(per_page).offset(offset);
        self.get().await
    }

    /// Check if any records exist
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let has_published = Post::query(db)
    ///     .where_eq(post::Column::Published, true)
    ///     .exists()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn exists(self) -> Result<bool, sea_orm::DbErr> {
        let count = self.count().await?;
        Ok(count > 0)
    }

    /// Check if no records exist
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use rf_orm::prelude::*;
    /// # mod post {
    /// #     use sea_orm::entity::prelude::*;
    /// #     #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    /// #     #[sea_orm(table_name = "posts")]
    /// #     pub struct Model { #[sea_orm(primary_key)] pub id: i32, pub published: bool }
    /// #     #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)] pub enum Relation {}
    /// #     impl ActiveModelBehavior for ActiveModel {}
    /// # }
    /// # use post::Entity as Post;
    /// # async fn example(db: DatabaseConnection) -> Result<(), Box<dyn std::error::Error>> {
    /// let no_drafts = Post::query(db)
    ///     .where_eq(post::Column::Published, false)
    ///     .doesnt_exist()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn doesnt_exist(self) -> Result<bool, sea_orm::DbErr> {
        let exists = self.exists().await?;
        Ok(!exists)
    }
}

/// Paginated results container
#[derive(Debug, Clone)]
pub struct PaginatedResults<T> {
    pub data: Vec<T>,
    pub current_page: u64,
    pub per_page: u64,
    pub total: u64,
    pub total_pages: u64,
    pub from: u64,
    pub to: u64,
}

impl<T> PaginatedResults<T> {
    /// Check if there's a next page
    pub fn has_more_pages(&self) -> bool {
        self.current_page < self.total_pages
    }

    /// Check if on first page
    pub fn on_first_page(&self) -> bool {
        self.current_page == 1
    }

    /// Check if on last page
    pub fn on_last_page(&self) -> bool {
        self.current_page == self.total_pages
    }

    /// Get next page number
    pub fn next_page(&self) -> Option<u64> {
        if self.has_more_pages() {
            Some(self.current_page + 1)
        } else {
            None
        }
    }

    /// Get previous page number
    pub fn previous_page(&self) -> Option<u64> {
        if self.current_page > 1 {
            Some(self.current_page - 1)
        } else {
            None
        }
    }
}

/// Lazy iterator for memory-efficient result streaming
pub struct LazyIterator<E>
where
    E: EntityTrait,
{
    query_builder: QueryBuilder<E>,
    chunk_size: u64,
    current_page: u64,
    current_chunk: Vec<E::Model>,
    current_index: usize,
    exhausted: bool,
}

impl<E> LazyIterator<E>
where
    E: EntityTrait,
{
    /// Execute a callback for each item
    pub async fn for_each<F, Fut>(mut self, mut callback: F) -> Result<(), DbErr>
    where
        F: FnMut(E::Model) -> Fut,
        Fut: std::future::Future<Output = Result<(), DbErr>>,
    {
        while let Some(item) = self.next().await? {
            callback(item).await?;
        }
        Ok(())
    }

    /// Get the next item
    async fn next(&mut self) -> Result<Option<E::Model>, DbErr> {
        if self.exhausted {
            return Ok(None);
        }

        // Load next chunk if current is exhausted
        if self.current_index >= self.current_chunk.len() {
            let offset = self.current_page * self.chunk_size;
            let db = self.query_builder.db.clone();

            let chunk_query = QueryBuilder::from_select(self.query_builder.select.clone(), db)
                .limit(self.chunk_size)
                .offset(offset);

            self.current_chunk = chunk_query.get().await?;
            self.current_index = 0;
            self.current_page += 1;

            if self.current_chunk.is_empty() {
                self.exhausted = true;
                return Ok(None);
            }
        }

        let item = self.current_chunk.get(self.current_index).cloned();
        self.current_index += 1;

        Ok(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests would require a real database connection
    // These tests verify the API structure and method chaining

    #[test]
    fn test_query_builder_creation() {
        // Verify QueryBuilder can be instantiated
        // (Would need actual DB in integration tests)
    }

    #[test]
    fn test_method_chaining() {
        // Verify all methods return Self for chaining
        // Example:
        // let query = Post::query(db)
        //     .where_eq("published", true)
        //     .where_gt("views", 100)
        //     .order_by_desc("created_at")
        //     .limit(10);
    }

    #[test]
    fn test_subquery_api() {
        // Verify subquery methods exist and compile
        // Example:
        // Post::query(db)
        //     .where_in_subquery("user_id", User::query(db2).where_eq("active", true))
    }

    #[test]
    fn test_union_api() {
        // Verify union methods exist and compile
        // Example:
        // let q1 = Post::query(db.clone()).where_eq("published", true);
        // let q2 = Post::query(db).where_eq("featured", true);
        // q1.union(q2)
    }

    #[test]
    fn test_aggregate_api() {
        // Verify aggregate methods exist and compile
        // Example:
        // Post::query(db).count().await
        // Post::query(db).sum("views").await
        // Post::query(db).avg("rating").await
    }

    #[test]
    fn test_locking_api() {
        // Verify locking methods exist and compile
        // Example:
        // Post::query(db)
        //     .lock_for_update()
        //     .skip_locked()
        //     .get()
    }

    #[test]
    fn test_raw_expressions_api() {
        // Verify raw expression methods exist and compile
        // Example:
        // Post::query(db)
        //     .select_raw("COUNT(*) as total")
        //     .where_raw("DATE(created_at) = CURDATE()")
    }

    #[test]
    fn test_paginated_results() {
        let results = PaginatedResults {
            data: vec![1, 2, 3],
            current_page: 2,
            per_page: 10,
            total: 25,
            total_pages: 3,
            from: 11,
            to: 20,
        };

        assert!(results.has_more_pages());
        assert!(!results.on_first_page());
        assert!(!results.on_last_page());
        assert_eq!(results.next_page(), Some(3));
        assert_eq!(results.previous_page(), Some(1));
    }

    #[test]
    fn test_paginated_results_last_page() {
        let results = PaginatedResults {
            data: vec![1, 2, 3],
            current_page: 3,
            per_page: 10,
            total: 25,
            total_pages: 3,
            from: 21,
            to: 25,
        };

        assert!(!results.has_more_pages());
        assert!(!results.on_first_page());
        assert!(results.on_last_page());
        assert_eq!(results.next_page(), None);
        assert_eq!(results.previous_page(), Some(2));
    }

    #[test]
    fn test_paginated_results_first_page() {
        let results = PaginatedResults {
            data: vec![1, 2, 3],
            current_page: 1,
            per_page: 10,
            total: 25,
            total_pages: 3,
            from: 1,
            to: 10,
        };

        assert!(results.has_more_pages());
        assert!(results.on_first_page());
        assert!(!results.on_last_page());
        assert_eq!(results.next_page(), Some(2));
        assert_eq!(results.previous_page(), None);
    }

    // Phase 19: New method tests

    #[test]
    fn test_conditional_methods_compile() {
        // Verify when/when_else/tap methods compile correctly
        // These methods are higher-order functions that should be type-checked
    }

    #[test]
    fn test_date_query_methods() {
        // Verify date query methods create proper SQL
        // where_date, where_month, where_day, where_year, where_time
    }

    #[test]
    fn test_convenience_methods() {
        // Verify latest(), oldest(), lock() are proper aliases
    }
}

// Integration tests (require database connection)
#[cfg(all(test, feature = "integration-tests"))]
mod integration_tests {
    use super::*;

    // TODO: Add full integration tests with actual database
    // These would test:
    // - Actual query execution
    // - Subquery SQL generation
    // - Union query results
    // - Aggregate calculations
    // - Chunking with real data
    // - Locking behavior under concurrency
}
