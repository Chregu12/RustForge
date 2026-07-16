//! Lenses for Nova resources
//!
//! Lenses provide custom query views for resources.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Lens trait
pub trait Lens: Send + Sync {
    /// Get lens name
    fn name(&self) -> &str;

    /// Get lens URI key (kebab-case identifier)
    fn uri_key(&self) -> String {
        self.name().to_lowercase().replace(' ', "-")
    }

    /// Get lens description
    fn description(&self) -> Option<&str> {
        None
    }

    /// Build the query modifications for this lens
    /// Returns a structured representation of query modifications
    fn query(&self) -> LensQuery;

    /// Serialize lens for JSON API
    fn to_json(&self) -> Value {
        serde_json::json!({
            "name": self.name(),
            "uri_key": self.uri_key(),
            "description": self.description(),
        })
    }
}

/// Lens query modifications
#[derive(Debug, Clone, Default)]
pub struct LensQuery {
    pub select: Vec<String>,
    pub joins: Vec<LensJoin>,
    pub wheres: Vec<LensWhere>,
    pub group_by: Vec<String>,
    pub having: Vec<LensHaving>,
    pub order_by: Vec<LensOrderBy>,
    pub limit: Option<u64>,
}

impl LensQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select(mut self, columns: Vec<String>) -> Self {
        self.select = columns;
        self
    }

    pub fn join(mut self, join: LensJoin) -> Self {
        self.joins.push(join);
        self
    }

    pub fn where_clause(mut self, where_clause: LensWhere) -> Self {
        self.wheres.push(where_clause);
        self
    }

    pub fn group_by(mut self, columns: Vec<String>) -> Self {
        self.group_by = columns;
        self
    }

    pub fn having(mut self, having: LensHaving) -> Self {
        self.having.push(having);
        self
    }

    pub fn order_by(mut self, field: String, direction: OrderDirection) -> Self {
        self.order_by.push(LensOrderBy { field, direction });
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }
}

/// Join clause for lens queries
#[derive(Debug, Clone)]
pub struct LensJoin {
    pub table: String,
    pub first: String,
    pub operator: String,
    pub second: String,
    pub join_type: JoinType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Cross,
}

impl LensJoin {
    pub fn inner(table: impl Into<String>, first: impl Into<String>, second: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            first: first.into(),
            operator: "=".to_string(),
            second: second.into(),
            join_type: JoinType::Inner,
        }
    }

    pub fn left(table: impl Into<String>, first: impl Into<String>, second: impl Into<String>) -> Self {
        Self {
            table: table.into(),
            first: first.into(),
            operator: "=".to_string(),
            second: second.into(),
            join_type: JoinType::Left,
        }
    }
}

/// Where clause for lens queries
#[derive(Debug, Clone)]
pub struct LensWhere {
    pub field: String,
    pub operator: String,
    pub value: Value,
}

impl LensWhere {
    pub fn new(field: impl Into<String>, operator: impl Into<String>, value: Value) -> Self {
        Self {
            field: field.into(),
            operator: operator.into(),
            value,
        }
    }

    pub fn equals(field: impl Into<String>, value: Value) -> Self {
        Self::new(field, "=", value)
    }

    pub fn gt(field: impl Into<String>, value: Value) -> Self {
        Self::new(field, ">", value)
    }

    pub fn lt(field: impl Into<String>, value: Value) -> Self {
        Self::new(field, "<", value)
    }
}

/// Having clause for lens queries
#[derive(Debug, Clone)]
pub struct LensHaving {
    pub expression: String,
    pub operator: String,
    pub value: Value,
}

impl LensHaving {
    pub fn new(expression: impl Into<String>, operator: impl Into<String>, value: Value) -> Self {
        Self {
            expression: expression.into(),
            operator: operator.into(),
            value,
        }
    }
}

/// Order by clause
#[derive(Debug, Clone)]
pub struct LensOrderBy {
    pub field: String,
    pub direction: OrderDirection,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// Example lens implementations

/// Most recent lens - shows recently created items
pub struct MostRecentLens {
    pub name: String,
    pub field: String,
    pub limit: u64,
}

impl MostRecentLens {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            field: "created_at".to_string(),
            limit: 50,
        }
    }

    pub fn field(mut self, field: impl Into<String>) -> Self {
        self.field = field.into();
        self
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = limit;
        self
    }
}

impl Lens for MostRecentLens {
    fn name(&self) -> &str {
        &self.name
    }

    fn query(&self) -> LensQuery {
        LensQuery::new()
            .order_by(self.field.clone(), OrderDirection::Desc)
            .limit(self.limit)
    }
}

/// Top items lens - shows items with highest count of something
pub struct TopItemsLens {
    pub name: String,
    pub count_field: String,
    pub limit: u64,
}

impl TopItemsLens {
    pub fn new(name: impl Into<String>, count_field: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            count_field: count_field.into(),
            limit: 10,
        }
    }

    pub fn limit(mut self, limit: u64) -> Self {
        self.limit = limit;
        self
    }
}

impl Lens for TopItemsLens {
    fn name(&self) -> &str {
        &self.name
    }

    fn query(&self) -> LensQuery {
        LensQuery::new()
            .order_by(self.count_field.clone(), OrderDirection::Desc)
            .limit(self.limit)
    }
}

/// Helper macro to create lenses
#[macro_export]
macro_rules! lens {
    (
        name: $name:expr,
        query: |$q:ident| $body:expr
    ) => {
        {
            struct CustomLens;

            impl $crate::lens::Lens for CustomLens {
                fn name(&self) -> &str {
                    $name
                }

                fn query(&self) -> $crate::lens::LensQuery {
                    let $q = $crate::lens::LensQuery::new();
                    $body
                }
            }

            Box::new(CustomLens) as Box<dyn $crate::lens::Lens>
        }
    };
}
