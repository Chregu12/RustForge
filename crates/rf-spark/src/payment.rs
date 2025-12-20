//! Payment method and charge management

use crate::{SparkError, SparkResult};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Payment method information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentMethod {
    pub id: String,
    pub pm_type: PaymentMethodType,
    pub card: Option<CardDetails>,
    pub is_default: bool,
}

/// Payment method type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentMethodType {
    Card,
    BankAccount,
    Sepa,
    Other,
}

/// Card details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDetails {
    pub brand: String,
    pub last4: String,
    pub exp_month: u32,
    pub exp_year: u32,
}

/// Payment intent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaymentIntent {
    pub id: String,
    pub amount: Decimal,
    pub currency: String,
    pub status: PaymentStatus,
    pub client_secret: Option<String>,
}

/// Payment status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaymentStatus {
    RequiresPaymentMethod,
    RequiresConfirmation,
    RequiresAction,
    Processing,
    RequiresCapture,
    Canceled,
    Succeeded,
}

impl From<stripe::PaymentIntentStatus> for PaymentStatus {
    fn from(status: stripe::PaymentIntentStatus) -> Self {
        match status {
            stripe::PaymentIntentStatus::RequiresPaymentMethod => {
                PaymentStatus::RequiresPaymentMethod
            }
            stripe::PaymentIntentStatus::RequiresConfirmation => {
                PaymentStatus::RequiresConfirmation
            }
            stripe::PaymentIntentStatus::RequiresAction => PaymentStatus::RequiresAction,
            stripe::PaymentIntentStatus::Processing => PaymentStatus::Processing,
            stripe::PaymentIntentStatus::RequiresCapture => PaymentStatus::RequiresCapture,
            stripe::PaymentIntentStatus::Canceled => PaymentStatus::Canceled,
            stripe::PaymentIntentStatus::Succeeded => PaymentStatus::Succeeded,
        }
    }
}

