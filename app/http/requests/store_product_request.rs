use serde::Deserialize;

/// StoreProduct Request
///
/// Validates and deserializes incoming product store requests.
/// Add custom fields as needed for your application.
#[derive(Debug, Deserialize)]
pub struct StoreProduct {
    /// Product name
    pub name: String,
    /// Product description
    pub description: Option<String>,
    /// Product price
    pub price: f64,
}

impl StoreProduct {
    /// Validates the incoming product data
    ///
    /// Returns Ok(()) if all validations pass
    pub fn validate(&self) -> Result<(), String> {
        // Validate name is not empty
        if self.name.trim().is_empty() {
            return Err("Product name is required".to_string());
        }

        // Validate name length
        if self.name.len() < 3 {
            return Err("Product name must be at least 3 characters".to_string());
        }

        // Validate price is positive
        if self.price <= 0.0 {
            return Err("Product price must be greater than 0".to_string());
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_valid_product() {
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: Some("A test product".to_string()),
            price: 99.99,
        };

        assert!(product.validate().is_ok());
    }

    #[test]
    fn validate_invalid_name_length() {
        let product = StoreProduct {
            name: "AB".to_string(),
            description: None,
            price: 99.99,
        };

        assert!(product.validate().is_err());
    }

    #[test]
    fn validate_invalid_price() {
        let product = StoreProduct {
            name: "Test Product".to_string(),
            description: None,
            price: -10.0,
        };

        assert!(product.validate().is_err());
    }
}
