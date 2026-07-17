//! Invoice management

use crate::errors::{CashierError, CashierResult};
use serde::{Deserialize, Serialize};

/// Invoice
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub number: Option<String>,
    pub amount_due: i64,
    pub amount_paid: i64,
    pub status: String,
    pub pdf_url: Option<String>,
    pub hosted_invoice_url: Option<String>,
}

impl From<stripe::Invoice> for Invoice {
    fn from(invoice: stripe::Invoice) -> Self {
        Self {
            id: invoice.id.to_string(),
            number: invoice.number,
            amount_due: invoice.amount_due.unwrap_or(0),
            amount_paid: invoice.amount_paid.unwrap_or(0),
            status: invoice.status.map(|s| format!("{:?}", s)).unwrap_or_default(),
            pdf_url: invoice.invoice_pdf,
            hosted_invoice_url: invoice.hosted_invoice_url,
        }
    }
}

/// Builder for creating invoices
pub struct InvoiceBuilder {
    #[allow(dead_code)]
    billable_id: i64,
    amount: i64,
    description: String,
    auto_advance: bool,
    collection_method: CollectionMethod,
}

/// Collection method for invoices
#[derive(Debug, Clone)]
pub enum CollectionMethod {
    ChargeAutomatically,
    SendInvoice,
}

impl InvoiceBuilder {
    /// Create a new invoice builder
    pub fn new(billable_id: i64, amount: i64, description: &str) -> Self {
        Self {
            billable_id,
            amount,
            description: description.to_string(),
            auto_advance: true,
            collection_method: CollectionMethod::ChargeAutomatically,
        }
    }

    /// Set auto advance
    pub fn auto_advance(mut self, auto_advance: bool) -> Self {
        self.auto_advance = auto_advance;
        self
    }

    /// Send invoice instead of charging automatically
    pub fn send_invoice(mut self) -> Self {
        self.collection_method = CollectionMethod::SendInvoice;
        self
    }

    /// Create and send the invoice
    pub async fn create(self, customer_id: &str) -> CashierResult<Invoice> {
        let client = crate::config::get_config().stripe_client();

        // Create invoice item first
        let mut item_params = stripe::CreateInvoiceItem::new(customer_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?);
        item_params.amount = Some(self.amount);
        item_params.currency = Some(stripe::Currency::USD); // Default to USD
        item_params.description = Some(&self.description);

        stripe::InvoiceItem::create(&client, item_params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        // Create invoice
        let collection = match self.collection_method {
            CollectionMethod::ChargeAutomatically => stripe::CollectionMethod::ChargeAutomatically,
            CollectionMethod::SendInvoice => stripe::CollectionMethod::SendInvoice,
        };

        let mut params = stripe::CreateInvoice::new();
        params.customer = Some(customer_id.parse().map_err(|_| CashierError::StripeError("Invalid Stripe ID format".to_string()))?);
        params.auto_advance = Some(self.auto_advance);
        params.collection_method = Some(collection);

        let invoice = stripe::Invoice::create(&client, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))?;

        Ok(Invoice::from(invoice))
    }
}