/// Add a payment method to customer
pub async fn add_payment_method(
    client: &stripe::Client,
    customer_id: &str,
    payment_method_id: &str,
) -> SparkResult<PaymentMethod> {
    use stripe::{AttachPaymentMethod, PaymentMethod as StripePM};

    let params = AttachPaymentMethod {
        customer: customer_id.parse().unwrap(),
    };

    let pm = StripePM::attach(client, &payment_method_id.parse().unwrap(), params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(convert_payment_method(pm, false))
}

/// Get default payment method
pub async fn get_default_payment_method(
    client: &stripe::Client,
    customer_id: &str,
) -> SparkResult<Option<PaymentMethod>> {
    use stripe::Customer;

    let customer = Customer::retrieve(client, &customer_id.parse().unwrap(), &[])
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    if let Some(pm) = customer.invoice_settings.and_then(|s| s.default_payment_method) {
        let pm_id = pm.id();
        let pm = stripe::PaymentMethod::retrieve(client, pm_id, &[])
            .await
            .map_err(|e| SparkError::StripeError(e.to_string()))?;
        Ok(Some(convert_payment_method(pm, true)))
    } else {
        Ok(None)
    }
}

/// Set default payment method
pub async fn set_default_payment_method(
    client: &stripe::Client,
    customer_id: &str,
    payment_method_id: &str,
) -> SparkResult<()> {
    use stripe::{Customer, UpdateCustomer, UpdateCustomerInvoiceSettings};

    let mut invoice_settings = UpdateCustomerInvoiceSettings::default();
    invoice_settings.default_payment_method = Some(payment_method_id);

    let mut params = UpdateCustomer::default();
    params.invoice_settings = Some(invoice_settings);

    Customer::update(client, &customer_id.parse().unwrap(), params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(())
}

/// List payment methods
pub async fn list_payment_methods(
    client: &stripe::Client,
    customer_id: &str,
) -> SparkResult<Vec<PaymentMethod>> {
    use stripe::{ListPaymentMethods, PaymentMethod as StripePM};

    let mut params = ListPaymentMethods::new();
    params.customer = Some(customer_id.parse().unwrap());
    params.type_ = Some(stripe::PaymentMethodTypeFilter::Card);

    let pms = StripePM::list(client, &params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    // Get default payment method
    let default_id = get_default_payment_method(client, customer_id)
        .await?
        .map(|pm| pm.id);

    Ok(pms
        .data
        .into_iter()
        .map(|pm| {
            let is_default = default_id.as_ref() == Some(&pm.id.to_string());
            convert_payment_method(pm, is_default)
        })
        .collect())
}

/// Delete a payment method
pub async fn delete_payment_method(
    client: &stripe::Client,
    payment_method_id: &str,
) -> SparkResult<()> {
    use stripe::PaymentMethod as StripePM;

    StripePM::detach(client, &payment_method_id.parse().unwrap())
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(())
}

/// Create a charge (PaymentIntent)
pub async fn create_charge(
    client: &stripe::Client,
    customer_id: &str,
    amount: Decimal,
    currency: &str,
    description: &str,
) -> SparkResult<PaymentIntent> {
    use stripe::{CreatePaymentIntent, PaymentIntent as StripePI};

    // Convert to cents
    let amount_cents = (amount * Decimal::from(100)).to_string().parse::<i64>().unwrap_or(0);

    let mut params = CreatePaymentIntent::new(amount_cents, currency.parse().unwrap());
    params.customer = Some(customer_id.parse().unwrap());
    params.description = Some(description);
    params.confirm = Some(true);

    let pi = StripePI::create(client, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(PaymentIntent {
        id: pi.id.to_string(),
        amount,
        currency: currency.to_string(),
        status: pi.status.into(),
        client_secret: pi.client_secret,
    })
}

/// Create a refund
pub async fn create_refund(
    client: &stripe::Client,
    payment_intent_id: &str,
    amount: Option<Decimal>,
) -> SparkResult<()> {
    use stripe::{CreateRefund, Refund};

    let mut params = CreateRefund::default();
    params.payment_intent = Some(payment_intent_id.parse().unwrap());

    if let Some(amt) = amount {
        let amount_cents = (amt * Decimal::from(100)).to_string().parse::<i64>().unwrap_or(0);
        params.amount = Some(amount_cents);
    }

    Refund::create(client, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(())
}

/// Create a setup intent for future payments
pub async fn create_setup_intent(
    client: &stripe::Client,
    customer_id: &str,
) -> SparkResult<String> {
    use stripe::{CreateSetupIntent, SetupIntent};

    let mut params = CreateSetupIntent::default();
    params.customer = Some(customer_id.parse().unwrap());
    params.usage = Some(stripe::SetupIntentUsage::OffSession);

    let intent = SetupIntent::create(client, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(intent.client_secret.unwrap_or_default())
}

/// Convert Stripe payment method to our type
fn convert_payment_method(pm: stripe::PaymentMethod, is_default: bool) -> PaymentMethod {
    let pm_type = match pm.type_ {
        stripe::PaymentMethodType::Card => PaymentMethodType::Card,
        stripe::PaymentMethodType::SepaDebit => PaymentMethodType::Sepa,
        stripe::PaymentMethodType::UsBankAccount => PaymentMethodType::BankAccount,
        _ => PaymentMethodType::Other,
    };

    let card = pm.card.map(|c| CardDetails {
        brand: c.brand.unwrap_or_else(|| "unknown".to_string()),
        last4: c.last4.unwrap_or_else(|| "****".to_string()),
        exp_month: c.exp_month as u32,
        exp_year: c.exp_year as u32,
    });

    PaymentMethod {
        id: pm.id.to_string(),
        pm_type,
        card,
        is_default,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_payment_status() {
        assert_eq!(
            PaymentStatus::Succeeded,
            stripe::PaymentIntentStatus::Succeeded.into()
        );
    }

    #[test]
    fn test_card_details() {
        let card = CardDetails {
            brand: "visa".to_string(),
            last4: "4242".to_string(),
            exp_month: 12,
            exp_year: 2025,
        };

        assert_eq!(card.brand, "visa");
        assert_eq!(card.last4, "4242");
    }
}
