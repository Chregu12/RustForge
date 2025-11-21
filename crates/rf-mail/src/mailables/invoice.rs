//! Invoice email with attachment

use crate::{Address, Mailable, MailBuilder};
use serde::Serialize;
use std::path::PathBuf;

/// Invoice email with PDF attachment
///
/// # Example
///
/// ```no_run
/// use rf_mail::mailables::InvoiceMail;
/// use rf_mail::Mailable;
/// use std::path::PathBuf;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mail = InvoiceMail {
///     to: "customer@example.com".into(),
///     customer_name: "Alice".into(),
///     invoice_number: "INV-2024-001".into(),
///     amount: 299.99,
///     pdf_path: PathBuf::from("/tmp/invoice.pdf"),
/// };
///
/// let builder = mail.build();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct InvoiceMail {
    /// Recipient email
    pub to: String,

    /// Customer name
    pub customer_name: String,

    /// Invoice number
    pub invoice_number: String,

    /// Invoice amount
    pub amount: f64,

    /// Path to PDF invoice
    #[serde(skip)]
    pub pdf_path: PathBuf,
}

impl Mailable for InvoiceMail {
    fn build(&self) -> MailBuilder {
        let markdown = format!(
            r#"# Invoice {invoice_number}

Dear {customer_name},

Thank you for your business! Please find your invoice attached.

@panel
**Invoice Number:** {invoice_number}
**Amount:** ${amount:.2}
**Date:** {date}
@endpanel

Please remit payment within 30 days.

If you have any questions about this invoice, please contact our billing department.

---

Best regards,
Accounts Receivable
"#,
            invoice_number = self.invoice_number,
            customer_name = self.customer_name,
            amount = self.amount,
            date = chrono::Utc::now().format("%B %d, %Y"),
        );

        let builder = MailBuilder::new()
            .from(Address::with_name("billing@example.com", "Billing Department"))
            .to(Address::new(&self.to))
            .subject(format!("Invoice {} - ${:.2}", self.invoice_number, self.amount))
            .markdown(markdown.clone());

        // Attach PDF if path is provided and file exists
        if self.pdf_path.exists() {
            builder.attach(&self.pdf_path).unwrap_or_else(|_| {
                MailBuilder::new()
                    .from(Address::with_name("billing@example.com", "Billing Department"))
                    .to(Address::new(&self.to))
                    .subject(format!("Invoice {} - ${:.2}", self.invoice_number, self.amount))
                    .markdown(markdown)
            })
        } else {
            builder
        }
    }
}
