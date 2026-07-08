//! Query builder for constructing database queries
//!
//! Provides a Laravel-style fluent query builder for database operations.

use crate::facade::db_manager::GLOBAL_DB;
use serde::Serialize;
use serde_json::Value;

/// Escape character used for `LIKE ... ESCAPE` clauses built by
/// [`escape_like`] and [`QueryBuilder::where_like_escaped`].
const LIKE_ESCAPE_CHAR: char = '\\';

/// Escape LIKE wildcard metacharacters in `input` so it can be matched
/// literally inside a `LIKE ... ESCAPE '\'` pattern.
///
/// The SQL `LIKE` operator treats `%` (any sequence) and `_` (any single
/// character) as wildcards. When a user-supplied search term is placed inside a
/// `LIKE` pattern, those characters — plus the escape character `\` itself —
/// must be escaped so they are matched literally. This helper prefixes each of
/// `\`, `%` and `_` with a backslash; pair it with an `ESCAPE '\'` clause (as
/// [`QueryBuilder::where_like_escaped`] does).
///
/// # Examples
///
/// ```rust
/// use rf_orm::escape_like;
///
/// assert_eq!(escape_like("50%"), "50\\%");
/// assert_eq!(escape_like("a_b"), "a\\_b");
/// assert_eq!(escape_like("plain"), "plain");
/// ```
pub fn escape_like(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch == LIKE_ESCAPE_CHAR || ch == '%' || ch == '_' {
            out.push(LIKE_ESCAPE_CHAR);
        }
        out.push(ch);
    }
    out
}

/// Render a single `column op value` condition, appending the value (if any) to
/// `bindings` as a `?` placeholder. `NULL` values become `IS`/`IS NOT NULL`.
///
/// The pseudo-operators `LIKE ESCAPE` / `NOT LIKE ESCAPE` render a
/// `column LIKE ? ESCAPE '\'` clause so that wildcards escaped by
/// [`escape_like`] are matched literally.
fn push_condition(out: &mut String, column: &str, op: &str, value: &Value, bindings: &mut Vec<Value>) {
    if value.is_null() {
        let op = if op == "IS NOT" || op == "!=" || op == "<>" {
            "IS NOT"
        } else {
            "IS"
        };
        out.push_str(&format!("{} {} NULL", column, op));
    } else if op == "LIKE ESCAPE" || op == "NOT LIKE ESCAPE" {
        bindings.push(value.clone());
        let like_op = if op == "NOT LIKE ESCAPE" { "NOT LIKE" } else { "LIKE" };
        out.push_str(&format!("{} {} ? ESCAPE '{}'", column, like_op, LIKE_ESCAPE_CHAR));
    } else {
        bindings.push(value.clone());
        out.push_str(&format!("{} {} ?", column, op));
    }
}

