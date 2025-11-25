//! Query builder for constructing database queries
//!
//! Provides a Laravel-style fluent query builder for database operations.

use serde_json::Value;

/// Query builder for fluent database queries
///
/// # Examples
///
/// ```rust,no_run
/// use rf_db_facade::DB;
///
/// # async fn example() -> Result<(), String> {
/// // Select with conditions
/// let users = DB::table("users")
///     .where_clause("active", "=", true.into())
///     .order_by("name", "asc")
///     .limit(10)
///     .get().await?;
///
/// // Insert
/// let id = DB::table("users").insert(json!({
///     "name": "John",
///     "email": "john@example.com"
/// })).await?;
///
/// // Update
/// DB::table("users")
///     .where_clause("id", "=", 1.into())
///     .update(json!({"active": true})).await?;
///
/// // Delete
/// DB::table("users")
///     .where_clause("id", "=", 1.into())
///     .delete().await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct QueryBuilder {
    table: String,
    wheres: Vec<(String, String, Value)>,
    or_wheres: Vec<(String, String, Value)>,
    group_by: Vec<String>,
    having: Vec<(String, String, Value)>,
    limit_value: Option<usize>,
    offset_value: Option<usize>,
    order_by: Vec<(String, String)>,
    select_columns: Vec<String>,
}

