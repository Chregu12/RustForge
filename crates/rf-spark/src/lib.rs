//! SaaS Billing and Subscription Management for RustForge
//!
//! This crate provides Laravel Spark/Cashier-like functionality for managing
//! subscriptions, payments, and billing in SaaS applications.
//!
//! # Features
//!
//! - **Stripe Integration**: Full Stripe API support
//! - **Subscriptions**: Create, update, cancel, resume subscriptions
//! - **Payment Methods**: Manage credit cards and payment sources
//! - **Invoices**: Generate and send invoices
//! - **Webhooks**: Handle Stripe webhook events
//! - **Metered Billing**: Usage-based billing support
//! - **Trials**: Free trial periods
//! - **Coupons**: Discount codes and promotions
//!
//! # Quick Start
//!
//! ```ignore
//! use rf_spark::{Spark, SparkConfig, Billable};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), rf_spark::SparkError> {
//!     let spark = Spark::new(SparkConfig {
//!         stripe_key: std::env::var("STRIPE_SECRET_KEY")?,
//!         webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET")?,
//!         ..Default::default()
//!     });
//!
//!     // Create a subscription
//!     let subscription = spark
//!         .user("user_123")
//!         .new_subscription("default", "price_monthly")
//!         .with_trial_days(14)
//!         .create()
//!         .await?;
//!
//!     // Check subscription status
//!     if spark.user("user_123").subscribed("default").await? {
//!         println!("User is subscribed!");
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Billable Trait
//!
//! ```ignore
//! use rf_spark::Billable;
//!
//! struct User {
//!     id: String,
//!     email: String,
//!     stripe_id: Option<String>,
//! }
//!
//! impl Billable for User {
//!     fn stripe_id(&self) -> Option<&str> {
//!         self.stripe_id.as_deref()
//!     }
//!
//!     fn email(&self) -> &str {
//!         &self.email
//!     }
//! }
//! ```

// The builder-style structs here are populated field-by-field after
// `Default::default()` for readability; this is intentional.
#![allow(clippy::field_reassign_with_default)]

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub mod customer;
pub mod invoice;
pub mod payment;
pub mod subscription;
pub mod webhook;

pub use customer::Customer;
pub use invoice::{Invoice, InvoiceBuilder};
pub use payment::{PaymentIntent, PaymentMethod};
pub use subscription::{Subscription, SubscriptionBuilder, SubscriptionStatus};
pub use webhook::{WebhookHandler, WebhookEvent};

/// Spark error types
#[derive(Debug, Error)]
pub enum SparkError {
    #[error("Stripe error: {0}")]
    StripeError(String),

    #[error("Customer not found: {0}")]
    CustomerNotFound(String),

    #[error("Subscription error: {0}")]
    SubscriptionError(String),

    #[error("Payment error: {0}")]
    PaymentError(String),

    #[error("Webhook error: {0}")]
    WebhookError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),
}

pub type SparkResult<T> = Result<T, SparkError>;

/// Spark configuration
#[derive(Debug, Clone)]
pub struct SparkConfig {
    /// Stripe secret key
    pub stripe_key: String,
    /// Stripe webhook signing secret
    pub webhook_secret: String,
    /// Currency (e.g., "usd", "eur")
    pub currency: String,
    /// Default trial days
    pub trial_days: u32,
    /// Prorate when changing plans
    pub prorate: bool,
    /// Invoice prefix
    pub invoice_prefix: String,
}

impl Default for SparkConfig {
    fn default() -> Self {
        Self {
            stripe_key: String::new(),
            webhook_secret: String::new(),
            currency: "usd".to_string(),
            trial_days: 0,
            prorate: true,
            invoice_prefix: "INV-".to_string(),
        }
    }
}

/// Billable trait for entities that can be billed
#[async_trait]
pub trait Billable: Send + Sync {
    /// Get the Stripe customer ID
    fn stripe_id(&self) -> Option<&str>;

    /// Get the entity's email
    fn email(&self) -> &str;

    /// Get the entity's name
    fn name(&self) -> Option<&str> {
        None
    }

    /// Set the Stripe customer ID
    async fn set_stripe_id(&mut self, id: &str) -> SparkResult<()>;
}

