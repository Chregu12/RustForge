//! Billable trait for models that can be charged

use crate::checkout::CheckoutBuilder;
use crate::errors::{CashierError, CashierResult};
use crate::invoice::InvoiceBuilder;
use crate::payment::PaymentMethodBuilder;
use crate::portal::PortalSession;
use crate::subscription::{Subscription, SubscriptionBuilder};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Trait for models that can be billed (like User)
#[async_trait]
pub trait Billable: Send + Sync {
    /// Get the billable's unique identifier
    fn billable_id(&self) -> i64;

    /// Get the billable's email
    fn billable_email(&self) -> &str;

    /// Get the billable's name (optional)
    fn billable_name(&self) -> Option<&str> {
        None
    }

    /// Get the Stripe customer ID
    fn stripe_id(&self) -> Option<&str>;

    /// Set the Stripe customer ID
    async fn set_stripe_id(&mut self, stripe_id: &str) -> CashierResult<()>;

    /// Get the default payment method type
    fn pm_type(&self) -> Option<&str>;

    /// Get the last four digits of the default payment method
    fn pm_last_four(&self) -> Option<&str>;

    /// Get the trial end date
    fn trial_ends_at(&self) -> Option<DateTime<Utc>>;

    /// Check if the billable has a Stripe customer ID
    fn has_stripe_id(&self) -> bool {
        self.stripe_id().is_some()
    }

    /// Check if on trial
    fn on_trial(&self) -> bool {
        if let Some(trial_ends) = self.trial_ends_at() {
            trial_ends > Utc::now()
        } else {
            false
        }
    }

    /// Check if on a generic trial (not subscription-specific)
    fn on_generic_trial(&self) -> bool {
        self.on_trial()
    }
}

/// Extension trait with Stripe operations
#[async_trait]
pub trait BillableExt: Billable {
    /// Create or get the Stripe customer
    async fn create_or_get_stripe_customer(&mut self) -> CashierResult<stripe::Customer> {
        if let Some(stripe_id) = self.stripe_id() {
            // Get existing customer
            let client = crate::config::get_config().stripe_client();
            let customer = stripe::Customer::retrieve(&client, &stripe_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?, &[])
                .await
                .map_err(|e| CashierError::StripeError(e.to_string()))?;
            Ok(customer)
        } else {
            // Create new customer
            self.create_as_stripe_customer().await
        }
    }

    /// Create a new Stripe customer
    async fn create_as_stripe_customer(&mut self) -> CashierResult<stripe::Customer> {
        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::CreateCustomer::new();
        params.email = Some(self.billable_email());
        if let Some(name) = self.billable_name() {
            params.name = Some(name);
        }

        let customer = stripe::Customer::create(&client, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        self.set_stripe_id(customer.id.as_ref()).await?;

        Ok(customer)
    }

    /// Get the Stripe customer
    async fn as_stripe_customer(&self) -> CashierResult<stripe::Customer> {
        let stripe_id = self.stripe_id().ok_or(CashierError::NotBillable)?;
        let client = crate::config::get_config().stripe_client();

        stripe::Customer::retrieve(&client, &stripe_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?, &[])
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))
    }

    /// Start building a new subscription
    fn new_subscription(&self, name: &str, price: &str) -> SubscriptionBuilder {
        SubscriptionBuilder::new(self.billable_id(), name, price)
    }

    /// Get a subscription by name
    async fn subscription(&self, name: &str) -> CashierResult<Option<Subscription>> {
        Subscription::find_by_name(self.billable_id(), name).await
    }

    /// Check if subscribed to a specific plan
    async fn subscribed(&self, name: &str) -> CashierResult<bool> {
        if let Some(sub) = self.subscription(name).await? {
            Ok(sub.is_active())
        } else {
            Ok(false)
        }
    }

    /// Check if subscribed to any plan
    async fn subscribed_to_any(&self) -> CashierResult<bool> {
        let subs = Subscription::find_all(self.billable_id()).await?;
        Ok(subs.iter().any(|s| s.is_active()))
    }

    /// Create a one-time charge
    async fn charge(&self, amount: i64, payment_method: &str) -> CashierResult<stripe::PaymentIntent> {
        let stripe_id = self.stripe_id().ok_or(CashierError::NotBillable)?;
        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::CreatePaymentIntent::new(amount, stripe::Currency::USD);
        params.customer = Some(stripe_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?);
        params.payment_method = Some(payment_method.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?);
        params.confirm = Some(true);
        params.off_session = Some(stripe::PaymentIntentOffSession::exists(true));

        stripe::PaymentIntent::create(&client, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))
    }

    /// Create an invoice for a one-time amount
    fn invoice_for(&self, amount: i64, description: &str) -> InvoiceBuilder {
        InvoiceBuilder::new(self.billable_id(), amount, description)
    }

    /// Start a checkout session
    fn checkout(&self, price: &str) -> CheckoutBuilder {
        CheckoutBuilder::new(self.billable_id(), price)
    }

    /// Get billing portal URL
    async fn billing_portal_url(&self, return_url: &str) -> CashierResult<String> {
        let stripe_id = self.stripe_id().ok_or(CashierError::NotBillable)?;
        let session = PortalSession::create(stripe_id, return_url).await?;
        Ok(session.url)
    }

    /// Add a payment method
    fn add_payment_method(&self, payment_method_id: &str) -> PaymentMethodBuilder {
        PaymentMethodBuilder::new(self.billable_id(), payment_method_id)
    }

    /// Get all payment methods
    async fn payment_methods(&self) -> CashierResult<Vec<stripe::PaymentMethod>> {
        let stripe_id = self.stripe_id().ok_or(CashierError::NotBillable)?;
        let client = crate::config::get_config().stripe_client();

        let params = stripe::ListPaymentMethods {
            customer: Some(stripe_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?),
            type_: Some(stripe::PaymentMethodTypeFilter::Card),
            ..Default::default()
        };

        let methods = stripe::PaymentMethod::list(&client, &params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        Ok(methods.data)
    }

    /// Set the default payment method
    async fn update_default_payment_method(&mut self, payment_method_id: &str) -> CashierResult<()> {
        let stripe_id = self.stripe_id().ok_or(CashierError::NotBillable)?;
        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::UpdateCustomer::new();
        params.invoice_settings = Some(stripe::CustomerInvoiceSettings {
            default_payment_method: Some(payment_method_id.to_string()),
            ..Default::default()
        });

        stripe::Customer::update(&client, &stripe_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        Ok(())
    }
}

// Blanket implementation for all Billable types
impl<T: Billable> BillableExt for T {}
