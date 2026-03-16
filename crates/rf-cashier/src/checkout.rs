//! Stripe Checkout session management

use crate::errors::{CashierError, CashierResult};
use serde::{Deserialize, Serialize};

/// Checkout session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckoutSession {
    pub id: String,
    pub url: String,
}

/// Builder for creating checkout sessions
pub struct CheckoutBuilder {
    #[allow(dead_code)]
    billable_id: i64,
    price: String,
    quantity: i32,
    success_url: Option<String>,
    cancel_url: Option<String>,
    mode: CheckoutMode,
    allow_promotion_codes: bool,
    metadata: std::collections::HashMap<String, String>,
}

/// Checkout mode
#[derive(Debug, Clone)]
pub enum CheckoutMode {
    Payment,
    Subscription,
    Setup,
}

impl CheckoutBuilder {
    /// Create a new checkout builder
    pub fn new(billable_id: i64, price: &str) -> Self {
        Self {
            billable_id,
            price: price.to_string(),
            quantity: 1,
            success_url: None,
            cancel_url: None,
            mode: CheckoutMode::Subscription,
            allow_promotion_codes: false,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set quantity
    pub fn quantity(mut self, quantity: i32) -> Self {
        self.quantity = quantity;
        self
    }

    /// Set success URL
    pub fn success_url(mut self, url: &str) -> Self {
        self.success_url = Some(url.to_string());
        self
    }

    /// Set cancel URL
    pub fn cancel_url(mut self, url: &str) -> Self {
        self.cancel_url = Some(url.to_string());
        self
    }

    /// Set mode to payment (one-time)
    pub fn as_payment(mut self) -> Self {
        self.mode = CheckoutMode::Payment;
        self
    }

    /// Set mode to subscription
    pub fn as_subscription(mut self) -> Self {
        self.mode = CheckoutMode::Subscription;
        self
    }

    /// Set mode to setup (save payment method)
    pub fn as_setup(mut self) -> Self {
        self.mode = CheckoutMode::Setup;
        self
    }

    /// Allow promotion codes
    pub fn allow_promotion_codes(mut self) -> Self {
        self.allow_promotion_codes = true;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Create the checkout session
    pub async fn create(self, customer_id: &str) -> CashierResult<CheckoutSession> {
        let client = crate::config::get_config().stripe_client();

        let mode = match self.mode {
            CheckoutMode::Payment => stripe::CheckoutSessionMode::Payment,
            CheckoutMode::Subscription => stripe::CheckoutSessionMode::Subscription,
            CheckoutMode::Setup => stripe::CheckoutSessionMode::Setup,
        };

        let line_item = stripe::CreateCheckoutSessionLineItems {
            price: Some(self.price),
            quantity: Some(self.quantity as u64),
            ..Default::default()
        };

        let mut params = stripe::CreateCheckoutSession::new();
        params.customer = Some(customer_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?);
        params.mode = Some(mode);
        params.line_items = Some(vec![line_item]);
        params.success_url = self.success_url.as_deref();
        params.cancel_url = self.cancel_url.as_deref();
        params.allow_promotion_codes = Some(self.allow_promotion_codes);

        let session = stripe::CheckoutSession::create(&client, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        Ok(CheckoutSession {
            id: session.id.to_string(),
            url: session.url.unwrap_or_default(),
        })
    }
}
