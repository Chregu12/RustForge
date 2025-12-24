//! Cashier configuration

use std::sync::OnceLock;

static CONFIG: OnceLock<CashierConfig> = OnceLock::new();

/// Cashier configuration
#[derive(Debug, Clone)]
pub struct CashierConfig {
    /// Stripe secret key
    pub stripe_secret: String,
    /// Stripe publishable key
    pub stripe_key: String,
    /// Webhook secret for verifying signatures
    pub webhook_secret: Option<String>,
    /// Default currency (e.g., "usd")
    pub currency: String,
    /// Model table name for subscriptions
    pub subscription_table: String,
    /// Model table name for subscription items
    pub subscription_item_table: String,
    /// Default trial days
    pub trial_days: Option<i32>,
    /// Prorate when changing plans
    pub prorate: bool,
    /// Keep past due subscriptions active
    pub keep_past_due_active: bool,
}

impl Default for CashierConfig {
    fn default() -> Self {
        Self {
            stripe_secret: std::env::var("STRIPE_SECRET").unwrap_or_default(),
            stripe_key: std::env::var("STRIPE_KEY").unwrap_or_default(),
            webhook_secret: std::env::var("STRIPE_WEBHOOK_SECRET").ok(),
            currency: "usd".to_string(),
            subscription_table: "subscriptions".to_string(),
            subscription_item_table: "subscription_items".to_string(),
            trial_days: None,
            prorate: true,
            keep_past_due_active: false,
        }
    }
}

impl CashierConfig {
    /// Create a new configuration
    pub fn new(stripe_secret: &str, stripe_key: &str) -> Self {
        Self {
            stripe_secret: stripe_secret.to_string(),
            stripe_key: stripe_key.to_string(),
            ..Default::default()
        }
    }

    /// Set webhook secret
    pub fn webhook_secret(mut self, secret: &str) -> Self {
        self.webhook_secret = Some(secret.to_string());
        self
    }

    /// Set default currency
    pub fn currency(mut self, currency: &str) -> Self {
        self.currency = currency.to_string();
        self
    }

    /// Set default trial days
    pub fn trial_days(mut self, days: i32) -> Self {
        self.trial_days = Some(days);
        self
    }

    /// Set proration behavior
    pub fn prorate(mut self, prorate: bool) -> Self {
        self.prorate = prorate;
        self
    }
}

/// Set the global configuration
pub fn set_config(config: CashierConfig) -> &'static CashierConfig {
    CONFIG.get_or_init(|| config)
}

/// Get the global configuration
pub fn get_config() -> &'static CashierConfig {
    CONFIG.get_or_init(CashierConfig::default)
}

impl CashierConfig {
    /// Get a Stripe client configured with the secret key
    pub fn stripe_client(&self) -> stripe::Client {
        stripe::Client::new(&self.stripe_secret)
    }
}
