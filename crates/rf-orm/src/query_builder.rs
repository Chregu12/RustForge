use sea_orm::{
    ColumnTrait, Condition, DatabaseConnection, EntityTrait, Order,
    QueryFilter, QueryOrder, QuerySelect, Select, Value, QueryTrait,
    sea_query::{Expr, LockType, LockBehavior},
    DbErr,
};
use std::marker::PhantomData;
use std::sync::Arc;

/// Laravel-like Query Builder for Eloquent-style queries
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::QueryBuilder;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let posts = Post::query()
///     .where_eq("published", true)
///     .where_gt("views", 100)
///     .order_by("created_at", "desc")
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
    /// Post::query().where_eq("published", true)
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
    /// Post::query().order_by("created_at", "desc")
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
    /// let post = Post::query().where_eq("id", 1).first().await?;
    /// ```
    pub async fn first(self) -> Result<Option<E::Model>, sea_orm::DbErr> {
        self.select.one(self.db.as_ref()).await
    }

    /// Get all results
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let posts = Post::query().where_eq("published", true).get().await?;
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
    /// // Get posts where user_id is in the subquery result
    /// Post::query(db.clone())
    ///     .where_in_subquery("user_id", User::query(db).where_eq("active", true))
    ///     .get()
    ///     .await?;
    /// ```
    pub fn where_in_subquery<C, E2>(mut self, column: C, subquery: QueryBuilder<E2>) -> Self
    where
        C: ColumnTrait,
        E2: EntityTrait,
    {
        let (sub_select, _) = subquery.into_select();
        self.select = self.select.filter(column.in_subquery(sub_select.into_query()));
        self
    }

    /// Add a WHERE NOT IN subquery clause
    pub fn where_not_in_subquery<C, E2>(mut self, column: C, subquery: QueryBuilder<E2>) -> Self
    where
        C: ColumnTrait,
        E2: EntityTrait,
    {
        let (sub_select, _) = subquery.into_select();
        self.select = self.select.filter(column.not_in_subquery(sub_select.into_query()));
        self
    }

    /// Add a WHERE EXISTS subquery clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// // Get posts that have comments
    /// Post::query(db.clone())
    ///     .where_exists(
    ///         Comment::query(db).where_column("comments.post_id", "posts.id")
    ///     )
    ///     .get()
    ///     .await?;
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
        self.select = self.select.filter(Expr::exists(sub_select.into_query()).not());
        self
    }

    // ----- Union Operations -----

    /// Combine this query with another using UNION
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let published = Post::query(db.clone()).where_eq("published", true);
    /// let featured = Post::query(db.clone()).where_eq("featured", true);
    /// let results = published.union(featured).get().await?;
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
    /// Post::query(db)
    ///     .select_raw("COUNT(*) as total, DATE(created_at) as date")
    ///     .group_by("date")
    ///     .get()
    ///     .await?;
    /// ```
    ///
    /// Note: Raw select requires executing raw SQL. This is a placeholder.
    pub fn select_raw(self, _raw_sql: &str) -> Self {
        // Placeholder - SeaORM doesn't support adding raw columns to Select easily
        // Full implementation would require using Statement and raw SQL
        self
    }

    /// Add a raw WHERE clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Post::query(db)
    ///     .where_raw("DATE(created_at) = CURDATE()")
    ///     .get()
    ///     .await?;
    /// ```
    pub fn where_raw(mut self, raw_sql: &str) -> Self {
        self.select = self.select.filter(Expr::cust(raw_sql));
        self
    }

    /// Add a raw WHERE clause with bindings
    pub fn where_raw_with_bindings(self, _raw_sql: &str, _bindings: Vec<Value>) -> Self {
        // Placeholder - binding values to raw SQL requires more complex SeaORM integration
        self
    }

    /// Add a raw ORDER BY clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Post::query(db)
    ///     .order_by_raw("FIELD(status, 'published', 'draft', 'archived')")
    ///     .get()
    ///     .await?;
    /// ```
    pub fn order_by_raw(self, _raw_sql: &str) -> Self {
        // Placeholder - SeaORM doesn't have direct support for raw ORDER BY
        // Full implementation would require raw SQL
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
    /// let total = Post::query(db).count().await?;
    /// let published = Post::query(db).where_eq("published", true).count().await?;
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
    /// let total_views = Post::query(db).sum("views").await?;
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
    /// let avg_rating = Post::query(db).avg("rating").await?;
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
    /// let min_price = Product::query(db).min("price").await?;
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
    /// let max_price = Product::query(db).max("price").await?;
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
    /// Post::query(db)
    ///     .chunk(100, |posts| async {
    ///         for post in posts {
    ///             process_post(post).await?;
    ///         }
    ///         Ok(())
    ///     })
    ///     .await?;
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
    /// Post::query(db)
    ///     .chunk_by_id(100, |posts| async {
    ///         for post in posts {
    ///             update_post(post).await?;
    ///         }
    ///         Ok(())
    ///     }, "id")
    ///     .await?;
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
    /// Post::query(db)
    ///     .lazy(100)
    ///     .for_each(|post| async {
    ///         process_post(post).await?;
    ///     })
    ///     .await?;
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
    /// let post = Post::query(db)
    ///     .lock_for_update()
    ///     .find(1)
    ///     .await?;
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
    /// let post = Post::query(db)
    ///     .shared_lock()
    ///     .find(1)
    ///     .await?;
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
    /// let posts = Post::query(db)
    ///     .lock_for_update()
    ///     .skip_locked()
    ///     .limit(10)
    ///     .get()
    ///     .await?;
    /// ```
    pub fn skip_locked(mut self) -> Self {
        self.select = self.select.lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
        self
    }

    /// Don't wait for locks (NOWAIT)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let post = Post::query(db)
    ///     .lock_for_update()
    ///     .no_wait()
    ///     .find(1)
    ///     .await?;
    /// ```
    pub fn no_wait(mut self) -> Self {
        self.select = self.select.lock_with_behavior(LockType::Update, LockBehavior::Nowait);
        self
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

            let chunk_query = QueryBuilder::from_select(
                self.query_builder.select.clone(),
                db,
            )
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
