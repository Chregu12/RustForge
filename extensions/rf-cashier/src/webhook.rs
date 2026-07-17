//! Stripe webhook handling

use crate::errors::{CashierError, CashierResult};
use axum::{
    body::Bytes,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};

/// Webhook event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookEvent {
    CustomerSubscriptionCreated,
    CustomerSubscriptionUpdated,
    CustomerSubscriptionDeleted,
    CustomerSubscriptionTrialWillEnd,
    InvoicePaid,
    InvoicePaymentFailed,
    InvoicePaymentActionRequired,
    PaymentIntentSucceeded,
    PaymentIntentPaymentFailed,
    CustomerUpdated,
    CustomerDeleted,
    PaymentMethodAttached,
    PaymentMethodDetached,
    Other(String),
}

impl From<&str> for WebhookEvent {
    fn from(s: &str) -> Self {
        match s {
            "customer.subscription.created" => WebhookEvent::CustomerSubscriptionCreated,
            "customer.subscription.updated" => WebhookEvent::CustomerSubscriptionUpdated,
            "customer.subscription.deleted" => WebhookEvent::CustomerSubscriptionDeleted,
            "customer.subscription.trial_will_end" => WebhookEvent::CustomerSubscriptionTrialWillEnd,
            "invoice.paid" => WebhookEvent::InvoicePaid,
            "invoice.payment_failed" => WebhookEvent::InvoicePaymentFailed,
            "invoice.payment_action_required" => WebhookEvent::InvoicePaymentActionRequired,
            "payment_intent.succeeded" => WebhookEvent::PaymentIntentSucceeded,
            "payment_intent.payment_failed" => WebhookEvent::PaymentIntentPaymentFailed,
            "customer.updated" => WebhookEvent::CustomerUpdated,
            "customer.deleted" => WebhookEvent::CustomerDeleted,
            "payment_method.attached" => WebhookEvent::PaymentMethodAttached,
            "payment_method.detached" => WebhookEvent::PaymentMethodDetached,
            other => WebhookEvent::Other(other.to_string()),
        }
    }
}

/// Webhook payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPayload {
    pub id: String,
    pub event_type: WebhookEvent,
    pub data: serde_json::Value,
}

/// Webhook handler for Axum
pub async fn webhook_handler(
    headers: HeaderMap,
    body: Bytes,
) -> impl IntoResponse {
    let config = crate::config::get_config();

    // Get signature from header
    let signature = headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    // Verify signature if webhook secret is configured
    if let Some(ref webhook_secret) = config.webhook_secret {
        if let Err(e) = verify_signature(&body, signature, webhook_secret) {
            return (StatusCode::BAD_REQUEST, format!("Invalid signature: {}", e));
        }
    }

    // Parse the event
    let event: stripe::Event = match serde_json::from_slice(&body) {
        Ok(e) => e,
        Err(e) => {
            return (StatusCode::BAD_REQUEST, format!("Invalid payload: {}", e));
        }
    };

    // Handle the event
    let event_type = WebhookEvent::from(format!("{:?}", event.type_).as_str());

    match event_type {
        WebhookEvent::CustomerSubscriptionCreated => {
            // Handle subscription created
        }
        WebhookEvent::CustomerSubscriptionUpdated => {
            // Handle subscription updated
        }
        WebhookEvent::CustomerSubscriptionDeleted => {
            // Handle subscription deleted
        }
        WebhookEvent::InvoicePaid => {
            // Handle invoice paid
        }
        WebhookEvent::InvoicePaymentFailed => {
            // Handle payment failed
        }
        _ => {
            // Handle other events
        }
    }

    (StatusCode::OK, "Webhook received".to_string())
}

/// Verify Stripe webhook signature
fn verify_signature(payload: &[u8], signature: &str, secret: &str) -> CashierResult<()> {
    // Parse signature header
    let parts: std::collections::HashMap<&str, &str> = signature
        .split(',')
        .filter_map(|part| {
            let mut kv = part.split('=');
            Some((kv.next()?, kv.next()?))
        })
        .collect();

    let timestamp = parts.get("t").ok_or(CashierError::InvalidWebhookSignature)?;
    let sig = parts.get("v1").ok_or(CashierError::InvalidWebhookSignature)?;

    // Create signed payload
    let signed_payload = format!("{}.{}", timestamp, String::from_utf8_lossy(payload));

    // Compute expected signature
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| CashierError::InvalidWebhookSignature)?;
    mac.update(signed_payload.as_bytes());

    let expected = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison to prevent timing attacks
    let expected_bytes = expected.as_bytes();
    let sig_bytes = sig.as_bytes();
    if expected_bytes.len() != sig_bytes.len() {
        return Err(CashierError::InvalidWebhookSignature);
    }
    let mut result = 0u8;
    for (a, b) in expected_bytes.iter().zip(sig_bytes.iter()) {
        result |= a ^ b;
    }
    if result != 0 {
        return Err(CashierError::InvalidWebhookSignature);
    }

    Ok(())
}
