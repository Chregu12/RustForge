//! # Subquery Support
//!
//! Laravel-style subquery support for complex database queries.
//!
//! ## Overview
//!
//! Subqueries allow you to nest queries within queries, enabling complex
//! filtering and data retrieval patterns. This module provides a fluent
//! interface for building subqueries that can be used with WHERE IN,
//! WHERE EXISTS, and other SQL operations.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::query::subquery::*;
//!
//! // Find users who have published posts
//! let users = User::query(db.clone())
//!     .where_in_subquery(
//!         "id",
//!         Subquery::new::<post::Entity>(db.clone())
//!             .select("user_id")
//!             .where_eq("published", true)
//!     )
//!     .get()
//!     .await?;
//!
//! // Find posts that have comments
//! let posts = Post::query(db.clone())
//!     .where_exists(
//!         Subquery::new::<comment::Entity>(db)
//!             .where_raw("comments.post_id = posts.id")
//!     )
//!     .get()
//!     .await?;
//! ```

use sea_orm::{DatabaseConnection, EntityTrait};
use std::marker::PhantomData;

/// A subquery builder that can be used in WHERE clauses
///
/// Subqueries are built using a fluent interface similar to the main
/// QueryBuilder, but they can be embedded within other queries.
///
/// # Example
///
/// ```rust,no_run
/// // Get published post IDs
/// let subquery = Subquery::new::<post::Entity>(db.clone())
///     .select("id")
///     .where_eq("published", true)
///     .where_gt("views", 1000);
///
/// // Use in main query
/// User::query(db)
///     .where_in_subquery("favorite_post_id", subquery)
///     .get()
///     .await?;
/// ```
pub struct Subquery<E>
where
    E: EntityTrait,
{
    db: DatabaseConnection,
    select_columns: Vec<String>,
    where_clauses: Vec<WhereClause>,
    order_by: Option<(String, String)>,
    limit_value: Option<u64>,
    offset_value: Option<u64>,
    _phantom: PhantomData<E>,
}

/// Internal representation of WHERE clauses for subqueries
#[derive(Clone)]
enum WhereClause {
    Eq(String, sea_orm::Value),
    Ne(String, sea_orm::Value),
    Gt(String, sea_orm::Value),
    Gte(String, sea_orm::Value),
    Lt(String, sea_orm::Value),
    Lte(String, sea_orm::Value),
    In(String, Vec<sea_orm::Value>),
    NotIn(String, Vec<sea_orm::Value>),
    IsNull(String),
    IsNotNull(String),
    Like(String, String),
    Raw(String),
}

