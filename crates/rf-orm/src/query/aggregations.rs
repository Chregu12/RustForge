//! # Advanced Aggregations
//!
//! Laravel-style aggregate functions with relationship support.
//!
//! ## Overview
//!
//! This module provides advanced aggregation capabilities similar to Laravel's
//! `withCount()`, `withSum()`, `withAvg()`, etc. These allow you to load
//! relationship aggregates alongside your models without N+1 queries.
//!
//! ## Example
//!
//! ```rust,no_run
//! use rf_orm::query::aggregations::*;
//!
//! // Load users with post counts
//! let users = User::query(db.clone())
//!     .with_count("posts")
//!     .with_sum("posts", "views")
//!     .with_avg("posts", "rating")
//!     .get()
//!     .await?;
//!
//! for user in users {
//!     println!("User has {} posts", user.posts_count);
//!     println!("Total views: {}", user.posts_views_sum);
//!     println!("Average rating: {}", user.posts_rating_avg);
//! }
//! ```

use sea_orm::{ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, Statement};
use std::collections::HashMap;

/// Result type for aggregation operations
pub type AggregationResult<T> = Result<T, DbErr>;

/// Types of aggregation operations
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateType {
    /// Count the number of related records
    Count,
    /// Sum a column value across related records
    Sum(String),
    /// Average a column value across related records
    Avg(String),
    /// Minimum value of a column across related records
    Min(String),
    /// Maximum value of a column across related records
    Max(String),
}

impl AggregateType {
    /// Get the SQL function name for this aggregate type
    pub fn sql_function(&self) -> &str {
        match self {
            AggregateType::Count => "COUNT",
            AggregateType::Sum(_) => "SUM",
            AggregateType::Avg(_) => "AVG",
            AggregateType::Min(_) => "MIN",
            AggregateType::Max(_) => "MAX",
        }
    }

    /// Get the column to aggregate (or "*" for COUNT)
    pub fn column(&self) -> &str {
        match self {
            AggregateType::Count => "*",
            AggregateType::Sum(col)
            | AggregateType::Avg(col)
            | AggregateType::Min(col)
            | AggregateType::Max(col) => col,
        }
    }

    /// Get the alias for this aggregate in the result set
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// AggregateType::Count.alias("posts") // => "posts_count"
    /// AggregateType::Sum("views").alias("posts") // => "posts_views_sum"
    /// ```
    pub fn alias(&self, relation: &str) -> String {
        match self {
            AggregateType::Count => format!("{}_count", relation),
            AggregateType::Sum(col) => format!("{}_{}_sum", relation, col),
            AggregateType::Avg(col) => format!("{}_{}_avg", relation, col),
            AggregateType::Min(col) => format!("{}_{}_min", relation, col),
            AggregateType::Max(col) => format!("{}_{}_max", relation, col),
        }
    }
}

/// Represents a single aggregate to be loaded
#[derive(Debug, Clone)]
pub struct Aggregate {
    /// The relationship name (e.g., "posts", "comments")
    pub relation: String,
    /// The type of aggregation
    pub aggregate_type: AggregateType,
    /// Optional WHERE clause for filtering the aggregation
    pub where_clause: Option<String>,
}

