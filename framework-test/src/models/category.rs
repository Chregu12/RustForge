// Category model stub
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: i32,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub parent_id: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
