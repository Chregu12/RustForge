//! Stripe webhook handling

use crate::{SparkError, SparkResult};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};

/// Webhook event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookEvent {
    pub id: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub created: i64,
}

/// Verify webhook signature
pub fn verify_signature(
    payload: &str,
    signature: &str,
    secret: &str,
) -> SparkResult<WebhookEvent> {
    // Parse the signature header
    let parts: std::collections::HashMap<String, String> = signature
        .split(',')
        .filter_map(|part| {
            let mut split = part.split('=');
            Some((split.next()?.to_string(), split.next()?.to_string()))
        })
        .collect();

    let timestamp = parts
        .get("t")
        .ok_or_else(|| SparkError::WebhookError("Missing timestamp".to_string()))?;

    let signature_v1 = parts
        .get("v1")
        .ok_or_else(|| SparkError::WebhookError("Missing v1 signature".to_string()))?;

    // Compute expected signature
    let signed_payload = format!("{}.{}", timestamp, payload);

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| SparkError::WebhookError(e.to_string()))?;
    mac.update(signed_payload.as_bytes());

    let expected = hex::encode(mac.finalize().into_bytes());

    // Constant-time comparison
    if !constant_time_compare(&expected, signature_v1) {
        return Err(SparkError::WebhookError(
            "Invalid signature".to_string(),
        ));
    }

    // Parse the payload
    let event: StripeWebhookEvent = serde_json::from_str(payload)
        .map_err(|e| SparkError::WebhookError(e.to_string()))?;

    Ok(WebhookEvent {
        id: event.id,
        event_type: event.type_field,
        data: event.data.object,
        created: event.created,
    })
}

/// Constant-time string comparison
fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}

#[derive(Deserialize)]
struct StripeWebhookEvent {
    id: String,
    #[serde(rename = "type")]
    type_field: String,
    data: StripeEventData,
    created: i64,
}

#[derive(Deserialize)]
struct StripeEventData {
    object: serde_json::Value,
}

/// Webhook handler
pub struct WebhookHandler {
    secret: String,
    handlers: std::collections::HashMap<String, Box<dyn Fn(&WebhookEvent) + Send + Sync>>,
}

impl WebhookHandler {
    pub fn new(secret: String) -> Self {
        Self {
            secret,
            handlers: std::collections::HashMap::new(),
        }
    }

    /// Register a handler for an event type
    pub fn on<F>(mut self, event_type: &str, handler: F) -> Self
    where
        F: Fn(&WebhookEvent) + Send + Sync + 'static,
    {
        self.handlers
            .insert(event_type.to_string(), Box::new(handler));
        self
    }

    /// Handle a webhook request
    pub fn handle(&self, payload: &str, signature: &str) -> SparkResult<()> {
        let event = verify_signature(payload, signature, &self.secret)?;

        if let Some(handler) = self.handlers.get(&event.event_type) {
            handler(&event);
        }

        Ok(())
    }

    /// Get all registered event types
    pub fn registered_events(&self) -> Vec<&str> {
        self.handlers.keys().map(|s| s.as_str()).collect()
    }
}

/// Common Stripe webhook event types
pub mod events {
    pub const CUSTOMER_CREATED: &str = "customer.created";
    pub const CUSTOMER_UPDATED: &str = "customer.updated";
    pub const CUSTOMER_DELETED: &str = "customer.deleted";

    pub const SUBSCRIPTION_CREATED: &str = "customer.subscription.created";
    pub const SUBSCRIPTION_UPDATED: &str = "customer.subscription.updated";
    pub const SUBSCRIPTION_DELETED: &str = "customer.subscription.deleted";
    pub const SUBSCRIPTION_TRIAL_WILL_END: &str = "customer.subscription.trial_will_end";

    pub const INVOICE_CREATED: &str = "invoice.created";
    pub const INVOICE_PAID: &str = "invoice.paid";
    pub const INVOICE_PAYMENT_FAILED: &str = "invoice.payment_failed";
    pub const INVOICE_FINALIZED: &str = "invoice.finalized";

    pub const PAYMENT_INTENT_SUCCEEDED: &str = "payment_intent.succeeded";
    pub const PAYMENT_INTENT_FAILED: &str = "payment_intent.payment_failed";

    pub const CHARGE_SUCCEEDED: &str = "charge.succeeded";
    pub const CHARGE_FAILED: &str = "charge.failed";
    pub const CHARGE_REFUNDED: &str = "charge.refunded";

    pub const PAYMENT_METHOD_ATTACHED: &str = "payment_method.attached";
    pub const PAYMENT_METHOD_DETACHED: &str = "payment_method.detached";
}

/// Extract subscription from webhook event
pub fn extract_subscription(event: &WebhookEvent) -> Option<SubscriptionWebhookData> {
    serde_json::from_value(event.data.clone()).ok()
}

/// Subscription webhook data
#[derive(Debug, Clone, Deserialize)]
pub struct SubscriptionWebhookData {
    pub id: String,
    pub customer: String,
    pub status: String,
    pub current_period_start: i64,
    pub current_period_end: i64,
    pub cancel_at_period_end: bool,
    pub canceled_at: Option<i64>,
    pub trial_start: Option<i64>,
    pub trial_end: Option<i64>,
}

/// Extract invoice from webhook event
pub fn extract_invoice(event: &WebhookEvent) -> Option<InvoiceWebhookData> {
    serde_json::from_value(event.data.clone()).ok()
}

/// Invoice webhook data
#[derive(Debug, Clone, Deserialize)]
pub struct InvoiceWebhookData {
    pub id: String,
    pub customer: String,
    pub status: String,
    pub total: i64,
    pub currency: String,
    pub paid: bool,
    pub hosted_invoice_url: Option<String>,
    pub invoice_pdf: Option<String>,
}

/// Extract payment intent from webhook event
pub fn extract_payment_intent(event: &WebhookEvent) -> Option<PaymentIntentWebhookData> {
    serde_json::from_value(event.data.clone()).ok()
}

/// Payment intent webhook data
#[derive(Debug, Clone, Deserialize)]
pub struct PaymentIntentWebhookData {
    pub id: String,
    pub customer: Option<String>,
    pub status: String,
    pub amount: i64,
    pub currency: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_compare() {
        assert!(constant_time_compare("hello", "hello"));
        assert!(!constant_time_compare("hello", "world"));
        assert!(!constant_time_compare("hello", "hell"));
    }

    #[test]
    fn test_webhook_handler() {
        let handler = WebhookHandler::new("secret".to_string())
            .on(events::INVOICE_PAID, |_event| {
                println!("Invoice paid!");
            });

        assert!(handler.registered_events().contains(&events::INVOICE_PAID));
    }
}
