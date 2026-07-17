//! Customer management

use crate::errors::{CashierError, CashierResult};

/// Builder for customer operations
pub struct CustomerBuilder {
    #[allow(dead_code)]
    billable_id: i64,
    name: Option<String>,
    email: Option<String>,
    metadata: std::collections::HashMap<String, String>,
}

impl CustomerBuilder {
    /// Create a new customer builder
    pub fn new(billable_id: i64) -> Self {
        Self {
            billable_id,
            name: None,
            email: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set customer name
    pub fn name(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// Set customer email
    pub fn email(mut self, email: &str) -> Self {
        self.email = Some(email.to_string());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Create the customer
    pub async fn create(self) -> CashierResult<stripe::Customer> {
        let client = crate::config::get_config().stripe_client();

        let mut params = stripe::CreateCustomer::new();
        params.email = self.email.as_deref();
        params.name = self.name.as_deref();

        stripe::Customer::create(&client, params)
            .await
            .map_err(|e| CashierError::StripeError(e.to_string()))
    }
}
