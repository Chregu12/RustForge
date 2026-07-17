//! Stripe Billing Portal

use crate::errors::{CashierError, CashierResult};
use serde::{Deserialize, Serialize};

/// Billing portal session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortalSession {
    pub id: String,
    pub url: String,
}

impl PortalSession {
    /// Create a billing portal session
    pub async fn create(customer_id: &str, return_url: &str) -> CashierResult<Self> {
        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::CreateBillingPortalSession::new(customer_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?);
        params.return_url = Some(return_url);

        let session = stripe::BillingPortalSession::create(&client, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        Ok(Self {
            id: session.id.to_string(),
            url: session.url,
        })
    }
}
