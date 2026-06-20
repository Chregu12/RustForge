//! Invoice management

use crate::{SparkError, SparkResult};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Invoice information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invoice {
    pub id: String,
    pub number: Option<String>,
    pub status: InvoiceStatus,
    pub total: Decimal,
    pub subtotal: Decimal,
    pub tax: Option<Decimal>,
    pub currency: String,
    pub customer_email: Option<String>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub due_date: Option<DateTime<Utc>>,
    pub paid_at: Option<DateTime<Utc>>,
    pub hosted_invoice_url: Option<String>,
    pub pdf_url: Option<String>,
    pub lines: Vec<InvoiceLine>,
}

/// Invoice status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvoiceStatus {
    Draft,
    Open,
    Paid,
    Void,
    Uncollectible,
}

impl From<stripe::InvoiceStatus> for InvoiceStatus {
    fn from(status: stripe::InvoiceStatus) -> Self {
        match status {
            stripe::InvoiceStatus::Draft => InvoiceStatus::Draft,
            stripe::InvoiceStatus::Open => InvoiceStatus::Open,
            stripe::InvoiceStatus::Paid => InvoiceStatus::Paid,
            stripe::InvoiceStatus::Void => InvoiceStatus::Void,
            stripe::InvoiceStatus::Uncollectible => InvoiceStatus::Uncollectible,
        }
    }
}

/// Invoice line item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceLine {
    pub id: String,
    pub description: Option<String>,
    pub amount: Decimal,
    pub quantity: i64,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Invoice builder for creating custom invoices
pub struct InvoiceBuilder<'a> {
    client: &'a stripe::Client,
    customer_id: String,
    items: Vec<InvoiceItemParams>,
    auto_advance: bool,
    collection_method: CollectionMethod,
    days_until_due: Option<u32>,
    description: Option<String>,
}

#[derive(Debug, Clone)]
struct InvoiceItemParams {
    price_id: Option<String>,
    amount: Option<i64>,
    description: String,
    quantity: i64,
}

#[derive(Debug, Clone, Copy)]
pub enum CollectionMethod {
    ChargeAutomatically,
    SendInvoice,
}

impl<'a> InvoiceBuilder<'a> {
    pub fn new(client: &'a stripe::Client, customer_id: impl Into<String>) -> Self {
        Self {
            client,
            customer_id: customer_id.into(),
            items: Vec::new(),
            auto_advance: true,
            collection_method: CollectionMethod::ChargeAutomatically,
            days_until_due: None,
            description: None,
        }
    }

    /// Add a line item with a price ID
    pub fn add_item(mut self, price_id: impl Into<String>, quantity: i64) -> Self {
        self.items.push(InvoiceItemParams {
            price_id: Some(price_id.into()),
            amount: None,
            description: String::new(),
            quantity,
        });
        self
    }

    /// Add a custom line item
    pub fn add_custom_item(
        mut self,
        amount: Decimal,
        description: impl Into<String>,
    ) -> SparkResult<Self> {
        let amount_cents = (amount * Decimal::from(100))
            .round_dp(0)
            .to_string()
            .parse::<i64>()
            .map_err(|_| SparkError::InvalidRequest(format!("Invalid invoice amount: {}", amount)))?;
        self.items.push(InvoiceItemParams {
            price_id: None,
            amount: Some(amount_cents),
            description: description.into(),
            quantity: 1,
        });
        Ok(self)
    }

    /// Set auto advance
    pub fn auto_advance(mut self, auto: bool) -> Self {
        self.auto_advance = auto;
        self
    }

    /// Set collection method
    pub fn collection_method(mut self, method: CollectionMethod) -> Self {
        self.collection_method = method;
        self
    }

    /// Set days until due
    pub fn days_until_due(mut self, days: u32) -> Self {
        self.days_until_due = Some(days);
        self
    }

    /// Set description
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Create the invoice
    pub async fn create(self) -> SparkResult<Invoice> {
        use stripe::{CreateInvoice, CreateInvoiceItem, Invoice as StripeInvoice, InvoiceItem};

        // Create invoice items first
        for item in &self.items {
            let mut params = CreateInvoiceItem::new(self.customer_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);

            if let Some(ref price_id) = item.price_id {
                params.price = Some(price_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);
                params.quantity = Some(item.quantity as u64);
            } else if let Some(amount) = item.amount {
                params.amount = Some(amount);
                params.description = Some(&item.description);
                params.currency = Some(stripe::Currency::USD);
            }

            InvoiceItem::create(self.client, params)
                .await
                .map_err(|e| SparkError::StripeError(e.to_string()))?;
        }

        // Create the invoice
        let mut params = CreateInvoice::new();
        params.customer = Some(self.customer_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);
        params.auto_advance = Some(self.auto_advance);
        params.collection_method = Some(match self.collection_method {
            CollectionMethod::ChargeAutomatically => {
                stripe::CollectionMethod::ChargeAutomatically
            }
            CollectionMethod::SendInvoice => stripe::CollectionMethod::SendInvoice,
        });

        if let Some(days) = self.days_until_due {
            params.days_until_due = Some(days);
        }

        if let Some(ref desc) = self.description {
            params.description = Some(desc);
        }

        let invoice = StripeInvoice::create(self.client, params)
            .await
            .map_err(|e| SparkError::StripeError(e.to_string()))?;

        Ok(convert_invoice(invoice))
    }
}

