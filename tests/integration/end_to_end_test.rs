//! End-to-end integration tests
//!
//! Tests complete workflows combining multiple components.

use domain::models::account::Account;
use domain::models::product::Product;
use app::events::order_placed_event::{OrderPlacedEvent, OrderPlacedPayload};
use app::http::requests::store_product_request::StoreProduct;

#[test]
fn complete_product_order_workflow() {
    // Create account
    let account = Account::new(
        1,
        "customer@example.com".to_string(),
        "John Doe".to_string(),
    );
    assert!(account.is_active);

    // Create product
    let mut product = Product::new(
        1,
        "Laptop".to_string(),
        1299.99,
        5,
    )
    .with_description("High performance laptop".to_string());

    // Customer purchases product
    assert_eq!(product.stock, 5);
    product
        .decrease_stock(1)
        .expect("Failed to decrease stock");
    assert_eq!(product.stock, 4);

    // Fire order placed event
    let event_payload = OrderPlacedPayload {
        user_id: account.id,
        order_id: "ORD-001".to_string(),
        amount: 1299.99,
    };
    let event = OrderPlacedEvent::new(event_payload);

    assert_eq!(event.payload.user_id, 1);
    assert_eq!(event.payload.order_id, "ORD-001");
    assert_eq!(event.payload.amount, 1299.99);
}

#[test]
fn account_lifecycle() {
    let mut account = Account::new(
        1,
        "test@example.com".to_string(),
        "Test User".to_string(),
    );

    // Account starts active
    assert!(account.is_active);

    // Deactivate account
    account.deactivate();
    assert!(!account.is_active);

    // Reactivate account
    account.activate();
    assert!(account.is_active);

    // Update tracking
    let original_updated = account.updated_at;
    std::thread::sleep(std::time::Duration::from_millis(10));
    account.deactivate();
    assert!(account.updated_at > original_updated);
}

#[test]
fn product_inventory_management() {
    let mut product = Product::new(
        1,
        "Widget".to_string(),
        29.99,
        100,
    );

    // Simulate multiple sales
    for i in 0..10 {
        product
            .decrease_stock(5)
            .expect(&format!("Sale {} failed", i + 1));
    }
    assert_eq!(product.stock, 50);

    // Restock
    product.increase_stock(25);
    assert_eq!(product.stock, 75);

    // Verify we can sell remaining stock
    product
        .decrease_stock(75)
        .expect("Final sale failed");
    assert_eq!(product.stock, 0);

    // Verify we can't sell more than available
    let result = product.decrease_stock(1);
    assert!(result.is_err());
}

#[test]
fn request_validation_workflow() {
    // Validate a valid product request
    let valid_request = StoreProduct {
        name: "New Product".to_string(),
        description: Some("Product description".to_string()),
        price: 49.99,
    };
    assert!(valid_request.validate().is_ok());

    // Validate an invalid request
    let invalid_request = StoreProduct {
        name: "AB".to_string(),
        description: None,
        price: 49.99,
    };
    assert!(invalid_request.validate().is_err());

    // Validate edge case
    let edge_case = StoreProduct {
        name: "ABC".to_string(),
        description: None,
        price: 0.01,
    };
    assert!(edge_case.validate().is_ok());
}

#[test]
fn multi_product_transaction() {
    let mut products = vec![
        Product::new(1, "Item A".to_string(), 10.0, 5),
        Product::new(2, "Item B".to_string(), 20.0, 3),
        Product::new(3, "Item C".to_string(), 30.0, 2),
    ];

    let mut total_amount = 0.0;

    // Customer buys from multiple products
    let purchases = vec![(0, 2), (1, 1), (2, 1)]; // (product_index, quantity)

    for (idx, qty) in purchases {
        let product = &mut products[idx];
        product
            .decrease_stock(qty)
            .expect("Purchase failed");
        total_amount += product.price * qty as f64;
    }

    assert_eq!(total_amount, 10.0 * 2.0 + 20.0 * 1.0 + 30.0 * 1.0);
    assert_eq!(products[0].stock, 3); // 5 - 2
    assert_eq!(products[1].stock, 2); // 3 - 1
    assert_eq!(products[2].stock, 1); // 2 - 1

    // Fire order placed event
    let event = OrderPlacedEvent::new(OrderPlacedPayload {
        user_id: 1,
        order_id: "ORD-MULTI-001".to_string(),
        amount: total_amount,
    });

    assert_eq!(event.payload.amount, 60.0);
}

#[test]
fn concurrent_account_operations() {
    let mut account = Account::new(
        1,
        "test@example.com".to_string(),
        "Test User".to_string(),
    );

    // Simulate multiple operations
    account.deactivate();
    assert!(!account.is_active);

    account.activate();
    assert!(account.is_active);

    account.deactivate();
    assert!(!account.is_active);

    // Final state verification
    assert_eq!(account.email, "test@example.com");
    assert_eq!(account.name, "Test User");
}

#[test]
fn large_transaction_handling() {
    let mut product = Product::new(
        1,
        "Bulk Item".to_string(),
        0.50,
        1000,
    );

    // Large purchase
    product
        .decrease_stock(500)
        .expect("Large purchase failed");
    assert_eq!(product.stock, 500);

    // Verify price calculation for large orders
    let unit_price = product.price;
    let order_amount = unit_price * 500.0;
    assert_eq!(order_amount, 250.0);

    // Attempt to purchase more than available
    let result = product.decrease_stock(600);
    assert!(result.is_err());
    assert_eq!(product.stock, 500); // Stock should not change
}
