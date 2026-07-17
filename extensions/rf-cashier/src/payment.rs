//! Payment method management

use crate::errors::{CashierError, CashierResult};
use serde::{Deserialize, Serialize};

/// Payment method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub id: String,
    pub type_: String,
    pub card: Option<CardDetails>,
}

/// Card details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetails {
    pub brand: String,
    pub last4: String,
    pub exp_month: u32,
    pub exp_year: u32,
}

impl From<stripe::PaymentMethod> for PaymentMethod {
    fn from(pm: stripe::PaymentMethod) -> Self {
        let card = pm.card.map(|c| CardDetails {
            brand: c.brand,
            last4: c.last4,
            exp_month: c.exp_month as u32,
            exp_year: c.exp_year as u32,
        });

        Self {
            id: pm.id.to_string(),
            type_: format!("{:?}", pm.type_),
            card,
        }
    }
}

/// Builder for payment method operations
pub struct PaymentMethodBuilder {
    #[allow(dead_code)]
    billable_id: i64,
    payment_method_id: String,
    set_as_default: bool,
}

impl PaymentMethodBuilder {
    /// Create a new payment method builder
    pub fn new(billable_id: i64, payment_method_id: &str) -> Self {
        Self {
            billable_id,
            payment_method_id: payment_method_id.to_string(),
            set_as_default: false,
        }
    }

    /// Set as default payment method
    pub fn as_default(mut self) -> Self {
        self.set_as_default = true;
        self
    }

    /// Attach the payment method to customer
    pub async fn attach(self, customer_id: &str) -> CashierResult<PaymentMethod> {
        let client = crate::config::get_config().stripe_client();

        let params = stripe::AttachPaymentMethod {
            customer: customer_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?,
        };

        let pm = stripe::PaymentMethod::attach(
            &client,
            &self.payment_method_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?,
            params,
        )
        .await
        .map_err(|e| CashierError::StripeError(e.to_string()))?;

        if self.set_as_default {
            let mut update_params = stripe::UpdateCustomer::new();
            update_params.invoice_settings = Some(stripe::CustomerInvoiceSettings {
                default_payment_method: Some(self.payment_method_id),
                ..Default::default()
            });

            stripe::Customer::update(&client, &customer_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?, update_params)
                .await
                .map_err(|e| CashierError::StripeError(e.to_string()))?;
        }

        Ok(PaymentMethod::from(pm))
    }
}