impl<E> Subquery<E>
where
    E: EntityTrait,
{
    /// Create a new subquery for the given entity
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// let subquery = Subquery::new::<post::Entity>(db.clone());
    /// ```
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            select_columns: vec!["id".to_string()],
            where_clauses: Vec::new(),
            order_by: None,
            limit_value: None,
            offset_value: None,
            _phantom: PhantomData,
        }
    }

    /// Select specific columns in the subquery
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// subquery.select("user_id")
    /// ```
    pub fn select(mut self, column: impl Into<String>) -> Self {
        self.select_columns = vec![column.into()];
        self
    }

    /// Select multiple columns in the subquery
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// subquery.select_multiple(vec!["user_id", "post_id"])
    /// ```
    pub fn select_multiple(mut self, columns: Vec<impl Into<String>>) -> Self {
        self.select_columns = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    /// Add a WHERE equals clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// subquery.where_eq("published", true)
    /// ```
    pub fn where_eq(mut self, column: &str, value: impl Into<sea_orm::Value>) -> Self {
        self.where_clauses
            .push(WhereClause::Eq(column.to_string(), value.into()));
        self
    }

    /// Add a WHERE not equals clause
    pub fn where_ne(mut self, column: &str, value: impl Into<sea_orm::Value>) -> Self {
        self.where_clauses
            .push(WhereClause::Ne(column.to_string(), value.into()));
        self
    }

    /// Add a WHERE greater than clause
    pub fn where_gt(mut self, column: &str, value: impl Into<sea_orm::Value>) -> Self {
        self.where_clauses
            .push(WhereClause::Gt(column.to_string(), value.into()));
        self
    }

    /// Add a WHERE greater than or equal clause
    pub fn where_gte(mut self, column: &str, value: impl Into<sea_orm::Value>) -> Self {
        self.where_clauses
            .push(WhereClause::Gte(column.to_string(), value.into()));
        self
    }

    /// Add a WHERE less than clause
    pub fn where_lt(mut self, column: &str, value: impl Into<sea_orm::Value>) -> Self {
        self.where_clauses
            .push(WhereClause::Lt(column.to_string(), value.into()));
        self
    }

    /// Add a WHERE less than or equal clause
    pub fn where_lte(mut self, column: &str, value: impl Into<sea_orm::Value>) -> Self {
        self.where_clauses
            .push(WhereClause::Lte(column.to_string(), value.into()));
        self
    }

    /// Add a WHERE IN clause
    pub fn where_in(
        mut self,
        column: &str,
        values: Vec<impl Into<sea_orm::Value>>,
    ) -> Self {
        let values = values.into_iter().map(|v| v.into()).collect();
        self.where_clauses
            .push(WhereClause::In(column.to_string(), values));
        self
    }

    /// Add a WHERE NOT IN clause
    pub fn where_not_in(
        mut self,
        column: &str,
        values: Vec<impl Into<sea_orm::Value>>,
    ) -> Self {
        let values = values.into_iter().map(|v| v.into()).collect();
        self.where_clauses
            .push(WhereClause::NotIn(column.to_string(), values));
        self
    }

    /// Add a WHERE IS NULL clause
    pub fn where_null(mut self, column: &str) -> Self {
        self.where_clauses
            .push(WhereClause::IsNull(column.to_string()));
        self
    }

    /// Add a WHERE IS NOT NULL clause
    pub fn where_not_null(mut self, column: &str) -> Self {
        self.where_clauses
            .push(WhereClause::IsNotNull(column.to_string()));
        self
    }

    /// Add a WHERE LIKE clause
    pub fn where_like(mut self, column: &str, pattern: &str) -> Self {
        self.where_clauses
            .push(WhereClause::Like(column.to_string(), pattern.to_string()));
        self
    }

    /// Add a raw WHERE clause
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// subquery.where_raw("DATE(created_at) = CURDATE()")
    /// ```
    pub fn where_raw(mut self, sql: &str) -> Self {
        self.where_clauses
            .push(WhereClause::Raw(sql.to_string()));
        self
    }

    /// Add an ORDER BY clause
    pub fn order_by(mut self, column: &str, direction: &str) -> Self {
        self.order_by = Some((column.to_string(), direction.to_string()));
        self
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

    /// Build the subquery as a SQL string
    ///
    /// This is used internally by the query builder to embed the subquery
    /// into WHERE IN and other clauses.
    ///
    /// # Returns
    ///
    /// A SQL string representation of the subquery wrapped in parentheses
    pub fn build_sql(&self) -> String {
        let entity = E::default();
        let table_name = entity.table_name().to_string();

        // Build SELECT clause
        let select_clause = if self.select_columns.is_empty() {
            "id".to_string()
        } else {
            self.select_columns.join(", ")
        };

        let mut sql = format!("SELECT {} FROM {}", select_clause, table_name);

        // Build WHERE clause
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            let where_parts: Vec<String> = self
                .where_clauses
                .iter()
                .map(|clause| match clause {
                    WhereClause::Eq(col, _) => format!("{} = ?", col),
                    WhereClause::Ne(col, _) => format!("{} != ?", col),
                    WhereClause::Gt(col, _) => format!("{} > ?", col),
                    WhereClause::Gte(col, _) => format!("{} >= ?", col),
                    WhereClause::Lt(col, _) => format!("{} < ?", col),
                    WhereClause::Lte(col, _) => format!("{} <= ?", col),
                    WhereClause::In(col, vals) => {
                        let placeholders = vec!["?"; vals.len()].join(", ");
                        format!("{} IN ({})", col, placeholders)
                    }
                    WhereClause::NotIn(col, vals) => {
                        let placeholders = vec!["?"; vals.len()].join(", ");
                        format!("{} NOT IN ({})", col, placeholders)
                    }
                    WhereClause::IsNull(col) => format!("{} IS NULL", col),
                    WhereClause::IsNotNull(col) => format!("{} IS NOT NULL", col),
                    WhereClause::Like(col, _) => format!("{} LIKE ?", col),
                    WhereClause::Raw(raw) => raw.clone(),
                })
                .collect();

            sql.push_str(&where_parts.join(" AND "));
        }

        // Build ORDER BY clause
        if let Some((column, direction)) = &self.order_by {
            sql.push_str(&format!(" ORDER BY {} {}", column, direction.to_uppercase()));
        }

        // Build LIMIT clause
        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Build OFFSET clause
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        format!("({})", sql)
    }

    /// Get the values for parameter binding
    ///
    /// Returns all values in the order they appear in the SQL
    pub fn get_values(&self) -> Vec<sea_orm::Value> {
        let mut values = Vec::new();

        for clause in &self.where_clauses {
            match clause {
                WhereClause::Eq(_, val)
                | WhereClause::Ne(_, val)
                | WhereClause::Gt(_, val)
                | WhereClause::Gte(_, val)
                | WhereClause::Lt(_, val)
                | WhereClause::Lte(_, val) => {
                    values.push(val.clone());
                }
                WhereClause::In(_, vals) | WhereClause::NotIn(_, vals) => {
                    values.extend(vals.clone());
                }
                WhereClause::Like(_, pattern) => {
                    values.push(sea_orm::Value::String(Some(Box::new(pattern.clone()))));
                }
                WhereClause::IsNull(_) | WhereClause::IsNotNull(_) | WhereClause::Raw(_) => {
                    // No values needed
                }
            }
        }

        values
    }

    /// Get a reference to the database connection
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }
}

