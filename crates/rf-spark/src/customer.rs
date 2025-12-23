//! Customer management

use crate::{SparkError, SparkResult};
use serde::{Deserialize, Serialize};

/// Customer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Customer {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub default_payment_method: Option<String>,
    pub balance: i64,
    pub currency: Option<String>,
    pub created: i64,
}

/// Create or get a Stripe customer
pub async fn create_or_get(
    client: &stripe::Client,
    existing_id: Option<&str>,
    email: &str,
    name: Option<&str>,
) -> SparkResult<Customer> {
    if let Some(id) = existing_id {
        // Get existing customer
        get_customer(client, id).await
    } else {
        // Create new customer
        create_customer(client, email, name).await
    }
}

/// Get a customer by ID
pub async fn get_customer(client: &stripe::Client, customer_id: &str) -> SparkResult<Customer> {
    use stripe::Customer as StripeCustomer;

    let customer = StripeCustomer::retrieve(client, &customer_id.parse().unwrap(), &[])
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(Customer {
        id: customer.id.to_string(),
        email: customer.email.unwrap_or_default(),
        name: customer.name,
        default_payment_method: customer
            .invoice_settings
            .and_then(|s| s.default_payment_method)
            .map(|pm| pm.id().to_string()),
        balance: customer.balance.unwrap_or(0),
        currency: customer.currency.map(|c| c.to_string()),
        created: customer.created.unwrap_or(0),
    })
}

/// Create a new customer
pub async fn create_customer(
    client: &stripe::Client,
    email: &str,
    name: Option<&str>,
) -> SparkResult<Customer> {
    use stripe::{CreateCustomer, Customer as StripeCustomer};

    let mut params = CreateCustomer::new();
    params.email = Some(email);
    params.name = name;

    let customer = StripeCustomer::create(client, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(Customer {
        id: customer.id.to_string(),
        email: customer.email.unwrap_or_default(),
        name: customer.name,
        default_payment_method: None,
        balance: 0,
        currency: None,
        created: customer.created.unwrap_or(0),
    })
}

/// Update customer
pub async fn update_customer(
    client: &stripe::Client,
    customer_id: &str,
    email: Option<&str>,
    name: Option<&str>,
) -> SparkResult<Customer> {
    use stripe::{Customer as StripeCustomer, UpdateCustomer};

    let mut params = UpdateCustomer::default();
    params.email = email;
    params.name = name;

    let customer = StripeCustomer::update(client, &customer_id.parse().unwrap(), params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(Customer {
        id: customer.id.to_string(),
        email: customer.email.unwrap_or_default(),
        name: customer.name,
        default_payment_method: customer
            .invoice_settings
            .and_then(|s| s.default_payment_method)
            .map(|pm| pm.id().to_string()),
        balance: customer.balance.unwrap_or(0),
        currency: customer.currency.map(|c| c.to_string()),
        created: customer.created.unwrap_or(0),
    })
}

/// Delete customer
pub async fn delete_customer(client: &stripe::Client, customer_id: &str) -> SparkResult<()> {
    use stripe::Customer as StripeCustomer;

    StripeCustomer::delete(client, &customer_id.parse().unwrap())
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(())
}

/// Apply a coupon to customer
pub async fn apply_coupon(
    client: &stripe::Client,
    customer_id: &str,
    coupon_code: &str,
) -> SparkResult<()> {
    use stripe::{Customer as StripeCustomer, UpdateCustomer};

    let mut params = UpdateCustomer::default();
    params.coupon = Some(coupon_code.parse().unwrap());

    StripeCustomer::update(client, &customer_id.parse().unwrap(), params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(())
}

/// Create a billing portal session
pub async fn create_billing_portal_session(
    client: &stripe::Client,
    customer_id: &str,
    return_url: &str,
) -> SparkResult<String> {
    use stripe::{BillingPortalSession, CreateBillingPortalSession};

    let mut params = CreateBillingPortalSession::new(customer_id.parse().unwrap());
    params.return_url = Some(return_url);

    let session = BillingPortalSession::create(client, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(session.url)
}

/// Get customer balance
pub async fn get_balance(client: &stripe::Client, customer_id: &str) -> SparkResult<i64> {
    let customer = get_customer(client, customer_id).await?;
    Ok(customer.balance)
}

/// Adjust customer balance
pub async fn adjust_balance(
    client: &stripe::Client,
    customer_id: &str,
    amount: i64,
    _description: Option<&str>,
) -> SparkResult<i64> {
    use stripe::{Customer as StripeCustomer, CreateCustomerBalanceTransaction};

    let params = CreateCustomerBalanceTransaction::new(amount, stripe::Currency::USD);

    let transaction = StripeCustomer::create_balance_transaction(
        client,
        &customer_id.parse().unwrap(),
        params,
    )
    .await
    .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(transaction.ending_balance)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_customer_struct() {
        let customer = Customer {
            id: "cus_123".to_string(),
            email: "test@example.com".to_string(),
            name: Some("Test User".to_string()),
            default_payment_method: None,
            balance: 0,
            currency: Some("usd".to_string()),
            created: 1234567890,
        };

        assert_eq!(customer.id, "cus_123");
        assert_eq!(customer.email, "test@example.com");
    }
}
