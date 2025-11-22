//! Order shipped notification email

use crate::{Address, MailBuilder, Mailable};
use serde::Serialize;

/// Order shipped notification
///
/// # Example
///
/// ```
/// use rf_mail::mailables::OrderShippedMail;
/// use rf_mail::Mailable;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mail = OrderShippedMail {
///     to: "customer@example.com".into(),
///     customer_name: "Alice".into(),
///     order_id: "ORD-12345".into(),
///     tracking_url: "https://tracking.example.com/12345".into(),
/// };
///
/// // Build the mail
/// let builder = mail.build();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct OrderShippedMail {
    /// Recipient email
    pub to: String,

    /// Customer name
    pub customer_name: String,

    /// Order ID
    pub order_id: String,

    /// Tracking URL
    pub tracking_url: String,
}

impl Mailable for OrderShippedMail {
    fn build(&self) -> MailBuilder {
        let markdown = format!(
            r#"# Order Shipped

Hello {customer_name},

Your order #{order_id} has been shipped!

@button({tracking_url})
Track Your Shipment
@endbutton

@panel
**Estimated Delivery:** 3-5 business days

If you have any questions, please don't hesitate to contact us.
@endpanel

Thank you for your order!

---

Best regards,
The Team
"#,
            customer_name = self.customer_name,
            order_id = self.order_id,
            tracking_url = self.tracking_url,
        );

        MailBuilder::new()
            .from(Address::with_name("noreply@example.com", "Online Shop"))
            .to(Address::new(&self.to))
            .subject(format!("Your Order #{} Has Shipped!", self.order_id))
            .markdown(markdown)
    }
}
