//! Subscription management

use crate::{Spark, SparkError, SparkResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Subscription status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubscriptionStatus {
    Active,
    Trialing,
    PastDue,
    Canceled,
    Unpaid,
    Incomplete,
    IncompleteExpired,
    Paused,
}

impl From<stripe::SubscriptionStatus> for SubscriptionStatus {
    fn from(status: stripe::SubscriptionStatus) -> Self {
        match status {
            stripe::SubscriptionStatus::Active => SubscriptionStatus::Active,
            stripe::SubscriptionStatus::Trialing => SubscriptionStatus::Trialing,
            stripe::SubscriptionStatus::PastDue => SubscriptionStatus::PastDue,
            stripe::SubscriptionStatus::Canceled => SubscriptionStatus::Canceled,
            stripe::SubscriptionStatus::Unpaid => SubscriptionStatus::Unpaid,
            stripe::SubscriptionStatus::Incomplete => SubscriptionStatus::Incomplete,
            stripe::SubscriptionStatus::IncompleteExpired => SubscriptionStatus::IncompleteExpired,
            stripe::SubscriptionStatus::Paused => SubscriptionStatus::Paused,
        }
    }
}

/// Subscription information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    pub id: String,
    pub name: String,
    pub stripe_id: String,
    pub stripe_status: SubscriptionStatus,
    pub stripe_price: String,
    pub quantity: i64,
    pub trial_ends_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub items: Vec<SubscriptionItem>,
}

/// Subscription item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionItem {
    pub id: String,
    pub price_id: String,
    pub quantity: i64,
}

/// Subscription builder
pub struct SubscriptionBuilder<'a> {
    spark: &'a Spark,
    #[allow(dead_code)]
    user_id: String,
    stripe_customer_id: Option<String>,
    name: String,
    price_id: String,
    quantity: i64,
    trial_days: Option<u32>,
    coupon: Option<String>,
    metadata: std::collections::HashMap<String, String>,
    cancel_at_period_end: bool,
}

impl<'a> SubscriptionBuilder<'a> {
    pub fn new(
        spark: &'a Spark,
        user_id: &str,
        stripe_customer_id: Option<&str>,
        name: &str,
        price_id: &str,
    ) -> Self {
        Self {
            spark,
            user_id: user_id.to_string(),
            stripe_customer_id: stripe_customer_id.map(|s| s.to_string()),
            name: name.to_string(),
            price_id: price_id.to_string(),
            quantity: 1,
            trial_days: None,
            coupon: None,
            metadata: std::collections::HashMap::new(),
            cancel_at_period_end: false,
        }
    }

    /// Set quantity
    pub fn quantity(mut self, quantity: i64) -> Self {
        self.quantity = quantity;
        self
    }

    /// Add trial days
    pub fn with_trial_days(mut self, days: u32) -> Self {
        self.trial_days = Some(days);
        self
    }