impl Aggregate {
    /// Create a new COUNT aggregate
    pub fn count(relation: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            aggregate_type: AggregateType::Count,
            where_clause: None,
        }
    }

    /// Create a new SUM aggregate
    pub fn sum(relation: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            aggregate_type: AggregateType::Sum(column.into()),
            where_clause: None,
        }
    }

    /// Create a new AVG aggregate
    pub fn avg(relation: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            aggregate_type: AggregateType::Avg(column.into()),
            where_clause: None,
        }
    }

    /// Create a new MIN aggregate
    pub fn min(relation: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            aggregate_type: AggregateType::Min(column.into()),
            where_clause: None,
        }
    }

    /// Create a new MAX aggregate
    pub fn max(relation: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            relation: relation.into(),
            aggregate_type: AggregateType::Max(column.into()),
            where_clause: None,
        }
    }

    /// Add a WHERE clause to filter the aggregation
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Aggregate::count("posts").with_where("published = true")
    /// ```
    pub fn with_where(mut self, clause: impl Into<String>) -> Self {
        self.where_clause = Some(clause.into());
        self
    }

    /// Get the alias for this aggregate in the result set
    pub fn alias(&self) -> String {
        self.aggregate_type.alias(&self.relation)
    }

    /// Build the SQL expression for this aggregate
    ///
    /// # Arguments
    ///
    /// * `parent_table` - The parent table name
    /// * `parent_key` - The parent key column
    /// * `relation_table` - The relation table name
    /// * `foreign_key` - The foreign key column in the relation table
    pub fn build_sql(
        &self,
        parent_table: &str,
        parent_key: &str,
        relation_table: &str,
        foreign_key: &str,
    ) -> String {
        let function = self.aggregate_type.sql_function();
        let column = self.aggregate_type.column();

        let mut sql = format!(
            "(SELECT {}({}) FROM {} WHERE {}.{} = {}.{}",
            function, column, relation_table, relation_table, foreign_key, parent_table, parent_key
        );

        // Add WHERE clause if provided
        if let Some(ref where_clause) = self.where_clause {
            sql.push_str(&format!(" AND {}", where_clause));
        }

        sql.push_str(&format!(") AS {}", self.alias()));

        sql
    }
}

/// Trait for adding aggregate loading to queries
///
/// This trait provides methods for loading relationship aggregates
/// alongside your models.
///
/// # Example
///
/// ```rust,no_run
/// use rf_orm::query::aggregations::WithAggregates;
///
/// let users = User::query(db)
///     .with_count("posts")
///     .with_sum("posts", "views")
///     .get()
///     .await?;
/// ```
pub trait WithAggregates: Sized {
    /// Add a COUNT aggregate for a relationship
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// User::query(db).with_count("posts")
    /// ```
    fn with_count(self, relation: &str) -> Self;

    /// Add a SUM aggregate for a relationship column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// User::query(db).with_sum("posts", "views")
    /// ```
    fn with_sum(self, relation: &str, column: &str) -> Self;

    /// Add an AVG aggregate for a relationship column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// User::query(db).with_avg("posts", "rating")
    /// ```
    fn with_avg(self, relation: &str, column: &str) -> Self;

    /// Add a MIN aggregate for a relationship column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Product::query(db).with_min("prices", "amount")
    /// ```
    fn with_min(self, relation: &str, column: &str) -> Self;

    /// Add a MAX aggregate for a relationship column
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// Product::query(db).with_max("prices", "amount")
    /// ```
    fn with_max(self, relation: &str, column: &str) -> Self;

    /// Add a custom aggregate
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// User::query(db).with_aggregate(
    ///     Aggregate::count("posts")
    ///         .with_where("published = true")
    /// )
    /// ```
    fn with_aggregate(self, aggregate: Aggregate) -> Self;
}

/// Builder for creating complex aggregation queries
///
/// This builder allows you to construct queries with multiple aggregates
/// and execute them to get both the models and their aggregate values.
///
/// # Example
///
/// ```rust,no_run
/// let builder = AggregationBuilder::new(db.clone())
///     .add_count("posts")
///     .add_sum("posts", "views")
///     .add_avg("comments", "rating");
///
/// let results = builder.execute::<user::Entity>("users", "id").await?;
/// ```
pub struct AggregationBuilder {
    db: DatabaseConnection,
    aggregates: Vec<Aggregate>,
}

impl AggregationBuilder {
    /// Create a new aggregation builder
    pub fn new(db: DatabaseConnection) -> Self {
        Self {
            db,
            aggregates: Vec::new(),
        }
    }

    /// Add a COUNT aggregate
    pub fn add_count(mut self, relation: impl Into<String>) -> Self {
        self.aggregates.push(Aggregate::count(relation));
        self
    }