/// Main Spark instance
pub struct Spark {
    config: SparkConfig,
    stripe: stripe::Client,
}

impl Spark {
    /// Create a new Spark instance
    pub fn new(config: SparkConfig) -> Self {
        let stripe = stripe::Client::new(&config.stripe_key);

        Self { config, stripe }
    }

    /// Get a billable user context
    pub fn user(&self, user_id: &str) -> BillableContext<'_> {
        BillableContext {
            spark: self,
            user_id: user_id.to_string(),
            stripe_customer_id: None,
        }
    }

    /// Get a billable user context with existing Stripe ID
    pub fn customer(&self, user_id: &str, stripe_id: &str) -> BillableContext<'_> {
        BillableContext {
            spark: self,
            user_id: user_id.to_string(),
            stripe_customer_id: Some(stripe_id.to_string()),
        }
    }

    /// Verify a webhook signature
    pub fn verify_webhook(&self, payload: &str, signature: &str) -> SparkResult<WebhookEvent> {
        webhook::verify_signature(payload, signature, &self.config.webhook_secret)
    }

    /// Get webhook handler
    pub fn webhooks(&self) -> WebhookHandler {
        WebhookHandler::new(self.config.webhook_secret.clone())
    }

    /// Get Stripe client
    pub fn stripe(&self) -> &stripe::Client {
        &self.stripe
    }

    /// Get configuration
    pub fn config(&self) -> &SparkConfig {
        &self.config
    }
}

/// Context for a billable user
pub struct BillableContext<'a> {
    spark: &'a Spark,
    user_id: String,
    stripe_customer_id: Option<String>,
}

impl<'a> BillableContext<'a> {
    /// Create or get a Stripe customer
    pub async fn create_or_get_customer(
        &self,
        email: &str,
        name: Option<&str>,
    ) -> SparkResult<Customer> {
        customer::create_or_get(
            &self.spark.stripe,
            self.stripe_customer_id.as_deref(),
            email,
            name,
        )
        .await
    }