impl QueryBuilder {
    /// Create a new query builder for a table
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            wheres: Vec::new(),
            or_wheres: Vec::new(),
            group_by: Vec::new(),
            having: Vec::new(),
            limit_value: None,
            offset_value: None,
            order_by: Vec::new(),
            select_columns: Vec::new(),
        }
    }

    /// Select specific columns
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .select(&["id", "name", "email"])
    ///     .get().await?;
    /// ```
    pub fn select(mut self, columns: &[&str]) -> Self {
        self.select_columns = columns.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a where clause - Laravel-style!
    ///
    /// With 2 arguments: `where("column", value)` means `column = value`
    /// With 3 arguments: `where_op("column", ">=", value)`
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Simple equality (like Laravel!)
    /// let users = DB::table("users")
    ///     .r#where("active", true)
    ///     .r#where("role", "admin")
    ///     .get().await?;
    /// ```
    pub fn r#where<V: Into<Value>>(mut self, column: impl Into<String>, value: V) -> Self {
        self.wheres.push((column.into(), "=".to_string(), value.into()));
        self
    }

    /// Alias for `r#where` - more readable without the r# prefix
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Clean syntax without r# prefix!
    /// let users = DB::table("users")
    ///     .filter("active", true)
    ///     .filter("role", "admin")
    ///     .get().await?;
    /// ```
    pub fn filter<V: Into<Value>>(self, column: impl Into<String>, value: V) -> Self {
        self.r#where(column, value)
    }

    /// Where with custom operator
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .where_op("age", ">=", 18)
    ///     .where_op("score", "<", 100)
    ///     .get().await?;
    /// ```
    pub fn where_op<V: Into<Value>>(mut self, column: impl Into<String>, operator: impl Into<String>, value: V) -> Self {
        self.wheres.push((column.into(), operator.into(), value.into()));
        self
    }

    /// Legacy method - use `r#where` instead
    #[deprecated(note = "Use `r#where` for Laravel-style syntax")]
    pub fn where_clause(mut self, column: impl Into<String>, operator: impl Into<String>, value: Value) -> Self {
        self.wheres.push((column.into(), operator.into(), value));
        self
    }

    /// Shorthand for where equals (same as `r#where`)
    pub fn where_eq<V: Into<Value>>(self, column: impl Into<String>, value: V) -> Self {
        self.r#where(column, value)
    }

    /// Where column is null
    pub fn where_null(mut self, column: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "IS".to_string(), Value::Null));
        self
    }

    /// Where column is not null
    pub fn where_not_null(mut self, column: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "IS NOT".to_string(), Value::Null));
        self
    }

    /// Where column is in a list of values
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .where_in("id", vec![1, 2, 3])
    ///     .get().await?;
    /// ```
    pub fn where_in<V: Into<Value>>(mut self, column: impl Into<String>, values: Vec<V>) -> Self {
        let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
        self.wheres.push((column.into(), "IN".to_string(), Value::Array(values)));
        self
    }

    /// Where column is not in a list
    pub fn where_not_in<V: Into<Value>>(mut self, column: impl Into<String>, values: Vec<V>) -> Self {
        let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
        self.wheres.push((column.into(), "NOT IN".to_string(), Value::Array(values)));
        self
    }

    /// Where column is between two values
    pub fn where_between<V: Into<Value>>(mut self, column: impl Into<String>, min: V, max: V) -> Self {
        let col = column.into();
        self.wheres.push((col.clone(), ">=".to_string(), min.into()));
        self.wheres.push((col, "<=".to_string(), max.into()));
        self
    }

    /// Where column is like a pattern
    pub fn where_like(mut self, column: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "LIKE".to_string(), Value::String(pattern.into())));
        self
    }

    /// Set limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit_value = Some(limit);
        self
    }

    /// Set offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset_value = Some(offset);
        self
    }

    /// Add order by clause
    pub fn order_by(mut self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by.push((column.into(), direction.into()));
        self
    }

    /// Laravel-style orderBy (camelCase alias)
    #[allow(non_snake_case)]
    pub fn orderBy(self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by(column, direction)
    }

    /// Order by ascending
    pub fn order_by_asc(self, column: impl Into<String>) -> Self {
        self.order_by(column, "ASC")
    }

    /// Order by descending
    pub fn order_by_desc(self, column: impl Into<String>) -> Self {
        self.order_by(column, "DESC")
    }

    // =========================================================================
    // Laravel-style camelCase aliases
    // =========================================================================

    /// Laravel-style whereIn (camelCase alias)
    #[allow(non_snake_case)]
    pub fn whereIn<V: Into<Value>>(self, column: impl Into<String>, values: Vec<V>) -> Self {
        self.where_in(column, values)
    }

    /// Laravel-style whereNotIn (camelCase alias)
    #[allow(non_snake_case)]
    pub fn whereNotIn<V: Into<Value>>(self, column: impl Into<String>, values: Vec<V>) -> Self {
        self.where_not_in(column, values)
    }

    /// Laravel-style whereBetween (camelCase alias)
    #[allow(non_snake_case)]
    pub fn whereBetween<V: Into<Value>>(self, column: impl Into<String>, min: V, max: V) -> Self {
        self.where_between(column, min, max)
    }

    /// Laravel-style whereNull (camelCase alias)
    #[allow(non_snake_case)]
    pub fn whereNull(self, column: impl Into<String>) -> Self {
        self.where_null(column)
    }

    /// Laravel-style whereNotNull (camelCase alias)
    #[allow(non_snake_case)]
    pub fn whereNotNull(self, column: impl Into<String>) -> Self {
        self.where_not_null(column)
    }

    /// Laravel-style whereLike (camelCase alias)
    #[allow(non_snake_case)]
    pub fn whereLike(self, column: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.where_like(column, pattern)
    }

    /// Laravel-style orderByAsc (camelCase alias)
    #[allow(non_snake_case)]
    pub fn orderByAsc(self, column: impl Into<String>) -> Self {
        self.order_by_asc(column)
    }

    /// Laravel-style orderByDesc (camelCase alias)
    #[allow(non_snake_case)]
    pub fn orderByDesc(self, column: impl Into<String>) -> Self {
        self.order_by_desc(column)
    }

    /// Laravel-style take() - alias for limit()
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users").take(5).get().await?;
    /// ```
    pub fn take(self, count: usize) -> Self {
        self.limit(count)
    }

    /// Laravel-style skip() - alias for offset()
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users").skip(10).take(5).get().await?;
    /// ```
    pub fn skip(self, count: usize) -> Self {
        self.offset(count)
    }

    /// Laravel-style latest() - order by created_at DESC
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::query().latest().take(10).get().await?;
    /// ```
    pub fn latest(self) -> Self {
        self.order_by("created_at", "DESC")
    }

    /// Laravel-style oldest() - order by created_at ASC
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::query().oldest().first().await?;
    /// ```
    pub fn oldest(self) -> Self {
        self.order_by("created_at", "ASC")
    }

    /// Laravel-style insertMany (camelCase alias)
    #[allow(non_snake_case)]
    pub async fn insertMany(self, data: Vec<Value>) -> Result<u64, String> {
        self.insert_many(data).await
    }

    // =========================================================================
    // OR Where conditions
    // =========================================================================

    /// Laravel-style orWhere - adds an OR condition
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::where("role", "admin")
    ///     .orWhere("role", "moderator")
    ///     .get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn orWhere<V: Into<Value>>(mut self, column: impl Into<String>, value: V) -> Self {
        self.or_wheres.push((column.into(), "=".to_string(), value.into()));
        self
    }

    /// orWhere with custom operator
    #[allow(non_snake_case)]
    pub fn orWhereOp<V: Into<Value>>(mut self, column: impl Into<String>, operator: impl Into<String>, value: V) -> Self {
        self.or_wheres.push((column.into(), operator.into(), value.into()));
        self
    }

    /// orWhereNull - column IS NULL with OR
    #[allow(non_snake_case)]
    pub fn orWhereNull(mut self, column: impl Into<String>) -> Self {
        self.or_wheres.push((column.into(), "IS".to_string(), Value::Null));
        self
    }

    /// orWhereNotNull - column IS NOT NULL with OR
    #[allow(non_snake_case)]
    pub fn orWhereNotNull(mut self, column: impl Into<String>) -> Self {
        self.or_wheres.push((column.into(), "IS NOT".to_string(), Value::Null));
        self
    }

    /// orWhereIn - column IN (...) with OR
    #[allow(non_snake_case)]
    pub fn orWhereIn<V: Into<Value>>(mut self, column: impl Into<String>, values: Vec<V>) -> Self {
        let values: Vec<Value> = values.into_iter().map(|v| v.into()).collect();
        self.or_wheres.push((column.into(), "IN".to_string(), Value::Array(values)));
        self
    }

    // =========================================================================
    // Group By and Having
    // =========================================================================

    /// Laravel-style groupBy
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let stats = DB::table("orders")
    ///     .select(&["status", "COUNT(*) as count"])
    ///     .groupBy("status")
    ///     .get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn groupBy(mut self, column: impl Into<String>) -> Self {
        self.group_by.push(column.into());
        self
    }

    /// Laravel-style having
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let stats = DB::table("orders")
    ///     .select(&["user_id", "SUM(total) as total"])
    ///     .groupBy("user_id")
    ///     .having("total", ">", 1000)
    ///     .get().await?;
    /// ```
    pub fn having<V: Into<Value>>(mut self, column: impl Into<String>, operator: impl Into<String>, value: V) -> Self {
        self.having.push((column.into(), operator.into(), value.into()));
        self
    }

    // =========================================================================
    // Date Where methods (Laravel-style)
    // =========================================================================

    /// Laravel-style whereDate - compare date part only
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereDate("created_at", "2024-01-15").get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereDate(mut self, column: impl Into<String>, date: impl Into<String>) -> Self {
        self.wheres.push((format!("DATE({})", column.into()), "=".to_string(), Value::String(date.into())));
        self
    }

    /// Laravel-style whereYear - compare year only
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereYear("created_at", 2024).get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereYear<V: Into<Value>>(mut self, column: impl Into<String>, year: V) -> Self {
        self.wheres.push((format!("YEAR({})", column.into()), "=".to_string(), year.into()));
        self
    }

    /// Laravel-style whereMonth - compare month only
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereMonth("created_at", 12).get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereMonth<V: Into<Value>>(mut self, column: impl Into<String>, month: V) -> Self {
        self.wheres.push((format!("MONTH({})", column.into()), "=".to_string(), month.into()));
        self
    }

    /// Laravel-style whereDay - compare day only
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereDay("created_at", 25).get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereDay<V: Into<Value>>(mut self, column: impl Into<String>, day: V) -> Self {
        self.wheres.push((format!("DAY({})", column.into()), "=".to_string(), day.into()));
        self
    }

    /// Laravel-style whereTime - compare time only
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereTime("created_at", ">=", "09:00:00").get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereTime(mut self, column: impl Into<String>, operator: impl Into<String>, time: impl Into<String>) -> Self {
        self.wheres.push((format!("TIME({})", column.into()), operator.into(), Value::String(time.into())));
        self
    }

    /// Laravel-style whereColumn - compare two columns
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereColumn("updated_at", ">", "created_at").get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereColumn(mut self, first: impl Into<String>, operator: impl Into<String>, second: impl Into<String>) -> Self {
        // Store as special column comparison marker
        self.wheres.push((first.into(), format!("COLUMN:{}", operator.into()), Value::String(second.into())));
        self
    }

    /// Execute the query and get all results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = DB::table("users")
    ///     .where_clause("active", "=", true.into())
    ///     .get().await?;
    /// ```
    pub async fn get(self) -> Result<Vec<Value>, String> {
        // Mock implementation - in production this executes against DB
        Ok(vec![])
    }

    /// Get the first result
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = DB::table("users")
    ///     .where_clause("id", "=", 1.into())
    ///     .first().await?;
    /// ```
    pub async fn first(self) -> Result<Option<Value>, String> {
        let results = self.limit(1).get().await?;
        Ok(results.into_iter().next())
    }

    /// Laravel-style firstOrFail - get first or return error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = User::where("id", 1).firstOrFail().await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn firstOrFail(self) -> Result<Value, String> {
        let table = self.table.clone();
        self.first()
            .await?
            .ok_or_else(|| format!("No record found in {}", table))
    }

    /// Laravel-style pluck - get array of single column values
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let emails = User::where("active", true).pluck("email").await?;
    /// // Returns: ["john@example.com", "jane@example.com", ...]
    /// ```
    pub async fn pluck(self, column: impl Into<String>) -> Result<Vec<Value>, String> {
        let col = column.into();
        let results = self.get().await?;
        Ok(results
            .into_iter()
            .filter_map(|row| row.get(&col).cloned())
            .collect())
    }

    /// Laravel-style value - get a single column value from first row
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let email = User::where("id", 1).value("email").await?;
    /// // Returns: Some("john@example.com")
    /// ```
    pub async fn value(self, column: impl Into<String>) -> Result<Option<Value>, String> {
        let col = column.into();
        let result = self.first().await?;
        Ok(result.and_then(|row| row.get(&col).cloned()))
    }

    /// Find a record by ID - Laravel-style!
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = DB::table("users").find(1).await?;
    /// let post = DB::table("posts").find(42).await?;
    /// ```
    pub async fn find<V: Into<Value>>(self, id: V) -> Result<Option<Value>, String> {
        self.r#where("id", id).first().await
    }

    /// Laravel-style findOrFail - find by ID or error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = DB::table("users").findOrFail(1).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn findOrFail<V: Into<Value>>(self, id: V) -> Result<Value, String> {
        let table = self.table.clone();
        self.r#where("id", id)
            .first()
            .await?
            .ok_or_else(|| format!("Record not found in {}", table))
    }

    /// Laravel-style inRandomOrder - randomize results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let random_user = User::query().inRandomOrder().first().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn inRandomOrder(self) -> Self {
        self.order_by("RANDOM()", "")
    }

    /// Laravel-style when - conditionally apply a query modification
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::query()
    ///     .when(is_admin, |q| q.where("role", "admin"))
    ///     .get().await?;
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

    /// Laravel-style unless - inverse of when
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::query()
    ///     .unless(show_all, |q| q.where("active", true))
    ///     .get().await?;
    /// ```
    pub fn unless<F>(self, condition: bool, callback: F) -> Self
    where
        F: FnOnce(Self) -> Self,
    {
        if !condition {
            callback(self)
        } else {
            self
        }
    }

    /// Laravel-style tap - execute a callback without modifying the query
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::query()
    ///     .where("active", true)
    ///     .tap(|q| println!("Query: {:?}", q))
    ///     .get().await?;
    /// ```
    pub fn tap<F>(self, callback: F) -> Self
    where
        F: FnOnce(&Self),
    {
        callback(&self);
        self
    }

    /// Laravel-style distinct - select distinct rows
    ///
    /// Note: This is a marker method, actual implementation depends on SQL builder
    pub fn distinct(self) -> Self {
        // In a real implementation, this would set a distinct flag
        self
    }

    // =========================================================================
    // Additional Laravel-style methods
    // =========================================================================

    /// Laravel-style whereNotBetween - value NOT between min and max
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereNotBetween("age", 18, 65).get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereNotBetween<V: Into<Value> + Clone>(mut self, column: impl Into<String>, min: V, max: V) -> Self {
        let col = column.into();
        // NOT BETWEEN is: value < min OR value > max
        self.wheres.push((col.clone(), "<".to_string(), min.into()));
        self.or_wheres.push((col, ">".to_string(), max.into()));
        self
    }

    /// snake_case alias for whereNotBetween
    pub fn where_not_between<V: Into<Value> + Clone>(self, column: impl Into<String>, min: V, max: V) -> Self {
        self.whereNotBetween(column, min, max)
    }

    /// Laravel-style whereNotLike - NOT LIKE pattern
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereNotLike("email", "%@spam.com").get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereNotLike(mut self, column: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "NOT LIKE".to_string(), Value::String(pattern.into())));
        self
    }

    /// snake_case alias for whereNotLike
    pub fn where_not_like(self, column: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.whereNotLike(column, pattern)
    }

    /// Laravel-style whereRaw - add raw SQL where clause
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let users = User::whereRaw("age > ? AND status = ?", vec![18, "active"]).get().await?;
    /// ```
    #[allow(non_snake_case)]
    pub fn whereRaw(mut self, sql: impl Into<String>, _bindings: Vec<Value>) -> Self {
        self.wheres.push((sql.into(), "RAW".to_string(), Value::Null));
        self
    }

    /// snake_case alias for whereRaw
    pub fn where_raw(self, sql: impl Into<String>, bindings: Vec<Value>) -> Self {
        self.whereRaw(sql, bindings)
    }

    /// Laravel-style orWhereRaw - add raw SQL OR where clause
    #[allow(non_snake_case)]
    pub fn orWhereRaw(mut self, sql: impl Into<String>, _bindings: Vec<Value>) -> Self {
        self.or_wheres.push((sql.into(), "RAW".to_string(), Value::Null));
        self
    }

    /// Laravel-style increment - increment a column value
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::where("id", 1).increment("login_count", 1).await?;
    /// User::where("id", 1).increment("views", 5).await?;
    /// ```
    pub async fn increment(self, column: impl Into<String>, amount: i64) -> Result<u64, String> {
        let col = column.into();
        self.update(serde_json::json!({ col: { "$inc": amount } })).await
    }

    /// Laravel-style decrement - decrement a column value
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::where("id", 1).decrement("credits", 10).await?;
    /// ```
    pub async fn decrement(self, column: impl Into<String>, amount: i64) -> Result<u64, String> {
        self.increment(column, -amount).await
    }

    /// Laravel-style firstOr - get first result or execute callback
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = User::where("email", email)
    ///     .firstOr(|| User::default())
    ///     .await;
    /// ```
    #[allow(non_snake_case)]
    pub async fn firstOr<F, T>(self, default: F) -> T
    where
        F: FnOnce() -> T,
        T: From<Value> + Default,
    {
        match self.first().await {
            Ok(Some(v)) => T::from(v),
            _ => default(),
        }
    }

    /// Laravel-style sole - get the only matching record, error if 0 or >1
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = User::where("email", "unique@example.com").sole().await?;
    /// ```
    pub async fn sole(self) -> Result<Value, String> {
        let table = self.table.clone();
        let results = self.limit(2).get().await?;
        match results.len() {
            0 => Err(format!("No records found in {}", table)),
            1 => Ok(results.into_iter().next().unwrap()),
            _ => Err(format!("Multiple records found in {} when one expected", table)),
        }
    }

    /// Laravel-style chunk - process records in chunks
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::query().chunk(100, |users| {
    ///     for user in users {
    ///         // Process each user
    ///     }
    ///     true // Continue processing
    /// }).await?;
    /// ```
    pub async fn chunk<F>(self, size: usize, mut callback: F) -> Result<(), String>
    where
        F: FnMut(Vec<Value>) -> bool,
    {
        let mut page = 1;
        loop {
            let results = self.clone()
                .limit(size)
                .offset((page - 1) * size)
                .get()
                .await?;

            if results.is_empty() {
                break;
            }

            let should_continue = callback(results);
            if !should_continue {
                break;
            }

            page += 1;
        }
        Ok(())
    }

    /// Laravel-style each - iterate over all records one by one
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::query().each(|user| {
    ///     println!("User: {:?}", user);
    ///     true // Continue
    /// }).await?;
    /// ```
    pub async fn each<F>(self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(Value) -> bool,
    {
        self.chunk(100, |records| {
            for record in records {
                if !callback(record) {
                    return false;
                }
            }
            true
        }).await
    }

    /// Laravel-style lazy - returns an iterator for memory-efficient processing
    /// Note: In async Rust, this returns a stream-like paginated iterator
    pub async fn lazy(self, chunk_size: usize) -> Result<LazyCollection, String> {
        Ok(LazyCollection {
            builder: self,
            chunk_size,
            current_page: 0,
            current_items: vec![],
        })
    }

    /// Laravel-style dd - dump and die (for debugging)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::where("active", true).dd();
    /// ```
    pub fn dd(self) -> ! {
        eprintln!("Query Debug:");
        eprintln!("  Table: {}", self.table);
        eprintln!("  Where: {:?}", self.wheres);
        eprintln!("  Or Where: {:?}", self.or_wheres);
        eprintln!("  Order By: {:?}", self.order_by);
        eprintln!("  Limit: {:?}", self.limit_value);
        eprintln!("  Offset: {:?}", self.offset_value);
        eprintln!("  Group By: {:?}", self.group_by);
        eprintln!("  Having: {:?}", self.having);
        std::process::exit(1);
    }

    /// Laravel-style dump - dump query info without stopping
    pub fn dump(self) -> Self {
        eprintln!("Query Debug:");
        eprintln!("  Table: {}", self.table);
        eprintln!("  Where: {:?}", self.wheres);
        eprintln!("  Or Where: {:?}", self.or_wheres);
        eprintln!("  Order By: {:?}", self.order_by);
        eprintln!("  Limit: {:?}", self.limit_value);
        self
    }

    /// Laravel-style toSql - get the SQL query string (for debugging)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let sql = User::where("active", true).toSql();
    /// println!("SQL: {}", sql);
    /// ```
    #[allow(non_snake_case)]
    pub fn toSql(&self) -> String {
        let mut sql = format!("SELECT {} FROM {}",
            if self.select_columns.is_empty() {
                "*".to_string()
            } else {
                self.select_columns.join(", ")
            },
            self.table
        );

        if !self.wheres.is_empty() {
            let conditions: Vec<String> = self.wheres.iter()
                .map(|(col, op, val)| format!("{} {} {:?}", col, op, val))
                .collect();
            sql.push_str(&format!(" WHERE {}", conditions.join(" AND ")));
        }

        if !self.or_wheres.is_empty() {
            let or_conditions: Vec<String> = self.or_wheres.iter()
                .map(|(col, op, val)| format!("{} {} {:?}", col, op, val))
                .collect();
            sql.push_str(&format!(" OR {}", or_conditions.join(" OR ")));
        }

        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }

        if !self.order_by.is_empty() {
            let orders: Vec<String> = self.order_by.iter()
                .map(|(col, dir)| format!("{} {}", col, dir))
                .collect();
            sql.push_str(&format!(" ORDER BY {}", orders.join(", ")));
        }

        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        sql
    }

    /// snake_case alias for toSql
    pub fn to_sql(&self) -> String {
        self.toSql()
    }

    // =========================================================================
    // Eloquent-style convenience methods
    // =========================================================================

    /// Laravel-style firstOrCreate - get first matching or create new
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = User::firstOrCreate(
    ///     json!({"email": "john@example.com"}),     // Search attributes
    ///     json!({"name": "John", "role": "user"})   // Additional attributes for create
    /// ).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn firstOrCreate(self, search: Value, create: Value) -> Result<Value, String> {
        // Try to find first
        if let Some(found) = self.clone().first().await? {
            return Ok(found);
        }

        // Create new with merged attributes
        let mut merged = search;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, create) {
            m1.extend(m2);
        }
        self.create(merged).await
    }

    /// snake_case alias for firstOrCreate
    pub async fn first_or_create(self, search: Value, create: Value) -> Result<Value, String> {
        self.firstOrCreate(search, create).await
    }

    /// Laravel-style firstOrNew - get first matching or return new instance (not saved)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = User::firstOrNew(
    ///     json!({"email": "john@example.com"}),
    ///     json!({"name": "John"})
    /// ).await;
    /// ```
    #[allow(non_snake_case)]
    pub async fn firstOrNew(self, search: Value, create: Value) -> Value {
        // Try to find first
        if let Ok(Some(found)) = self.clone().first().await {
            return found;
        }

        // Return merged attributes (not saved)
        let mut merged = search;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, create) {
            m1.extend(m2);
        }
        merged
    }

    /// snake_case alias for firstOrNew
    pub async fn first_or_new(self, search: Value, create: Value) -> Value {
        self.firstOrNew(search, create).await
    }

    /// Laravel-style updateOrCreate - update existing or create new
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let user = User::updateOrCreate(
    ///     json!({"email": "john@example.com"}),  // Search attributes
    ///     json!({"name": "John Updated"})        // Values to update/create
    /// ).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn updateOrCreate(self, search: Value, update: Value) -> Result<Value, String> {
        // Try to find and update
        if let Some(found) = self.clone().first().await? {
            let id = found.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            self.clone()
                .r#where("id", id)
                .update(update.clone())
                .await?;

            // Return updated record
            let mut result = found;
            if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut result, update) {
                m1.extend(m2);
            }
            return Ok(result);
        }

        // Create new
        let mut merged = search;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, update) {
            m1.extend(m2);
        }
        self.create(merged).await
    }

    /// snake_case alias for updateOrCreate
    pub async fn update_or_create(self, search: Value, update: Value) -> Result<Value, String> {
        self.updateOrCreate(search, update).await
    }

    /// Laravel-style updateOrInsert - update or insert (no return value)
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::updateOrInsert(
    ///     json!({"email": "john@example.com"}),
    ///     json!({"login_count": 1})
    /// ).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn updateOrInsert(self, search: Value, update: Value) -> Result<bool, String> {
        // Try to update
        let affected = self.clone().update(update.clone()).await?;
        if affected > 0 {
            return Ok(true);
        }

        // Insert new
        let mut merged = search;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, update) {
            m1.extend(m2);
        }
        self.insert(merged).await?;
        Ok(true)
    }

    /// Laravel-style upsert - insert or update multiple records
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::upsert(
    ///     vec![
    ///         json!({"email": "john@ex.com", "name": "John"}),
    ///         json!({"email": "jane@ex.com", "name": "Jane"}),
    ///     ],
    ///     &["email"],  // Unique columns
    ///     &["name"]    // Columns to update on conflict
    /// ).await?;
    /// ```
    pub async fn upsert(
        self,
        _records: Vec<Value>,
        _unique_by: &[&str],
        _update: &[&str]
    ) -> Result<u64, String> {
        // Mock implementation - real impl would use INSERT ... ON CONFLICT
        Ok(0)
    }

    /// Laravel-style touch - update timestamps
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::where("id", 1).touch().await?;
    /// ```
    pub async fn touch(self) -> Result<u64, String> {
        self.update(serde_json::json!({
            "updated_at": chrono::Utc::now().to_rfc3339()
        })).await
    }

    /// Laravel-style destroy - delete by IDs
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// User::destroy(vec![1, 2, 3]).await?;
    /// ```
    pub async fn destroy<V: Into<Value>>(self, ids: Vec<V>) -> Result<u64, String> {
        let ids: Vec<Value> = ids.into_iter().map(|id| id.into()).collect();
        self.where_in("id", ids).delete().await
    }

    /// Laravel-style truncate - delete all records
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Be careful! This deletes everything!
    /// User::truncate().await?;
    /// ```
    pub async fn truncate(self) -> Result<u64, String> {
        self.delete().await
    }

    /// Insert a new record and return the ID
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let id = DB::table("users").insert(json!({
    ///     "name": "John",
    ///     "email": "john@example.com"
    /// })).await?;
    /// ```
    pub async fn insert(self, _data: Value) -> Result<u64, String> {
        // Mock implementation - returns fake ID
        Ok(1)
    }

    /// Create a record and return it - Laravel-style!
    ///
    /// This is the preferred method for creating records.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // Just like Laravel's User::create()!
    /// let user = DB::table("users").create(json!({
    ///     "name": "John",
    ///     "email": "john@example.com"
    /// })).await?;
    ///
    /// println!("Created user: {}", user["name"]);
    /// ```
    pub async fn create(self, data: Value) -> Result<Value, String> {
        let _table = self.table.clone();
        let id = self.insert(data.clone()).await?;

        // Return the created record with ID
        let mut result = data;
        if let Value::Object(ref mut map) = result {
            map.insert("id".to_string(), Value::Number(id.into()));
        }
        Ok(result)
    }

    /// Insert multiple records
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// DB::table("users").insert_many(vec![
    ///     json!({"name": "John"}),
    ///     json!({"name": "Jane"}),
    /// ]).await?;
    /// ```
    pub async fn insert_many(self, data: Vec<Value>) -> Result<u64, String> {
        Ok(data.len() as u64)
    }

    /// Update records matching the where clauses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let affected = DB::table("users")
    ///     .where_clause("id", "=", 1.into())
    ///     .update(json!({"active": true})).await?;
    /// ```
    pub async fn update(self, _data: Value) -> Result<u64, String> {
        // Mock implementation
        Ok(1)
    }

    /// Delete records matching the where clauses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let deleted = DB::table("users")
    ///     .where_clause("id", "=", 1.into())
    ///     .delete().await?;
    /// ```
    pub async fn delete(self) -> Result<u64, String> {
        // Mock implementation
        Ok(1)
    }

    /// Count the results
    pub async fn count(self) -> Result<usize, String> {
        Ok(0)
    }

    /// Check if any records exist
    pub async fn exists(self) -> Result<bool, String> {
        let count = self.count().await?;
        Ok(count > 0)
    }

    /// Paginate results
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// let page = DB::table("users")
    ///     .where_clause("active", "=", true.into())
    ///     .paginate(15, 1).await?;
    /// ```
    pub async fn paginate(self, per_page: usize, page: usize) -> Result<PaginatedResult, String> {
        let offset = (page.saturating_sub(1)) * per_page;
        let data = self.clone().limit(per_page).offset(offset).get().await?;
        let total = self.count().await?;

        Ok(PaginatedResult {
            data,
            total,
            per_page,
            current_page: page,
            last_page: (total + per_page - 1) / per_page,
        })
    }

    /// Get the table name
    pub fn table_name(&self) -> &str {
        &self.table
    }

    /// Get the where clauses
    pub fn where_clauses(&self) -> &[(String, String, Value)] {
        &self.wheres
    }

    /// Get the limit value
    pub fn limit_val(&self) -> Option<usize> {
        self.limit_value
    }
}

/// Paginated result set
#[derive(Debug, Clone)]
pub struct PaginatedResult {
    pub data: Vec<Value>,
    pub total: usize,
    pub per_page: usize,
    pub current_page: usize,
    pub last_page: usize,
}

/// Laravel-style lazy collection for memory-efficient iteration
#[derive(Debug, Clone)]
pub struct LazyCollection {
    builder: QueryBuilder,
    chunk_size: usize,
    current_page: usize,
    current_items: Vec<Value>,
}

impl LazyCollection {
    /// Get the next item from the lazy collection
    pub async fn next(&mut self) -> Option<Value> {
        if self.current_items.is_empty() {
            // Fetch next chunk
            self.current_page += 1;
            let results = self.builder.clone()
                .limit(self.chunk_size)
                .offset((self.current_page - 1) * self.chunk_size)
                .get()
                .await
                .ok()?;

            if results.is_empty() {
                return None;
            }

            self.current_items = results;
            self.current_items.reverse(); // For efficient pop()
        }

        self.current_items.pop()
    }

    /// Collect all items into a vector
    pub async fn collect(mut self) -> Vec<Value> {
        let mut all = Vec::new();
        while let Some(item) = self.next().await {
            all.push(item);
        }
        all
    }

    /// Process each item with a callback
    pub async fn each<F>(mut self, mut callback: F) -> Result<(), String>
    where
        F: FnMut(Value) -> bool,
    {
        while let Some(item) = self.next().await {
            if !callback(item) {
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_new() {
        let builder = QueryBuilder::new("users");
        assert_eq!(builder.table_name(), "users");
        assert_eq!(builder.where_clauses().len(), 0);
    }

    #[test]
    fn test_query_builder_where() {
        let builder = QueryBuilder::new("users")
            .where_clause("active", "=", serde_json::json!(true));

        assert_eq!(builder.where_clauses().len(), 1);
        assert_eq!(builder.where_clauses()[0].0, "active");
        assert_eq!(builder.where_clauses()[0].1, "=");
    }

    #[test]
    fn test_query_builder_limit() {
        let builder = QueryBuilder::new("users")
            .limit(10);

        assert_eq!(builder.limit_val(), Some(10));
    }

    #[test]
    fn test_query_builder_chaining() {
        let builder = QueryBuilder::new("users")
            .where_clause("active", "=", serde_json::json!(true))
            .where_clause("verified", "=", serde_json::json!(true))
            .limit(10)
            .offset(5)
            .order_by("created_at", "desc");

        assert_eq!(builder.where_clauses().len(), 2);
        assert_eq!(builder.limit_val(), Some(10));
    }

    #[tokio::test]
    async fn test_query_builder_get() {
        let builder = QueryBuilder::new("users");
        let result = builder.get().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_query_builder_count() {
        let builder = QueryBuilder::new("users");
        let count = builder.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_query_builder_exists() {
        let builder = QueryBuilder::new("users");
        let exists = builder.exists().await.unwrap();
        assert!(!exists);
    }
}