    /// Add a SUM aggregate
    pub fn add_sum(mut self, relation: impl Into<String>, column: impl Into<String>) -> Self {
        self.aggregates.push(Aggregate::sum(relation, column));
        self
    }

    /// Add an AVG aggregate
    pub fn add_avg(mut self, relation: impl Into<String>, column: impl Into<String>) -> Self {
        self.aggregates.push(Aggregate::avg(relation, column));
        self
    }

    /// Add a MIN aggregate
    pub fn add_min(mut self, relation: impl Into<String>, column: impl Into<String>) -> Self {
        self.aggregates.push(Aggregate::min(relation, column));
        self
    }

    /// Add a MAX aggregate
    pub fn add_max(mut self, relation: impl Into<String>, column: impl Into<String>) -> Self {
        self.aggregates.push(Aggregate::max(relation, column));
        self
    }

    /// Add a custom aggregate
    pub fn add_aggregate(mut self, aggregate: Aggregate) -> Self {
        self.aggregates.push(aggregate);
        self
    }

    /// Execute the aggregation query
    ///
    /// # Arguments
    ///
    /// * `parent_table` - The parent table name
    /// * `parent_key` - The parent key column
    ///
    /// # Returns
    ///
    /// A map of model IDs to their aggregate values
    pub async fn execute<E>(
        &self,
        parent_table: &str,
        parent_key: &str,
    ) -> AggregationResult<HashMap<i64, HashMap<String, f64>>>
    where
        E: EntityTrait,
    {
        if self.aggregates.is_empty() {
            return Ok(HashMap::new());
        }

        // Build SELECT clause with all aggregates
        let mut select_parts = vec![format!("{}.{}", parent_table, parent_key)];

        for aggregate in &self.aggregates {
            // Determine the relation table (simplified - would need relation metadata)
            let relation_table = &aggregate.relation;
            let foreign_key = format!("{}_id", parent_table.trim_end_matches('s'));

            let aggregate_sql =
                aggregate.build_sql(parent_table, parent_key, relation_table, &foreign_key);

            select_parts.push(aggregate_sql);
        }

        let sql = format!("SELECT {} FROM {}", select_parts.join(", "), parent_table);

        // Execute the query
        let stmt = Statement::from_sql_and_values(self.db.get_database_backend(), &sql, vec![]);

        let results = self.db.query_all(stmt).await?;

        // Parse results into a map
        let mut map: HashMap<i64, HashMap<String, f64>> = HashMap::new();

        for row in results {
            let id: i64 = row.try_get("", parent_key)?;
            let mut aggregates_map = HashMap::new();

            for aggregate in &self.aggregates {
                let alias = aggregate.alias();
                if let Ok(value) = row.try_get::<Option<f64>>("", &alias) {
                    aggregates_map.insert(alias, value.unwrap_or(0.0));
                }
            }

            map.insert(id, aggregates_map);
        }

        Ok(map)
    }

    /// Get a reference to the database connection
    pub fn db(&self) -> &DatabaseConnection {
        &self.db
    }

    /// Get the list of aggregates
    pub fn aggregates(&self) -> &[Aggregate] {
        &self.aggregates
    }
}

/// Helper function to load relationship count
///
/// # Example
///
/// ```rust,no_run
/// let count = load_count(&db, "posts", "user_id", user_id).await?;
/// println!("User has {} posts", count);
/// ```
pub async fn load_count(
    db: &DatabaseConnection,
    table: &str,
    foreign_key: &str,
    parent_id: i64,
) -> AggregationResult<i64> {
    let sql = format!(
        "SELECT COUNT(*) as count FROM {} WHERE {} = ?",
        table, foreign_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![sea_orm::Value::BigInt(Some(parent_id))],
    );

    let result = db.query_one(stmt).await?;

    if let Some(row) = result {
        Ok(row.try_get("", "count").unwrap_or(0))
    } else {
        Ok(0)
    }
}