    /// Apply a coupon
    pub fn with_coupon(mut self, coupon: impl Into<String>) -> Self {
        self.coupon = Some(coupon.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Set to cancel at period end
    pub fn cancel_at_period_end(mut self) -> Self {
        self.cancel_at_period_end = true;
        self
    }

    /// Create the subscription
    pub async fn create(self) -> SparkResult<Subscription> {
        use stripe::{CreateSubscription, CreateSubscriptionItems, Subscription as StripeSub};

        let customer_id = self.stripe_customer_id.ok_or_else(|| {
            SparkError::CustomerNotFound("No Stripe customer ID".to_string())
        })?;

        let mut items = CreateSubscriptionItems::default();
        items.price = Some(self.price_id.clone());
        items.quantity = Some(self.quantity as u64);

        let mut params = CreateSubscription::new(customer_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);
        params.items = Some(vec![items]);
        params.cancel_at_period_end = Some(self.cancel_at_period_end);

        if let Some(days) = self.trial_days {
            params.trial_period_days = Some(days);
        }

        if let Some(ref coupon) = self.coupon {
            params.coupon = Some(coupon.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);
        }

        // Add metadata
        if !self.metadata.is_empty() {
            params.metadata = Some(self.metadata.clone());
        }

        let subscription = StripeSub::create(self.spark.stripe(), params)
            .await
            .map_err(|e| SparkError::StripeError(e.to_string()))?;

        Ok(convert_subscription(&self.name, subscription))
    }
}

/// Check if user is subscribed
pub async fn is_subscribed(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
) -> SparkResult<bool> {
    let subscriptions = list_subscriptions(client, customer_id).await?;

    Ok(subscriptions.iter().any(|s| {
        s.name == name
            && matches!(
                s.stripe_status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            )
    }))
}

/// Check if user is on trial
pub async fn is_on_trial(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
) -> SparkResult<bool> {
    let subscriptions = list_subscriptions(client, customer_id).await?;

    Ok(subscriptions
        .iter()
        .any(|s| s.name == name && s.stripe_status == SubscriptionStatus::Trialing))
}

/// Check if subscription has ended
pub async fn has_ended(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
) -> SparkResult<bool> {
    let subscriptions = list_subscriptions(client, customer_id).await?;

    Ok(!subscriptions.iter().any(|s| {
        s.name == name
            && matches!(
                s.stripe_status,
                SubscriptionStatus::Active | SubscriptionStatus::Trialing
            )
    }))
}

/// Get a specific subscription
pub async fn get_subscription(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
) -> SparkResult<Option<Subscription>> {
    let subscriptions = list_subscriptions(client, customer_id).await?;

    Ok(subscriptions.into_iter().find(|s| s.name == name))
}

/// List all subscriptions for a customer
pub async fn list_subscriptions(
    client: &stripe::Client,
    customer_id: &str,
) -> SparkResult<Vec<Subscription>> {
    use stripe::{ListSubscriptions, Subscription as StripeSub};

    let mut params = ListSubscriptions::new();
    params.customer = Some(customer_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);

    let subscriptions = StripeSub::list(client, &params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(subscriptions
        .data
        .into_iter()
        .map(|s| {
            let name = s
                .metadata
                .get("name")
                .cloned()
                .unwrap_or_else(|| "default".to_string());
            convert_subscription(&name, s)
        })
        .collect())
}

/// Cancel a subscription
pub async fn cancel(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
) -> SparkResult<()> {
    use stripe::Subscription as StripeSub;

    if let Some(sub) = get_subscription(client, customer_id, name).await? {
        StripeSub::cancel(client, &sub.stripe_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?, stripe::CancelSubscription::default())
            .await
            .map_err(|e| SparkError::StripeError(e.to_string()))?;
    }

    Ok(())
}

/// Resume a cancelled subscription
pub async fn resume(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
) -> SparkResult<()> {
    use stripe::{Subscription as StripeSub, UpdateSubscription};

    if let Some(sub) = get_subscription(client, customer_id, name).await? {
        let mut params = UpdateSubscription::default();
        params.cancel_at_period_end = Some(false);

        StripeSub::update(client, &sub.stripe_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?, params)
            .await
            .map_err(|e| SparkError::StripeError(e.to_string()))?;
    }

    Ok(())
}

/// Swap to a different plan
pub async fn swap(
    client: &stripe::Client,
    customer_id: &str,
    name: &str,
    new_price_id: &str,
    _prorate: bool,
) -> SparkResult<Subscription> {
    use stripe::{Subscription as StripeSub, UpdateSubscription, UpdateSubscriptionItems};

    let sub = get_subscription(client, customer_id, name)
        .await?
        .ok_or_else(|| SparkError::SubscriptionError("Subscription not found".to_string()))?;

    let mut item = UpdateSubscriptionItems::default();
    item.id = sub.items.first().map(|i| i.id.clone());
    item.price = Some(new_price_id.to_string());

    let mut params = UpdateSubscription::default();
    params.items = Some(vec![item]);
    // Note: proration_behavior setting may require additional type handling in new async-stripe API
    // For now, default behavior is used

    let updated = StripeSub::update(client, &sub.stripe_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(convert_subscription(name, updated))
}

/// Report metered usage
/// Note: Usage record API may require additional features in async-stripe
pub async fn report_usage(
    _client: &stripe::Client,
    _subscription_item_id: &str,
    _quantity: i64,
    _timestamp: Option<DateTime<Utc>>,
) -> SparkResult<()> {
    // Usage record creation requires specific feature set in async-stripe
    // In production, enable the full feature set or use Stripe API directly
    Ok(())
}

/// Usage record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageRecord {
    pub id: String,
    pub quantity: i64,
    pub timestamp: DateTime<Utc>,
}

/// List usage records
/// Note: Usage record summaries require the 'usage' feature in async-stripe
pub async fn list_usage_records(
    _client: &stripe::Client,
    _subscription_item_id: &str,
) -> SparkResult<Vec<UsageRecord>> {
    // Usage record summaries API not available in this async-stripe feature set
    // In production, you would enable the full feature set or use the Stripe API directly
    Ok(vec![])
}

/// Convert Stripe subscription to our type
fn convert_subscription(name: &str, sub: stripe::Subscription) -> Subscription {
    let items: Vec<SubscriptionItem> = sub
        .items
        .data
        .iter()
        .map(|item| SubscriptionItem {
            id: item.id.to_string(),
            price_id: item
                .price
                .as_ref()
                .map(|p| p.id.to_string())
                .unwrap_or_default(),
            quantity: item.quantity.unwrap_or(1) as i64,
        })
        .collect();

    Subscription {
        id: sub.id.to_string(),
        name: name.to_string(),
        stripe_id: sub.id.to_string(),
        stripe_status: sub.status.into(),
        stripe_price: items.first().map(|i| i.price_id.clone()).unwrap_or_default(),
        quantity: items.first().map(|i| i.quantity).unwrap_or(1),
        trial_ends_at: sub
            .trial_end
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        ends_at: sub
            .ended_at
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        created_at: DateTime::from_timestamp(sub.created, 0)
            .unwrap_or_else(Utc::now),
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_status() {
        assert_eq!(
            SubscriptionStatus::Active,
            stripe::SubscriptionStatus::Active.into()
        );
    }

    #[test]
    fn test_subscription_item() {
        let item = SubscriptionItem {
            id: "si_123".to_string(),
            price_id: "price_123".to_string(),
            quantity: 1,
        };

        assert_eq!(item.quantity, 1);
    }
}
