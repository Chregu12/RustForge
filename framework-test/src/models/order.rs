// Order model stub
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: i32,
    pub user_id: i32,
    pub order_number: String,
    pub status: String,
    pub total_amount: Decimal,
    pub currency: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