    /// Start building a new subscription
    pub fn new_subscription(&self, name: &str, price_id: &str) -> SubscriptionBuilder<'a> {
        SubscriptionBuilder::new(
            self.spark,
            &self.user_id,
            self.stripe_customer_id.as_deref(),
            name,
            price_id,
        )
    }

    /// Check if user is subscribed to a plan
    pub async fn subscribed(&self, name: &str) -> SparkResult<bool> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::is_subscribed(&self.spark.stripe, stripe_id, name).await
        } else {
            Ok(false)
        }
    }

    /// Check if user is on trial
    pub async fn on_trial(&self, name: &str) -> SparkResult<bool> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::is_on_trial(&self.spark.stripe, stripe_id, name).await
        } else {
            Ok(false)
        }
    }

    /// Check if subscription has ended
    pub async fn ended(&self, name: &str) -> SparkResult<bool> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::has_ended(&self.spark.stripe, stripe_id, name).await
        } else {
            Ok(true)
        }
    }

    /// Get the current subscription
    pub async fn subscription(&self, name: &str) -> SparkResult<Option<Subscription>> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::get_subscription(&self.spark.stripe, stripe_id, name).await
        } else {
            Ok(None)
        }
    }

    /// Get all subscriptions
    pub async fn subscriptions(&self) -> SparkResult<Vec<Subscription>> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::list_subscriptions(&self.spark.stripe, stripe_id).await
        } else {
            Ok(vec![])
        }
    }

    /// Cancel a subscription
    pub async fn cancel_subscription(&self, name: &str) -> SparkResult<()> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::cancel(&self.spark.stripe, stripe_id, name).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Resume a cancelled subscription
    pub async fn resume_subscription(&self, name: &str) -> SparkResult<()> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::resume(&self.spark.stripe, stripe_id, name).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Swap to a different plan
    pub async fn swap_subscription(&self, name: &str, new_price_id: &str) -> SparkResult<Subscription> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            subscription::swap(&self.spark.stripe, stripe_id, name, new_price_id, self.spark.config.prorate).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Add a payment method
    pub async fn add_payment_method(&self, payment_method_id: &str) -> SparkResult<PaymentMethod> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            payment::add_payment_method(&self.spark.stripe, stripe_id, payment_method_id).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Get default payment method
    pub async fn default_payment_method(&self) -> SparkResult<Option<PaymentMethod>> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            payment::get_default_payment_method(&self.spark.stripe, stripe_id).await
        } else {
            Ok(None)
        }
    }

    /// Update default payment method
    pub async fn update_default_payment_method(&self, payment_method_id: &str) -> SparkResult<()> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            payment::set_default_payment_method(&self.spark.stripe, stripe_id, payment_method_id).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Get all payment methods
    pub async fn payment_methods(&self) -> SparkResult<Vec<PaymentMethod>> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            payment::list_payment_methods(&self.spark.stripe, stripe_id).await
        } else {
            Ok(vec![])
        }
    }

    /// Delete a payment method
    pub async fn delete_payment_method(&self, payment_method_id: &str) -> SparkResult<()> {
        payment::delete_payment_method(&self.spark.stripe, payment_method_id).await
    }

    /// Get invoices
    pub async fn invoices(&self) -> SparkResult<Vec<Invoice>> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            invoice::list_invoices(&self.spark.stripe, stripe_id).await
        } else {
            Ok(vec![])
        }
    }

    /// Get upcoming invoice
    pub async fn upcoming_invoice(&self) -> SparkResult<Option<Invoice>> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            invoice::get_upcoming(&self.spark.stripe, stripe_id).await
        } else {
            Ok(None)
        }
    }

    /// Download invoice PDF
    pub async fn download_invoice(&self, invoice_id: &str) -> SparkResult<String> {
        invoice::get_pdf_url(&self.spark.stripe, invoice_id).await
    }

    /// Charge a one-time amount
    pub async fn charge(&self, amount: Decimal, description: &str) -> SparkResult<PaymentIntent> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            payment::create_charge(
                &self.spark.stripe,
                stripe_id,
                amount,
                &self.spark.config.currency,
                description,
            )
            .await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Refund a charge
    pub async fn refund(&self, payment_intent_id: &str, amount: Option<Decimal>) -> SparkResult<()> {
        payment::create_refund(&self.spark.stripe, payment_intent_id, amount).await
    }

    /// Report metered usage
    pub async fn report_usage(
        &self,
        subscription_item_id: &str,
        quantity: i64,
        timestamp: Option<DateTime<Utc>>,
    ) -> SparkResult<()> {
        subscription::report_usage(&self.spark.stripe, subscription_item_id, quantity, timestamp).await
    }

    /// Get usage records
    pub async fn usage_records(
        &self,
        subscription_item_id: &str,
    ) -> SparkResult<Vec<subscription::UsageRecord>> {
        subscription::list_usage_records(&self.spark.stripe, subscription_item_id).await
    }

    /// Apply a coupon
    pub async fn apply_coupon(&self, coupon_code: &str) -> SparkResult<()> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            customer::apply_coupon(&self.spark.stripe, stripe_id, coupon_code).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }

    /// Get billing portal URL
    pub async fn billing_portal_url(&self, return_url: &str) -> SparkResult<String> {
        if let Some(ref stripe_id) = self.stripe_customer_id {
            customer::create_billing_portal_session(&self.spark.stripe, stripe_id, return_url).await
        } else {
            Err(SparkError::CustomerNotFound(self.user_id.clone()))
        }
    }
}

/// Plan/Price information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    pub id: String,
    pub name: String,
    pub price: Decimal,
    pub currency: String,
    pub interval: BillingInterval,
    pub features: Vec<String>,
}

/// Billing interval
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingInterval {
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl BillingInterval {
    pub fn to_stripe(&self) -> &'static str {
        match self {
            BillingInterval::Daily => "day",
            BillingInterval::Weekly => "week",
            BillingInterval::Monthly => "month",
            BillingInterval::Yearly => "year",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spark_config_default() {
        let config = SparkConfig::default();
        assert_eq!(config.currency, "usd");
        assert!(config.prorate);
    }

    #[test]
    fn test_billing_interval() {
        assert_eq!(BillingInterval::Monthly.to_stripe(), "month");
        assert_eq!(BillingInterval::Yearly.to_stripe(), "year");
    }
}
