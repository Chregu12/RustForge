use serde_json::json;

/// Common test fixtures

/// Create a sample product fixture
pub fn product_fixture() -> serde_json::Value {
    json!({
        "name": "Test Product",
        "description": "A test product description",
        "price": "19.99",
        "stock": 100,
        "sku": "TEST-001",
        "active": true
    })
}

/// Create a sample account fixture
pub fn account_fixture() -> serde_json::Value {
    json!({
        "email": "test@example.com",
        "name": "Test User",
        "role": "user",
        "active": true
    })
}

/// Create multiple product fixtures
pub fn products_fixture(count: usize) -> Vec<serde_json::Value> {
    (1..=count)
        .map(|i| {
            json!({
                "name": format!("Product {}", i),
                "description": format!("Description for product {}", i),
                "price": format!("{}.99", 10 + i),
                "stock": 50 + i as i32,
                "sku": format!("TEST-{:03}", i),
                "active": true
            })
        })
        .collect()
}

/// Create multiple account fixtures
pub fn accounts_fixture(count: usize) -> Vec<serde_json::Value> {
    (1..=count)
        .map(|i| {
            json!({
                "email": format!("user{}@example.com", i),
                "name": format!("User {}", i),
                "role": if i % 2 == 0 { "admin" } else { "user" },
                "active": true
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_product_fixture() {
        let product = product_fixture();
        assert_eq!(product["name"], "Test Product");
        assert_eq!(product["sku"], "TEST-001");
    }

    #[test]
    fn test_account_fixture() {
        let account = account_fixture();
        assert_eq!(account["email"], "test@example.com");
        assert_eq!(account["role"], "user");
    }

    #[test]
    fn test_multiple_fixtures() {
        let products = products_fixture(5);
        assert_eq!(products.len(), 5);

        let accounts = accounts_fixture(3);
        assert_eq!(accounts.len(), 3);
    }
}