/// Query builder for fluent database queries
///
/// # Examples
///
/// ```rust,no_run
/// use rf_orm::DB;
/// use serde_json::json;
///
/// async fn example() {
///     // Select with conditions
///     let users = DB::table("users")
///         .where_clause("active", "=", true.into())
///         .order_by("name", "asc")
///         .limit(10)
///         .get().await.unwrap();
///
///     // Insert
///     let id = DB::table("users").insert(json!({
///         "name": "John",
///         "email": "john@example.com"
///     })).await.unwrap();
///
///     // Update
///     DB::table("users")
///         .where_clause("id", "=", 1.into())
///         .update(json!({"active": true})).await.unwrap();
///
///     // Delete
///     DB::table("users")
///         .where_clause("id", "=", 1.into())
///         .delete().await.unwrap();
/// }
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
    distinct: bool,
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
            distinct: false,
        }
    }

    /// Select specific columns
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let users = DB::table("users")
    ///         .select(&["id", "name", "email"])
    ///         .get().await.unwrap();
    /// }
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     // Simple equality (like Laravel!)
    ///     let users = DB::table("users")
    ///         .r#where("active", true)
    ///         .r#where("role", "admin")
    ///         .get().await.unwrap();
    /// }
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     // Clean syntax without r# prefix!
    ///     let users = DB::table("users")
    ///         .filter("active", true)
    ///         .filter("role", "admin")
    ///         .get().await.unwrap();
    /// }
    /// ```
    pub fn filter<V: Into<Value>>(self, column: impl Into<String>, value: V) -> Self {
        self.r#where(column, value)
    }

    /// Where with custom operator
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let users = DB::table("users")
    ///         .where_op("age", ">=", 18)
    ///         .where_op("score", "<", 100)
    ///         .get().await.unwrap();
    /// }
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let users = DB::table("users")
    ///         .where_in("id", vec![1, 2, 3])
    ///         .get().await.unwrap();
    /// }
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

    /// Where column is like a **raw** LIKE pattern.
    ///
    /// The pattern is passed through verbatim, so `%` and `_` act as wildcards.
    /// This is intentional for callers that build their own patterns. When the
    /// pattern comes from user input, prefer [`where_like_escaped`](Self::where_like_escaped),
    /// which treats the term as a literal substring (a user term of `%` or `_`,
    /// or an empty term, will otherwise match far more rows than intended).
    pub fn where_like(mut self, column: impl Into<String>, pattern: impl Into<String>) -> Self {
        self.wheres.push((column.into(), "LIKE".to_string(), Value::String(pattern.into())));
        self
    }

    /// Where column contains `term` as a literal substring (wildcard-safe).
    ///
    /// Unlike [`where_like`](Self::where_like), the user-supplied `term` is
    /// escaped with [`escape_like`] and matched via `LIKE '%term%' ESCAPE '\'`,
    /// so any `%` or `_` in `term` is treated literally rather than as a
    /// wildcard. An **empty** `term` matches no rows (instead of matching every
    /// row, which a naive `%%` pattern would do).
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example(user_input: &str) {
    ///     // Safe search: a `%` in `user_input` is matched literally.
    ///     let tasks = DB::table("tasks")
    ///         .where_like_escaped("title", user_input)
    ///         .get().await.unwrap();
    /// }
    /// ```
    pub fn where_like_escaped(mut self, column: impl Into<String>, term: impl AsRef<str>) -> Self {
        let term = term.as_ref();
        if term.is_empty() {
            // An empty term would build the `%%` pattern, which matches every
            // row. Emit an always-false predicate so an empty search matches
            // nothing instead of leaking the whole table.
            self.wheres.push(("1".to_string(), "=".to_string(), Value::from(0)));
            return self;
        }
        let pattern = format!("%{}%", escape_like(term));
        self.wheres.push((column.into(), "LIKE ESCAPE".to_string(), Value::String(pattern)));
        self
    }

    /// Laravel-style whereLikeEscaped (camelCase alias for
    /// [`where_like_escaped`](Self::where_like_escaped)).
    #[allow(non_snake_case)]
    pub fn whereLikeEscaped(self, column: impl Into<String>, term: impl AsRef<str>) -> Self {
        self.where_like_escaped(column, term)
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let users = DB::table("users").take(5).get().await.unwrap();
    /// }
    /// ```
    pub fn take(self, count: usize) -> Self {
        self.limit(count)
    }

    /// Laravel-style skip() - alias for offset()
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let users = DB::table("users").skip(10).take(5).get().await.unwrap();
    /// }
    /// ```
    pub fn skip(self, count: usize) -> Self {
        self.offset(count)
    }

    /// Laravel-style latest() - order by created_at DESC
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let users = User::query().latest().take(10).get().await?;
    /// ```
    pub fn latest(self) -> Self {
        self.order_by("created_at", "DESC")
    }

    /// Laravel-style oldest() - order by created_at ASC
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// let users = User::query().oldest().first().await?;
    /// ```
    pub fn oldest(self) -> Self {
        self.order_by("created_at", "ASC")
    }

    /// Laravel-style insertMany (camelCase alias)
    #[allow(non_snake_case)]
    pub async fn insertMany<D: Serialize>(self, data: Vec<D>) -> Result<u64, String> {
        self.insert_many(data).await
    }

    // =========================================================================
    // OR Where conditions
    // =========================================================================

    /// Laravel-style orWhere - adds an OR condition
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let stats = DB::table("orders")
    ///         .select(&["status", "COUNT(*) as count"])
    ///         .groupBy("status")
    ///         .get().await.unwrap();
    /// }
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let stats = DB::table("orders")
    ///         .select(&["user_id", "SUM(total) as total"])
    ///         .groupBy("user_id")
    ///         .having("total", ">", 1000)
    ///         .get().await.unwrap();
    /// }
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let users = DB::table("users")
    ///         .where_clause("active", "=", true.into())
    ///         .get().await.unwrap();
    /// }
    /// ```
    /// Build the `WHERE` clause body (AND-joined `wheres`, then OR-joined
    /// `or_wheres`), collecting bound values into `bindings`. Empty if no filters.
    fn build_where(&self, bindings: &mut Vec<Value>) -> String {
        let mut clause = String::new();
        for (col, op, val) in &self.wheres {
            if !clause.is_empty() {
                clause.push_str(" AND ");
            }
            push_condition(&mut clause, col, op, val, bindings);
        }
        for (col, op, val) in &self.or_wheres {
            if clause.is_empty() {
                push_condition(&mut clause, col, op, val, bindings);
            } else {
                clause.push_str(" OR ");
                push_condition(&mut clause, col, op, val, bindings);
            }
        }
        clause
    }

    /// Build the full `SELECT` statement and its bound values from this builder.
    fn build_select_sql(&self) -> (String, Vec<Value>) {
        let mut bindings = Vec::new();
        let columns = if self.select_columns.is_empty() {
            "*".to_string()
        } else {
            self.select_columns.join(", ")
        };
        let select_kw = if self.distinct { "SELECT DISTINCT" } else { "SELECT" };
        let mut sql = format!("{} {} FROM {}", select_kw, columns, self.table);

        let where_clause = self.build_where(&mut bindings);
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }
        if !self.group_by.is_empty() {
            sql.push_str(&format!(" GROUP BY {}", self.group_by.join(", ")));
        }
        if !self.having.is_empty() {
            let mut having = String::new();
            for (col, op, val) in &self.having {
                if !having.is_empty() {
                    having.push_str(" AND ");
                }
                push_condition(&mut having, col, op, val, &mut bindings);
            }
            sql.push_str(&format!(" HAVING {}", having));
        }
        if !self.order_by.is_empty() {
            let orders: Vec<String> = self
                .order_by
                .iter()
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
        (sql, bindings)
    }

    pub async fn get(self) -> Result<Vec<Value>, String> {
        let (sql, bindings) = self.build_select_sql();
        let manager = GLOBAL_DB.lock().unwrap();
        manager.select(&sql, &bindings)
    }

    /// Get the first result
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let user = DB::table("users")
    ///         .where_clause("id", "=", 1.into())
    ///         .first().await.unwrap();
    /// }
    /// ```
    pub async fn first(self) -> Result<Option<Value>, String> {
        let results = self.limit(1).get().await?;
        Ok(results.into_iter().next())
    }

    /// Laravel-style firstOrFail - get first or return error
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let user = DB::table("users").find(1).await.unwrap();
    ///     let post = DB::table("posts").find(42).await.unwrap();
    /// }
    /// ```
    pub async fn find<V: Into<Value>>(self, id: V) -> Result<Option<Value>, String> {
        self.r#where("id", id).first().await
    }

    /// Laravel-style findOrFail - find by ID or error
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let user = DB::table("users").findOrFail(1).await.unwrap();
    /// }
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// Emits `SELECT DISTINCT ...` in the generated SQL (see [`get`](Self::get)
    /// and [`to_sql`](Self::to_sql)).
    pub fn distinct(mut self) -> Self {
        self.distinct = true;
        self
    }

    // =========================================================================
    // Additional Laravel-style methods
    // =========================================================================

    /// Laravel-style whereNotBetween - value NOT between min and max
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
    /// User::where("id", 1).increment("login_count", 1).await?;
    /// User::where("id", 1).increment("views", 5).await?;
    /// ```
    pub async fn increment(self, column: impl Into<String>, amount: i64) -> Result<u64, String> {
        let col = column.into();
        let mut bindings: Vec<Value> = vec![Value::from(amount)];
        let mut sql = format!("UPDATE {} SET {} = {} + ?", self.table, col, col);
        let where_clause = self.build_where(&mut bindings);
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.update(&sql, &bindings)
    }

    /// Laravel-style decrement - decrement a column value
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// User::where("id", 1).decrement("credits", 10).await?;
    /// ```
    pub async fn decrement(self, column: impl Into<String>, amount: i64) -> Result<u64, String> {
        self.increment(column, -amount).await
    }

    /// Laravel-style firstOr - get first result or execute callback
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
    /// let sql = User::where("active", true).toSql();
    /// println!("SQL: {}", sql);
    /// ```
    #[allow(non_snake_case)]
    pub fn toSql(&self) -> String {
        let mut sql = format!("{} {} FROM {}",
            if self.distinct { "SELECT DISTINCT" } else { "SELECT" },
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
    /// ```rust,ignore
    /// // With json! macro:
    /// let user = User::firstOrCreate(
    ///     json!({"email": "john@example.com"}),     // Search attributes
    ///     json!({"name": "John", "role": "user"})   // Additional attributes for create
    /// ).await?;
    ///
    /// // With structs (recommended):
    /// let user = User::firstOrCreate(
    ///     SearchEmail { email: "john@example.com".into() },
    ///     NewUser { name: "John".into(), role: "user".into() }
    /// ).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn firstOrCreate<S: Serialize, C: Serialize>(self, search: S, create: C) -> Result<Value, String> {
        let search_value = serde_json::to_value(search).map_err(|e| e.to_string())?;
        let create_value = serde_json::to_value(create).map_err(|e| e.to_string())?;

        // Try to find first
        if let Some(found) = self.clone().first().await? {
            return Ok(found);
        }

        // Create new with merged attributes
        let mut merged = search_value;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, create_value) {
            m1.extend(m2);
        }
        self.create(merged).await
    }

    /// snake_case alias for firstOrCreate
    pub async fn first_or_create<S: Serialize, C: Serialize>(self, search: S, create: C) -> Result<Value, String> {
        self.firstOrCreate(search, create).await
    }

    /// Laravel-style firstOrNew - get first matching or return new instance (not saved)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // With json! macro:
    /// let user = User::firstOrNew(
    ///     json!({"email": "john@example.com"}),
    ///     json!({"name": "John"})
    /// ).await;
    ///
    /// // With structs (recommended):
    /// let user = User::firstOrNew(
    ///     SearchEmail { email: "john@example.com".into() },
    ///     NewUser { name: "John".into() }
    /// ).await;
    /// ```
    #[allow(non_snake_case)]
    pub async fn firstOrNew<S: Serialize, C: Serialize>(self, search: S, create: C) -> Value {
        let search_value = serde_json::to_value(search).unwrap_or(Value::Null);
        let create_value = serde_json::to_value(create).unwrap_or(Value::Null);

        // Try to find first
        if let Ok(Some(found)) = self.clone().first().await {
            return found;
        }

        // Return merged attributes (not saved)
        let mut merged = search_value;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, create_value) {
            m1.extend(m2);
        }
        merged
    }

    /// snake_case alias for firstOrNew
    pub async fn first_or_new<S: Serialize, C: Serialize>(self, search: S, create: C) -> Value {
        self.firstOrNew(search, create).await
    }

    /// Laravel-style updateOrCreate - update existing or create new
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // With json! macro:
    /// let user = User::updateOrCreate(
    ///     json!({"email": "john@example.com"}),  // Search attributes
    ///     json!({"name": "John Updated"})        // Values to update/create
    /// ).await?;
    ///
    /// // With structs (recommended):
    /// let user = User::updateOrCreate(
    ///     SearchEmail { email: "john@example.com".into() },
    ///     UserUpdate { name: "John Updated".into() }
    /// ).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn updateOrCreate<S: Serialize, U: Serialize>(self, search: S, update: U) -> Result<Value, String> {
        let search_value = serde_json::to_value(search).map_err(|e| e.to_string())?;
        let update_value = serde_json::to_value(update).map_err(|e| e.to_string())?;

        // Try to find and update
        if let Some(found) = self.clone().first().await? {
            let id = found.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            self.clone()
                .r#where("id", id)
                .update(update_value.clone())
                .await?;

            // Return updated record
            let mut result = found;
            if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut result, update_value) {
                m1.extend(m2);
            }
            return Ok(result);
        }

        // Create new
        let mut merged = search_value;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, update_value) {
            m1.extend(m2);
        }
        self.create(merged).await
    }

    /// snake_case alias for updateOrCreate
    pub async fn update_or_create<S: Serialize, U: Serialize>(self, search: S, update: U) -> Result<Value, String> {
        self.updateOrCreate(search, update).await
    }

    /// Laravel-style updateOrInsert - update or insert (no return value)
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // With json! macro:
    /// User::updateOrInsert(
    ///     json!({"email": "john@example.com"}),
    ///     json!({"login_count": 1})
    /// ).await?;
    ///
    /// // With structs (recommended):
    /// User::updateOrInsert(
    ///     SearchEmail { email: "john@example.com".into() },
    ///     LoginUpdate { login_count: 1 }
    /// ).await?;
    /// ```
    #[allow(non_snake_case)]
    pub async fn updateOrInsert<S: Serialize, U: Serialize>(
        mut self,
        search: S,
        update: U,
    ) -> Result<bool, String> {
        let search_value = serde_json::to_value(search).map_err(|e| e.to_string())?;
        let update_value = serde_json::to_value(update).map_err(|e| e.to_string())?;

        // The search attributes are the WHERE filter for the update.
        if let Value::Object(map) = &search_value {
            for (key, value) in map {
                self = self.where_clause(key.clone(), "=", value.clone());
            }
        }

        // Try to update the matching row(s) first.
        let affected = self.clone().update(update_value.clone()).await?;
        if affected > 0 {
            return Ok(true);
        }

        // No match: insert the merged search + update attributes.
        let mut merged = search_value;
        if let (Value::Object(ref mut m1), Value::Object(m2)) = (&mut merged, update_value) {
            m1.extend(m2);
        }
        self.insert(merged).await?;
        Ok(true)
    }

    /// Laravel-style upsert - insert or update multiple records
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// // With json! macro:
    /// User::upsert(
    ///     vec![
    ///         json!({"email": "john@ex.com", "name": "John"}),
    ///         json!({"email": "jane@ex.com", "name": "Jane"}),
    ///     ],
    ///     &["email"],  // Unique columns
    ///     &["name"]    // Columns to update on conflict
    /// ).await?;
    ///
    /// // With structs (recommended):
    /// User::upsert(
    ///     vec![
    ///         NewUser { email: "john@ex.com".into(), name: "John".into() },
    ///         NewUser { email: "jane@ex.com".into(), name: "Jane".into() },
    ///     ],
    ///     &["email"],
    ///     &["name"]
    /// ).await?;
    /// ```
    pub async fn upsert<D: Serialize>(
        self,
        records: Vec<D>,
        unique_by: &[&str],
        update: &[&str],
    ) -> Result<u64, String> {
        let conflict = unique_by.join(", ");
        let mut affected = 0u64;

        for record in records {
            let value = serde_json::to_value(record).map_err(|e| e.to_string())?;
            let obj = value
                .as_object()
                .ok_or_else(|| "upsert() records must be JSON objects".to_string())?;
            if obj.is_empty() {
                continue;
            }

            let columns: Vec<&str> = obj.keys().map(String::as_str).collect();
            let placeholders = vec!["?"; columns.len()].join(", ");
            let conflict_action = if update.is_empty() {
                "DO NOTHING".to_string()
            } else {
                let sets = update
                    .iter()
                    .map(|c| format!("{c} = excluded.{c}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("DO UPDATE SET {sets}")
            };
            let sql = format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT({}) {}",
                self.table,
                columns.join(", "),
                placeholders,
                conflict,
                conflict_action
            );
            let bindings: Vec<Value> = obj.values().cloned().collect();

            let mut manager = GLOBAL_DB.lock().unwrap();
            manager.update(&sql, &bindings)?;
            affected += 1;
        }

        Ok(affected)
    }

    /// Laravel-style touch - update timestamps
    ///
    /// # Examples
    ///
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// ```rust,ignore
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
    /// use rf_orm::DB;
    /// use serde_json::json;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct NewUser { name: String, email: String }
    ///
    /// async fn example() {
    ///     // With json! macro:
    ///     let id = DB::table("users").insert(json!({
    ///         "name": "John",
    ///         "email": "john@example.com"
    ///     })).await.unwrap();
    ///
    ///     // With struct (recommended):
    ///     let id = DB::table("users").insert(NewUser {
    ///         name: "John".into(),
    ///         email: "john@example.com".into(),
    ///     }).await.unwrap();
    /// }
    /// ```
    pub async fn insert<D: Serialize>(self, data: D) -> Result<u64, String> {
        let value = serde_json::to_value(data).map_err(|e| e.to_string())?;
        let obj = value
            .as_object()
            .ok_or_else(|| "insert() data must be a JSON object".to_string())?;
        if obj.is_empty() {
            return Err("insert() data must not be empty".to_string());
        }

        let columns: Vec<&str> = obj.keys().map(String::as_str).collect();
        let placeholders = vec!["?"; columns.len()].join(", ");
        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            self.table,
            columns.join(", "),
            placeholders
        );
        let bindings: Vec<Value> = obj.values().cloned().collect();

        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.insert(&sql, &bindings)
    }

    /// Create a record and return it - Laravel-style!
    ///
    /// This is the preferred method for creating records.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    /// use serde_json::json;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct NewUser { name: String, email: String }
    ///
    /// async fn example() {
    ///     // With json! macro:
    ///     let user = DB::table("users").create(json!({
    ///         "name": "John",
    ///         "email": "john@example.com"
    ///     })).await.unwrap();
    ///
    ///     // With struct (recommended):
    ///     let user = DB::table("users").create(NewUser {
    ///         name: "John".into(),
    ///         email: "john@example.com".into(),
    ///     }).await.unwrap();
    ///
    ///     println!("Created user: {}", user["name"]);
    /// }
    /// ```
    pub async fn create<D: Serialize>(self, data: D) -> Result<Value, String> {
        let value = serde_json::to_value(data).map_err(|e| e.to_string())?;
        let _table = self.table.clone();
        let id = self.insert(value.clone()).await?;

        // Return the created record with ID
        let mut result = value;
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
    /// use rf_orm::DB;
    /// use serde_json::json;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct NewUser { name: String }
    ///
    /// async fn example() {
    ///     // With json! macro:
    ///     DB::table("users").insert_many(vec![
    ///         json!({"name": "John"}),
    ///         json!({"name": "Jane"}),
    ///     ]).await.unwrap();
    ///
    ///     // With structs (recommended):
    ///     DB::table("users").insert_many(vec![
    ///         NewUser { name: "John".into() },
    ///         NewUser { name: "Jane".into() },
    ///     ]).await.unwrap();
    /// }
    /// ```
    pub async fn insert_many<D: Serialize>(self, data: Vec<D>) -> Result<u64, String> {
        let table = self.table.clone();
        let mut inserted = 0u64;
        for item in data {
            let value = serde_json::to_value(item).map_err(|e| e.to_string())?;
            QueryBuilder::new(table.clone()).insert(value).await?;
            inserted += 1;
        }
        Ok(inserted)
    }

    /// Update records matching the where clauses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    /// use serde_json::json;
    /// use serde::Serialize;
    ///
    /// #[derive(Serialize)]
    /// struct UserUpdate { active: bool }
    ///
    /// async fn example() {
    ///     // With json! macro:
    ///     let affected = DB::table("users")
    ///         .where_clause("id", "=", 1.into())
    ///         .update(json!({"active": true})).await.unwrap();
    ///
    ///     // With struct (recommended):
    ///     let affected = DB::table("users")
    ///         .where_clause("id", "=", 1.into())
    ///         .update(UserUpdate { active: true }).await.unwrap();
    /// }
    /// ```
    pub async fn update<D: Serialize>(self, data: D) -> Result<u64, String> {
        let value = serde_json::to_value(data).map_err(|e| e.to_string())?;
        let obj = value
            .as_object()
            .ok_or_else(|| "update() data must be a JSON object".to_string())?;
        if obj.is_empty() {
            return Ok(0);
        }

        let mut bindings: Vec<Value> = Vec::new();
        let set_clause = obj
            .iter()
            .map(|(col, val)| {
                bindings.push(val.clone());
                format!("{} = ?", col)
            })
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!("UPDATE {} SET {}", self.table, set_clause);
        let where_clause = self.build_where(&mut bindings);
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.update(&sql, &bindings)
    }

    /// Delete records matching the where clauses
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let deleted = DB::table("users")
    ///         .where_clause("id", "=", 1.into())
    ///         .delete().await.unwrap();
    /// }
    /// ```
    pub async fn delete(self) -> Result<u64, String> {
        let mut bindings: Vec<Value> = Vec::new();
        let where_clause = self.build_where(&mut bindings);
        let mut sql = format!("DELETE FROM {}", self.table);
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }

        let mut manager = GLOBAL_DB.lock().unwrap();
        manager.delete(&sql, &bindings)
    }

    /// Count the results
    pub async fn count(self) -> Result<usize, String> {
        let mut bindings = Vec::new();
        let where_clause = self.build_where(&mut bindings);
        let mut sql = format!("SELECT COUNT(*) AS count FROM {}", self.table);
        if !where_clause.is_empty() {
            sql.push_str(&format!(" WHERE {}", where_clause));
        }
        let manager = GLOBAL_DB.lock().unwrap();
        let rows = manager.select(&sql, &bindings)?;
        let count = rows
            .first()
            .and_then(|row| row.get("count"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        Ok(count as usize)
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
    /// use rf_orm::DB;
    ///
    /// async fn example() {
    ///     let page = DB::table("users")
    ///         .where_clause("active", "=", true.into())
    ///         .paginate(15, 1).await.unwrap();
    /// }
    /// ```
    pub async fn paginate(self, per_page: usize, page: usize) -> Result<PaginatedResult, String> {
        let per_page = per_page.max(1);
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
        // Dedicated empty table so the result is deterministic on the shared global DB.
        crate::DB::statement("CREATE TABLE IF NOT EXISTS qb_get (id INTEGER PRIMARY KEY)").unwrap();
        crate::DB::statement("DELETE FROM qb_get").unwrap();
        let result = QueryBuilder::new("qb_get").get().await;
        assert_eq!(result.unwrap().len(), 0);
    }

    #[tokio::test]
    async fn test_query_builder_get_returns_real_rows() {
        crate::DB::statement("CREATE TABLE IF NOT EXISTS qb_rows (id INTEGER PRIMARY KEY, active INTEGER)")
            .unwrap();
        crate::DB::statement("DELETE FROM qb_rows").unwrap();
        crate::DB::insert("INSERT INTO qb_rows (id, active) VALUES (1, 1), (2, 0)", &[]).unwrap();
        let rows = QueryBuilder::new("qb_rows")
            .where_clause("active", "=", serde_json::json!(1))
            .get()
            .await
            .unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["id"], serde_json::json!(1));
    }

    #[tokio::test]
    async fn test_query_builder_count() {
        crate::DB::statement("CREATE TABLE IF NOT EXISTS qb_count (id INTEGER PRIMARY KEY)").unwrap();
        crate::DB::statement("DELETE FROM qb_count").unwrap();
        assert_eq!(QueryBuilder::new("qb_count").count().await.unwrap(), 0);
        crate::DB::insert("INSERT INTO qb_count (id) VALUES (1), (2), (3)", &[]).unwrap();
        assert_eq!(QueryBuilder::new("qb_count").count().await.unwrap(), 3);
    }

    #[test]
    fn test_escape_like_escapes_metacharacters() {
        assert_eq!(escape_like("50%"), "50\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("back\\slash"), "back\\\\slash");
        assert_eq!(escape_like("plain text"), "plain text");
        assert_eq!(escape_like(""), "");
    }

    #[test]
    fn test_where_like_escaped_builds_escape_clause() {
        let (sql, bindings) = QueryBuilder::new("tasks")
            .where_like_escaped("title", "50%")
            .build_select_sql();
        assert!(
            sql.contains("title LIKE ? ESCAPE '\\'"),
            "expected ESCAPE clause, got: {sql}"
        );
        assert_eq!(bindings, vec![Value::String("%50\\%%".to_string())]);
    }

    #[test]
    fn test_where_like_escaped_empty_term_matches_nothing() {
        let (sql, bindings) = QueryBuilder::new("tasks")
            .where_like_escaped("title", "")
            .build_select_sql();
        assert!(sql.contains("WHERE 1 = ?"), "expected always-false clause, got: {sql}");
        assert_eq!(bindings, vec![Value::from(0)]);
    }

    #[tokio::test]
    async fn test_where_like_escaped_real_sqlite() {
        crate::DB::statement(
            "CREATE TABLE IF NOT EXISTS qb_like (id INTEGER PRIMARY KEY, title TEXT)",
        )
        .unwrap();
        crate::DB::statement("DELETE FROM qb_like").unwrap();
        for title in ["Write report", "50% off sale", "snake_case docs", "Read book"] {
            crate::DB::insert(
                "INSERT INTO qb_like (title) VALUES (?)",
                &[serde_json::json!(title)],
            )
            .unwrap();
        }

        // Plain term: only real substring match.
        let hits = QueryBuilder::new("qb_like")
            .where_like_escaped("title", "report")
            .get()
            .await
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0]["title"], serde_json::json!("Write report"));

        // Literal `%` matches only the row that actually contains `%`.
        let percent = QueryBuilder::new("qb_like")
            .where_like_escaped("title", "%")
            .get()
            .await
            .unwrap();
        assert_eq!(percent.len(), 1);
        assert_eq!(percent[0]["title"], serde_json::json!("50% off sale"));

        // Empty term matches nothing (not the whole table).
        let empty = QueryBuilder::new("qb_like")
            .where_like_escaped("title", "")
            .get()
            .await
            .unwrap();
        assert_eq!(empty.len(), 0);
    }

    #[tokio::test]
    async fn test_query_builder_exists() {
        crate::DB::statement("CREATE TABLE IF NOT EXISTS qb_exists (id INTEGER PRIMARY KEY)").unwrap();
        crate::DB::statement("DELETE FROM qb_exists").unwrap();
        assert!(!QueryBuilder::new("qb_exists").exists().await.unwrap());
        crate::DB::insert("INSERT INTO qb_exists (id) VALUES (1)", &[]).unwrap();
        assert!(QueryBuilder::new("qb_exists").exists().await.unwrap());
    }
}
