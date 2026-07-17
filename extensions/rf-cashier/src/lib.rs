//! # rf-cashier - Laravel Cashier-style Stripe Integration
//!
//! Subscription billing and payment processing with Stripe, inspired by Laravel Cashier.
//!
//! ## Features
//!
//! - **Subscriptions**: Create, update, cancel, and resume subscriptions
//! - **One-time Charges**: Process single payments
//! - **Payment Methods**: Manage customer payment methods
//! - **Invoices**: Generate and retrieve invoices
//! - **Checkout Sessions**: Create Stripe Checkout sessions
//! - **Customer Portal**: Redirect to Stripe billing portal
//! - **Webhooks**: Handle Stripe webhooks
//! - **Trial Periods**: Support for subscription trials
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use rf_cashier::{Billable, Cashier};
//!
//! // Implement Billable for your User model
//! #[derive(Billable)]
//! struct User {
//!     id: i64,
//!     email: String,
//!     stripe_id: Option<String>,
//!     pm_type: Option<String>,
//!     pm_last_four: Option<String>,
//!     trial_ends_at: Option<DateTime<Utc>>,
//! }
//!
//! // Create a subscription
//! let subscription = user.new_subscription("default", "price_xxx")
//!     .create("pm_xxx")
//!     .await?;
//!
//! // Check subscription status
//! if user.subscribed("default").await? {
//!     println!("User is subscribed!");
//! }
//!
//! // Cancel subscription
//! user.subscription("default").await?
//!     .cancel()
//!     .await?;
//! ```
//!
//! ## Webhook Handling
//!
//! ```rust,ignore
//! use rf_cashier::webhook_handler;
//! use axum::Router;
//!
//! let app = Router::new()
//!     .route("/stripe/webhook", post(webhook_handler));
//! ```

pub mod billable;
pub mod checkout;
pub mod config;
pub mod customer;
pub mod errors;
pub mod invoice;
pub mod models;
pub mod payment;
pub mod portal;
pub mod subscription;
pub mod webhook;

pub use billable::{Billable, BillableExt};
pub use checkout::{CheckoutBuilder, CheckoutSession};
pub use config::CashierConfig;
pub use customer::CustomerBuilder;
pub use errors::{CashierError, CashierResult};
pub use invoice::{Invoice, InvoiceBuilder};
pub use payment::{PaymentMethod, PaymentMethodBuilder};
pub use portal::PortalSession;
pub use subscription::{Subscription, SubscriptionBuilder, SubscriptionStatus};
pub use webhook::{webhook_handler, WebhookEvent, WebhookPayload};

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        Billable, BillableExt, CashierConfig, CashierError, CashierResult,
        CheckoutBuilder, CheckoutSession, CustomerBuilder, Invoice, InvoiceBuilder,
        PaymentMethod, PaymentMethodBuilder, PortalSession, Subscription,
        SubscriptionBuilder, SubscriptionStatus, WebhookEvent, WebhookPayload,
        webhook_handler,
    };
}

/// Cashier facade for static configuration access
pub struct Cashier;

impl Cashier {
    /// Initialize Cashier with configuration
    pub fn configure(config: CashierConfig) -> &'static CashierConfig {
        config::set_config(config)
    }

    /// Get current configuration
    pub fn config() -> &'static CashierConfig {
        config::get_config()
    }

    /// Get the Stripe secret key
    pub fn stripe_key() -> &'static str {
        &Cashier::config().stripe_secret
    }

    /// Get webhook handler for Axum
    pub fn webhook() -> axum::routing::MethodRouter {
        axum::routing::post(webhook_handler)
    }
}

// No unit-testable pure logic in this crate; billing flows require a payment
// gateway stub and are covered by integration tests.
