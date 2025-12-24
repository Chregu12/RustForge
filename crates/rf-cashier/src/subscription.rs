//! Subscription management

use crate::errors::{CashierError, CashierResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Subscription status
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Active,
    Canceled,
    Incomplete,
    IncompleteExpired,
    PastDue,
    Trialing,
    Unpaid,
    Paused,
}

impl From<stripe::SubscriptionStatus> for SubscriptionStatus {
    fn from(status: stripe::SubscriptionStatus) -> Self {
        match status {
            stripe::SubscriptionStatus::Active => SubscriptionStatus::Active,
            stripe::SubscriptionStatus::Canceled => SubscriptionStatus::Canceled,
            stripe::SubscriptionStatus::Incomplete => SubscriptionStatus::Incomplete,
            stripe::SubscriptionStatus::IncompleteExpired => SubscriptionStatus::IncompleteExpired,
            stripe::SubscriptionStatus::PastDue => SubscriptionStatus::PastDue,
            stripe::SubscriptionStatus::Trialing => SubscriptionStatus::Trialing,
            stripe::SubscriptionStatus::Unpaid => SubscriptionStatus::Unpaid,
            stripe::SubscriptionStatus::Paused => SubscriptionStatus::Paused,
        }
    }
}

/// A subscription
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: i64,
    pub billable_id: i64,
    pub name: String,
    pub stripe_id: String,
    pub stripe_status: SubscriptionStatus,
    pub stripe_price: Option<String>,
    pub quantity: i32,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Subscription {
    /// Check if subscription is active
    pub fn is_active(&self) -> bool {
        matches!(
            self.stripe_status,
            SubscriptionStatus::Active | SubscriptionStatus::Trialing
        ) && self.ends_at.is_none()
    }

    /// Check if subscription is on trial
    pub fn on_trial(&self) -> bool {
        self.stripe_status == SubscriptionStatus::Trialing
    }

    /// Check if subscription is canceled
    pub fn is_canceled(&self) -> bool {
        self.ends_at.is_some()
    }

    /// Check if subscription has ended
    pub fn has_ended(&self) -> bool {
        if let Some(ends_at) = self.ends_at {
            ends_at < Utc::now()
        } else {
            false
        }
    }

    /// Check if subscription is on grace period
    pub fn on_grace_period(&self) -> bool {
        if let Some(ends_at) = self.ends_at {
            ends_at > Utc::now()
        } else {
            false
        }
    }

    /// Cancel the subscription
    pub async fn cancel(&mut self) -> CashierResult<()> {
        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::UpdateSubscription::new();
        params.cancel_at_period_end = Some(true);

        let sub = stripe::Subscription::update(&client, &self.stripe_id.parse().unwrap(), params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        if let Some(cancel_at) = sub.cancel_at {
            self.ends_at = Some(DateTime::from_timestamp(cancel_at, 0).unwrap_or(Utc::now()));
        }

        Ok(())
    }

    /// Cancel the subscription immediately
    pub async fn cancel_now(&mut self) -> CashierResult<()> {
        let client = crate::config::get_config().stripe_client();

        stripe::Subscription::cancel(&client, &self.stripe_id.parse().unwrap(), stripe::CancelSubscription::default())
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        self.stripe_status = SubscriptionStatus::Canceled;
        self.ends_at = Some(Utc::now());

        Ok(())
    }

    /// Resume the subscription (if on grace period)
    pub async fn resume(&mut self) -> CashierResult<()> {
        if !self.on_grace_period() {
            return Err(CashierError::AlreadyCancelled);
        }

        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::UpdateSubscription::new();
        params.cancel_at_period_end = Some(false);

        stripe::Subscription::update(&client, &self.stripe_id.parse().unwrap(), params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        self.ends_at = None;

        Ok(())
    }

    /// Swap to a different price
    pub async fn swap(&mut self, price: &str) -> CashierResult<()> {
        let client = crate::config::get_config().stripe_client();

        // Get current subscription to find item ID
        let sub = stripe::Subscription::retrieve(&client, &self.stripe_id.parse().unwrap(), &[])
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        let item_id = sub.items.data.first()
            .ok_or_else(|| CashierError::SubscriptionNotFound("No items".to_string()))?
            .id.to_string();

        let items = vec![stripe::UpdateSubscriptionItems {
            id: Some(item_id),
            price: Some(price.to_string()),
            ..Default::default()
        }];

        let mut params = stripe::UpdateSubscription::new();
        params.items = Some(items);
        // Proration is enabled by default, we'll skip explicit setting

        stripe::Subscription::update(&client, &self.stripe_id.parse().unwrap(), params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        self.stripe_price = Some(price.to_string());

        Ok(())
    }

    /// Update quantity
    pub async fn update_quantity(&mut self, quantity: i32) -> CashierResult<()> {
        let client = crate::config::get_config().stripe_client();

        let sub = stripe::Subscription::retrieve(&client, &self.stripe_id.parse().unwrap(), &[])
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        let item_id = sub.items.data.first()
            .ok_or_else(|| CashierError::SubscriptionNotFound("No items".to_string()))?
            .id.to_string();

        let items = vec![stripe::UpdateSubscriptionItems {
            id: Some(item_id),
            quantity: Some(quantity as u64),
            ..Default::default()
        }];

        let mut params = stripe::UpdateSubscription::new();
        params.items = Some(items);

        stripe::Subscription::update(&client, &self.stripe_id.parse().unwrap(), params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        self.quantity = quantity;

        Ok(())
    }

    /// Find subscription by name
    pub async fn find_by_name(_billable_id: i64, _name: &str) -> CashierResult<Option<Subscription>> {
        // In a real implementation, this would query the database
        // For now, return None as placeholder
        Ok(None)
    }

    /// Find all subscriptions for a billable
    pub async fn find_all(_billable_id: i64) -> CashierResult<Vec<Subscription>> {
        // In a real implementation, this would query the database
        Ok(vec![])
    }
}

/// Builder for creating subscriptions
pub struct SubscriptionBuilder {
    #[allow(dead_code)]
    billable_id: i64,
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    price: String,
    quantity: i32,
    trial_days: Option<i32>,
    trial_until: Option<DateTime<Utc>>,
    coupon: Option<String>,
    metadata: std::collections::HashMap<String, String>,
}

impl SubscriptionBuilder {
    /// Create a new subscription builder
    pub fn new(billable_id: i64, name: &str, price: &str) -> Self {
        Self {
            billable_id,
            name: name.to_string(),
            price: price.to_string(),
            quantity: 1,
            trial_days: None,
            trial_until: None,
            coupon: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set quantity
    pub fn quantity(mut self, quantity: i32) -> Self {
        self.quantity = quantity;
        self
    }

    /// Set trial days
    pub fn trial_days(mut self, days: i32) -> Self {
        self.trial_days = Some(days);
        self
    }

    /// Set trial end date
    pub fn trial_until(mut self, until: DateTime<Utc>) -> Self {
        self.trial_until = Some(until);
        self
    }

    /// Skip trial
    pub fn skip_trial(mut self) -> Self {
        self.trial_days = Some(0);
        self.trial_until = None;
        self
    }

    /// Apply a coupon
    pub fn with_coupon(mut self, coupon: &str) -> Self {
        self.coupon = Some(coupon.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Create the subscription
    pub async fn create(self, _payment_method: &str) -> CashierResult<Subscription> {
        let _client = crate::config::get_config().stripe_client();

        // This would need the customer ID from the billable
        // For now, return a placeholder error
        Err(CashierError::Other("Subscription creation requires database integration".to_string()))
    }
}