/// List invoices for a customer
pub async fn list_invoices(
    client: &stripe::Client,
    customer_id: &str,
) -> SparkResult<Vec<Invoice>> {
    use stripe::{Invoice as StripeInvoice, ListInvoices};

    let mut params = ListInvoices::default();
    params.customer = Some(customer_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);

    let invoices = StripeInvoice::list(client, &params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(invoices.data.into_iter().map(convert_invoice).collect())
}

/// Get upcoming invoice
pub async fn get_upcoming(
    client: &stripe::Client,
    customer_id: &str,
) -> SparkResult<Option<Invoice>> {
    use stripe::{Invoice as StripeInvoice, RetrieveUpcomingInvoice};

    let params = RetrieveUpcomingInvoice::new(customer_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?);

    match StripeInvoice::upcoming(client, params).await {
        Ok(invoice) => Ok(Some(convert_invoice(invoice))),
        Err(stripe::StripeError::Stripe(ref e)) if e.code == Some(stripe::ErrorCode::ResourceMissing) => {
            Ok(None)
        }
        Err(e) => Err(SparkError::StripeError(e.to_string())),
    }
}

/// Get invoice PDF URL
pub async fn get_pdf_url(
    client: &stripe::Client,
    invoice_id: &str,
) -> SparkResult<String> {
    use stripe::Invoice as StripeInvoice;

    let invoice = StripeInvoice::retrieve(client, &invoice_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?, &[])
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    invoice
        .invoice_pdf
        .ok_or_else(|| SparkError::StripeError("No PDF available".to_string()))
}

/// Pay an invoice
pub async fn pay_invoice(
    client: &stripe::Client,
    invoice_id: &str,
) -> SparkResult<Invoice> {
    use stripe::Invoice as StripeInvoice;

    let invoice = StripeInvoice::pay(client, &invoice_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(convert_invoice(invoice))
}

/// Void an invoice
pub async fn void_invoice(
    client: &stripe::Client,
    invoice_id: &str,
) -> SparkResult<Invoice> {
    use stripe::Invoice as StripeInvoice;

    let invoice = StripeInvoice::void(client, &invoice_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(convert_invoice(invoice))
}

/// Finalize a draft invoice
pub async fn finalize_invoice(
    client: &stripe::Client,
    invoice_id: &str,
) -> SparkResult<Invoice> {
    use stripe::{Invoice as StripeInvoice, FinalizeInvoiceParams};

    let params = FinalizeInvoiceParams::default();
    let invoice = StripeInvoice::finalize(client, &invoice_id.parse().map_err(|_| SparkError::InvalidRequest("Invalid Stripe ID format".into()))?, params)
        .await
        .map_err(|e| SparkError::StripeError(e.to_string()))?;

    Ok(convert_invoice(invoice))
}

/// Convert Stripe invoice to our type
fn convert_invoice(inv: stripe::Invoice) -> Invoice {
    let lines: Vec<InvoiceLine> = inv
        .lines
        .as_ref()
        .map(|l| {
            l.data
                .iter()
                .map(|line| InvoiceLine {
                    id: line.id.to_string(),
                    description: line.description.clone(),
                    amount: Decimal::from(line.amount) / Decimal::from(100),
                    quantity: line.quantity.unwrap_or(1) as i64,
                    period_start: DateTime::from_timestamp(
                        line.period.as_ref().and_then(|p| p.start).unwrap_or(0),
                        0,
                    )
                    .unwrap_or_else(Utc::now),
                    period_end: DateTime::from_timestamp(
                        line.period.as_ref().and_then(|p| p.end).unwrap_or(0),
                        0,
                    )
                    .unwrap_or_else(Utc::now),
                })
                .collect()
        })
        .unwrap_or_default();

    Invoice {
        id: inv.id.as_str().to_string(),
        number: inv.number,
        status: inv.status.map(|s| s.into()).unwrap_or(InvoiceStatus::Draft),
        total: Decimal::from(inv.total.unwrap_or(0)) / Decimal::from(100),
        subtotal: Decimal::from(inv.subtotal.unwrap_or(0)) / Decimal::from(100),
        tax: inv.tax.map(|t| Decimal::from(t) / Decimal::from(100)),
        currency: inv.currency.map(|c| c.to_string()).unwrap_or_else(|| "usd".to_string()),
        customer_email: inv.customer_email,
        period_start: DateTime::from_timestamp(inv.period_start.unwrap_or(0), 0)
            .unwrap_or_else(Utc::now),
        period_end: DateTime::from_timestamp(inv.period_end.unwrap_or(0), 0)
            .unwrap_or_else(Utc::now),
        due_date: inv.due_date.and_then(|d| DateTime::from_timestamp(d, 0)),
        paid_at: inv
            .status_transitions
            .and_then(|st| st.paid_at)
            .and_then(|ts| DateTime::from_timestamp(ts, 0)),
        hosted_invoice_url: inv.hosted_invoice_url,
        pdf_url: inv.invoice_pdf,
        lines,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invoice_status() {
        assert_eq!(
            InvoiceStatus::Paid,
            stripe::InvoiceStatus::Paid.into()
        );
    }

    #[test]
    fn test_invoice_line() {
        let line = InvoiceLine {
            id: "il_123".to_string(),
            description: Some("Monthly subscription".to_string()),
            amount: Decimal::from(1000) / Decimal::from(100),
            quantity: 1,
            period_start: Utc::now(),
            period_end: Utc::now(),
        };

        assert_eq!(line.quantity, 1);
    }
}