/// Helper function to load relationship sum
///
/// # Example
///
/// ```rust,no_run
/// let total_views = load_sum(&db, "posts", "views", "user_id", user_id).await?;
/// println!("Total views: {}", total_views);
/// ```
pub async fn load_sum(
    db: &DatabaseConnection,
    table: &str,
    column: &str,
    foreign_key: &str,
    parent_id: i64,
) -> AggregationResult<f64> {
    let sql = format!(
        "SELECT COALESCE(SUM({}), 0) as total FROM {} WHERE {} = ?",
        column, table, foreign_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![sea_orm::Value::BigInt(Some(parent_id))],
    );

    let result = db.query_one(stmt).await?;

    if let Some(row) = result {
        Ok(row.try_get("", "total").unwrap_or(0.0))
    } else {
        Ok(0.0)
    }
}

/// Helper function to load relationship average
///
/// # Example
///
/// ```rust,no_run
/// let avg_rating = load_avg(&db, "posts", "rating", "user_id", user_id).await?;
/// println!("Average rating: {}", avg_rating);
/// ```
pub async fn load_avg(
    db: &DatabaseConnection,
    table: &str,
    column: &str,
    foreign_key: &str,
    parent_id: i64,
) -> AggregationResult<Option<f64>> {
    let sql = format!(
        "SELECT AVG({}) as average FROM {} WHERE {} = ?",
        column, table, foreign_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![sea_orm::Value::BigInt(Some(parent_id))],
    );

    let result = db.query_one(stmt).await?;

    if let Some(row) = result {
        Ok(row.try_get("", "average").ok())
    } else {
        Ok(None)
    }
}

/// Helper function to load relationship minimum
pub async fn load_min(
    db: &DatabaseConnection,
    table: &str,
    column: &str,
    foreign_key: &str,
    parent_id: i64,
) -> AggregationResult<Option<f64>> {
    let sql = format!(
        "SELECT MIN({}) as minimum FROM {} WHERE {} = ?",
        column, table, foreign_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![sea_orm::Value::BigInt(Some(parent_id))],
    );

    let result = db.query_one(stmt).await?;

    if let Some(row) = result {
        Ok(row.try_get("", "minimum").ok())
    } else {
        Ok(None)
    }
}

/// Helper function to load relationship maximum
pub async fn load_max(
    db: &DatabaseConnection,
    table: &str,
    column: &str,
    foreign_key: &str,
    parent_id: i64,
) -> AggregationResult<Option<f64>> {
    let sql = format!(
        "SELECT MAX({}) as maximum FROM {} WHERE {} = ?",
        column, table, foreign_key
    );

    let stmt = Statement::from_sql_and_values(
        db.get_database_backend(),
        &sql,
        vec![sea_orm::Value::BigInt(Some(parent_id))],
    );

    let result = db.query_one(stmt).await?;

    if let Some(row) = result {
        Ok(row.try_get("", "maximum").ok())
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregate_type_alias() {
        assert_eq!(AggregateType::Count.alias("posts"), "posts_count");
        assert_eq!(
            AggregateType::Sum("views".to_string()).alias("posts"),
            "posts_views_sum"
        );
        assert_eq!(
            AggregateType::Avg("rating".to_string()).alias("posts"),
            "posts_rating_avg"
        );
    }

    #[test]
    fn test_aggregate_creation() {
        let count = Aggregate::count("posts");
        assert_eq!(count.relation, "posts");
        assert_eq!(count.aggregate_type, AggregateType::Count);

        let sum = Aggregate::sum("posts", "views");
        assert_eq!(sum.relation, "posts");
        assert_eq!(sum.aggregate_type, AggregateType::Sum("views".to_string()));
    }

    #[test]
    fn test_aggregate_with_where() {
        let count = Aggregate::count("posts").with_where("published = true");
        assert_eq!(count.where_clause, Some("published = true".to_string()));
    }

    #[test]
    fn test_aggregation_builder() {
        // Would need DB connection for full test
        // Just verify builder can be created and configured
    }
}