/// Builder for creating subqueries with a more flexible API
///
/// This is an alternative to the `Subquery` struct that doesn't require
/// specifying the entity type upfront.
///
/// # Example
///
/// ```rust,no_run
/// let subquery = SubqueryBuilder::new("posts")
///     .select("user_id")
///     .where_clause("published = true")
///     .build();
/// ```
pub struct SubqueryBuilder {
    table: String,
    select_columns: Vec<String>,
    where_clauses: Vec<String>,
    order_by: Option<(String, String)>,
    limit_value: Option<u64>,
    offset_value: Option<u64>,
}

impl SubqueryBuilder {
    /// Create a new subquery builder for a table
    pub fn new(table: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            select_columns: vec!["id".to_string()],
            where_clauses: Vec::new(),
            order_by: None,
            limit_value: None,
            offset_value: None,
        }
    }

    /// Select a column
    pub fn select(mut self, column: impl Into<String>) -> Self {
        self.select_columns = vec![column.into()];
        self
    }

    /// Select multiple columns
    pub fn select_multiple(mut self, columns: Vec<impl Into<String>>) -> Self {
        self.select_columns = columns.into_iter().map(|c| c.into()).collect();
        self
    }

    /// Add a WHERE clause (raw SQL)
    pub fn where_clause(mut self, clause: impl Into<String>) -> Self {
        self.where_clauses.push(clause.into());
        self
    }

    /// Add an ORDER BY clause
    pub fn order_by(mut self, column: impl Into<String>, direction: impl Into<String>) -> Self {
        self.order_by = Some((column.into(), direction.into()));
        self
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

    /// Build the subquery as a SQL string
    pub fn build(&self) -> String {
        let select_clause = self.select_columns.join(", ");
        let mut sql = format!("SELECT {} FROM {}", select_clause, self.table);

        // Add WHERE clause
        if !self.where_clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&self.where_clauses.join(" AND "));
        }

        // Add ORDER BY
        if let Some((column, direction)) = &self.order_by {
            sql.push_str(&format!(" ORDER BY {} {}", column, direction.to_uppercase()));
        }

        // Add LIMIT
        if let Some(limit) = self.limit_value {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        // Add OFFSET
        if let Some(offset) = self.offset_value {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        format!("({})", sql)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subquery_builder_sql() {
        let sql = SubqueryBuilder::new("posts")
            .select("user_id")
            .where_clause("published = true")
            .where_clause("views > 100")
            .order_by("created_at", "desc")
            .limit(10)
            .build();

        assert!(sql.contains("SELECT user_id FROM posts"));
        assert!(sql.contains("WHERE published = true AND views > 100"));
        assert!(sql.contains("ORDER BY created_at DESC"));
        assert!(sql.contains("LIMIT 10"));
        assert!(sql.starts_with('('));
        assert!(sql.ends_with(')'));
    }

    #[test]
    fn test_subquery_builder_minimal() {
        let sql = SubqueryBuilder::new("users").build();
        assert_eq!(sql, "(SELECT id FROM users)");
    }

    #[test]
    fn test_subquery_builder_chaining() {
        let builder = SubqueryBuilder::new("posts")
            .select("id")
            .where_clause("status = 'active'")
            .limit(5);

        let sql = builder.build();
        assert!(sql.contains("posts"));
        assert!(sql.contains("status = 'active'"));
        assert!(sql.contains("LIMIT 5"));
    }
}
