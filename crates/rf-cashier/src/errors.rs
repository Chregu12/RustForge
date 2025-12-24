//! Error types for Cashier

use thiserror::Error;

/// Cashier error types
#[derive(Error, Debug)]
pub enum CashierError {
    #[error("Stripe API error: {0}")]
    StripeError(String),

    #[error("Customer not found: {0}")]
    CustomerNotFound(String),

    #[error("Subscription not found: {0}")]
    SubscriptionNotFound(String),

    #[error("Payment method not found: {0}")]
    PaymentMethodNotFound(String),

    #[error("Invalid payment method: {0}")]
    InvalidPaymentMethod(String),

    #[error("Payment failed: {0}")]
    PaymentFailed(String),

    #[error("Subscription already cancelled")]
    AlreadyCancelled,

    #[error("Subscription not on trial")]
    NotOnTrial,

    #[error("Invalid webhook signature")]
    InvalidWebhookSignature,

    #[error("Invalid webhook payload: {0}")]
    InvalidWebhookPayload(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Not billable: customer has no Stripe ID")]
    NotBillable,

    #[error("{0}")]
    Other(String),
}

impl From<stripe::StripeError> for CashierError {
    fn from(err: stripe::StripeError) -> Self {
        CashierError::StripeError(err.to_string())
    }
}

impl From<sea_orm::DbErr> for CashierError {
    fn from(err: sea_orm::DbErr) -> Self {
        CashierError::DatabaseError(err.to_string())
    }
}

/// Result type for Cashier operations
pub type CashierResult<T> = Result<T, CashierError>;
