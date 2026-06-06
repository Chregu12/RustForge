//! Integration tests for the RustForge framework
//!
//! Tests the main features together using in-memory/fake backends:
//! - Password hashing (bcrypt + argon2)
//! - Cache operations (in-memory)
//! - Validation rules
//! - Collection operations
//! - Mail fake

// ============================================================================
// Test 1: Password Hashing
// ============================================================================

mod password_hashing {
    use rf_auth::PasswordHasher;

    #[test]
    fn test_bcrypt_hash_and_verify() {
        // Use low cost (4) for fast tests
        let hasher = PasswordHasher::bcrypt(4).expect("bcrypt cost 4 is valid");
        let password = "super_secret_password_123";

        let hash = hasher.hash(password).expect("hashing should succeed");

        // Hash should have bcrypt prefix
        assert!(
            hash.starts_with("$2"),
            "bcrypt hash should start with $2, got: {}",
            &hash[..4]
        );

        // Correct password verifies
        assert!(
            hasher.verify(password, &hash).expect("verification should not error"),
            "correct password should verify"
        );

        // Wrong password does not verify
        assert!(
            !hasher.verify("wrong_password", &hash).expect("verification should not error"),
            "wrong password should not verify"
        );

        // Empty password does not verify against non-empty password hash
        assert!(
            !hasher.verify("", &hash).expect("verification should not error"),
            "empty password should not verify against non-empty hash"
        );
    }

    #[test]
    fn test_argon2_hash_and_verify() {
        let hasher = PasswordHasher::argon2().expect("argon2 creation should succeed");
        let password = "another_secure_password_456";

        let hash = hasher.hash(password).expect("hashing should succeed");

        // Hash should have argon2 prefix
        assert!(
            hash.starts_with("$argon2"),
            "argon2 hash should start with $argon2, got: {}",
            &hash[..8.min(hash.len())]
        );

        // Correct password verifies
        assert!(
            hasher.verify(password, &hash).expect("verification should not error"),
            "correct password should verify"
        );

        // Wrong password does not verify
        assert!(
            !hasher.verify("wrong_password", &hash).expect("verification should not error"),
            "wrong password should not verify"
        );
    }

    #[test]
    fn test_bcrypt_different_hashes_for_same_password() {
        // bcrypt uses random salts so the same password produces different hashes
        let hasher = PasswordHasher::bcrypt(4).expect("bcrypt cost 4 is valid");
        let password = "same_password";

        let hash1 = hasher.hash(password).expect("first hash should succeed");
        let hash2 = hasher.hash(password).expect("second hash should succeed");

        assert_ne!(hash1, hash2, "same password should produce different hashes due to random salt");

        // Both hashes should still verify the same password
        assert!(hasher.verify(password, &hash1).expect("verify hash1 should not error"));
        assert!(hasher.verify(password, &hash2).expect("verify hash2 should not error"));
    }

    #[test]
    fn test_cross_algorithm_verification() {
        // The verify method auto-detects the algorithm from the hash prefix
        let bcrypt_hasher = PasswordHasher::bcrypt(4).expect("bcrypt is valid");
        let argon2_hasher = PasswordHasher::argon2().expect("argon2 is valid");
        let password = "cross_check_password";

        let bcrypt_hash = bcrypt_hasher.hash(password).expect("bcrypt hash should succeed");
        let argon2_hash = argon2_hasher.hash(password).expect("argon2 hash should succeed");

        // Both hashers auto-detect the algorithm, so either can verify either hash
        assert!(
            argon2_hasher.verify(password, &bcrypt_hash).expect("cross-verify bcrypt should not error"),
            "argon2 hasher should be able to verify bcrypt hash via auto-detection"
        );
        assert!(
            bcrypt_hasher.verify(password, &argon2_hash).expect("cross-verify argon2 should not error"),
            "bcrypt hasher should be able to verify argon2 hash via auto-detection"
        );
    }

    #[test]
    fn test_invalid_bcrypt_cost_rejected() {
        assert!(PasswordHasher::bcrypt(3).is_err(), "cost 3 is below minimum of 4");
        assert!(PasswordHasher::bcrypt(32).is_err(), "cost 32 is above maximum of 31");
        assert!(PasswordHasher::bcrypt(4).is_ok(), "cost 4 is the minimum valid cost");
        assert!(PasswordHasher::bcrypt(12).is_ok(), "cost 12 is the recommended cost");
        assert!(PasswordHasher::bcrypt(31).is_ok(), "cost 31 is the maximum valid cost");
    }
}

// ============================================================================
// Test 2: Cache Operations
// ============================================================================

mod cache_operations {
    use rf_cache::{Cache, MemoryCache};
    use std::time::Duration;

    #[tokio::test]
    async fn test_cache_set_get_delete() {
        let cache = MemoryCache::new();

        // Key does not exist yet
        let missing: Option<String> = cache.get("greeting").await.expect("get should not error");
        assert!(missing.is_none(), "key should not exist before being set");

        // Set a value
        cache
            .set("greeting", &"Hello, World!".to_string(), Duration::from_secs(60))
            .await
            .expect("set should succeed");

        // Retrieve it
        let value: Option<String> = cache.get("greeting").await.expect("get should not error");
        assert_eq!(value, Some("Hello, World!".to_string()));

        // key exists
        assert!(
            cache.exists("greeting").await.expect("exists should not error"),
            "key should report as existing after set"
        );

        // Delete it
        cache.delete("greeting").await.expect("delete should succeed");

        // Gone now
        let after_delete: Option<String> =
            cache.get("greeting").await.expect("get after delete should not error");
        assert!(after_delete.is_none(), "key should be gone after delete");

        assert!(
            !cache.exists("greeting").await.expect("exists after delete should not error"),
            "exists should return false after delete"
        );
    }

    #[tokio::test]
    async fn test_cache_ttl_expiry() {
        let cache = MemoryCache::new();

        // Set with a very short TTL (1 millisecond)
        cache
            .set("short_lived", &42_i32, Duration::from_millis(1))
            .await
            .expect("set should succeed");

        // Value exists immediately
        let immediate: Option<i32> =
            cache.get("short_lived").await.expect("immediate get should not error");
        assert_eq!(immediate, Some(42), "value should be available immediately after set");

        // Wait for expiry — use tokio::time to avoid flakiness via a deterministic sleep
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Value should be gone
        let expired: Option<i32> =
            cache.get("short_lived").await.expect("get after expiry should not error");
        assert!(expired.is_none(), "value should be expired after TTL passes");
    }

    #[tokio::test]
    async fn test_cache_multiple_keys_and_types() {
        let cache = MemoryCache::new();
        let ttl = Duration::from_secs(60);

        cache.set("count", &100_i64, ttl).await.expect("set i64 should succeed");
        cache.set("flag", &true, ttl).await.expect("set bool should succeed");
        cache.set("label", &"RustForge".to_string(), ttl).await.expect("set String should succeed");

        let count: Option<i64> = cache.get("count").await.expect("get i64 should not error");
        let flag: Option<bool> = cache.get("flag").await.expect("get bool should not error");
        let label: Option<String> = cache.get("label").await.expect("get String should not error");

        assert_eq!(count, Some(100_i64));
        assert_eq!(flag, Some(true));
        assert_eq!(label, Some("RustForge".to_string()));
    }

    #[tokio::test]
    async fn test_cache_remember_pattern() {
        let cache = MemoryCache::new();
        let ttl = Duration::from_secs(60);

        let mut computation_count = 0_u32;

        // First call computes and caches
        let val: String = cache
            .remember("expensive_key", ttl, || {
                computation_count += 1;
                async { Ok("computed_result".to_string()) }
            })
            .await
            .expect("remember should succeed");

        assert_eq!(val, "computed_result");
        assert_eq!(computation_count, 1, "computation should have run once");

        // Second call should use the cache — computation_count does NOT change
        let val2: String = cache
            .remember("expensive_key", ttl, || {
                computation_count += 1;
                async { Ok("new_result".to_string()) }
            })
            .await
            .expect("remember should succeed");

        assert_eq!(val2, "computed_result", "should return cached value, not new result");
        assert_eq!(computation_count, 1, "computation should still only have run once");
    }

    #[tokio::test]
    async fn test_cache_flush() {
        let cache = MemoryCache::new();
        let ttl = Duration::from_secs(60);

        cache.set("k1", &"v1".to_string(), ttl).await.unwrap();
        cache.set("k2", &"v2".to_string(), ttl).await.unwrap();
        cache.set("k3", &"v3".to_string(), ttl).await.unwrap();

        assert!(cache.exists("k1").await.unwrap());
        assert!(cache.exists("k2").await.unwrap());
        assert!(cache.exists("k3").await.unwrap());

        cache.flush().await.expect("flush should succeed");

        assert!(!cache.exists("k1").await.unwrap(), "k1 should be gone after flush");
        assert!(!cache.exists("k2").await.unwrap(), "k2 should be gone after flush");
        assert!(!cache.exists("k3").await.unwrap(), "k3 should be gone after flush");
    }
}

// ============================================================================
// Test 3: Validation
// ============================================================================

mod validation {
    use rf_validation::{Rule, Validator, rules::*};
    use serde_json::json;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_email_validation() {
        // Valid emails
        let valid_emails = [
            "user@example.com",
            "user+tag@example.co.uk",
            "firstname.lastname@subdomain.example.org",
            "user123@example.io",
        ];

        for email in &valid_emails {
            let mut data = HashMap::new();
            data.insert("email".to_string(), json!(email));

            let result = Validator::quick_validate(
                data,
                HashMap::from([("email", vec![Box::new(EmailRule) as Box<dyn Rule>])]),
            )
            .await;

            assert!(result.is_ok(), "email '{}' should be valid", email);
        }

        // Invalid emails
        let invalid_emails = [
            "not-an-email",
            "missing@tld",
            "@no-local-part.com",
            "spaces in@email.com",
        ];

        for email in &invalid_emails {
            let mut data = HashMap::new();
            data.insert("email".to_string(), json!(email));

            let result = Validator::quick_validate(
                data,
                HashMap::from([("email", vec![Box::new(EmailRule) as Box<dyn Rule>])]),
            )
            .await;

            assert!(result.is_err(), "email '{}' should be invalid", email);
        }
    }

    #[tokio::test]
    async fn test_required_field_validation() {
        let required_rule = || vec![Box::new(RequiredRule) as Box<dyn Rule>];

        // Missing field (null)
        let mut data = HashMap::new();
        data.insert("name".to_string(), json!(null));

        let result = Validator::quick_validate(data, HashMap::from([("name", required_rule())])).await;
        assert!(result.is_err(), "null value should fail required rule");

        // Empty string
        let mut data = HashMap::new();
        data.insert("name".to_string(), json!(""));

        let result = Validator::quick_validate(data, HashMap::from([("name", required_rule())])).await;
        assert!(result.is_err(), "empty string should fail required rule");

        // Whitespace only
        let mut data = HashMap::new();
        data.insert("name".to_string(), json!("   "));

        let result = Validator::quick_validate(data, HashMap::from([("name", required_rule())])).await;
        assert!(result.is_err(), "whitespace-only string should fail required rule");

        // Non-empty string passes
        let mut data = HashMap::new();
        data.insert("name".to_string(), json!("Alice"));

        let result = Validator::quick_validate(data, HashMap::from([("name", required_rule())])).await;
        assert!(result.is_ok(), "non-empty string should pass required rule");

        // Number passes
        let mut data = HashMap::new();
        data.insert("age".to_string(), json!(25));

        let result = Validator::quick_validate(data, HashMap::from([("age", required_rule())])).await;
        assert!(result.is_ok(), "number should pass required rule");
    }

    #[tokio::test]
    async fn test_min_max_length_validation() {
        let min_rule = || vec![Box::new(MinLengthRule::new(5)) as Box<dyn Rule>];
        let max_rule = || vec![Box::new(MaxLengthRule::new(10)) as Box<dyn Rule>];

        // ---- MinLengthRule ----

        // Exactly at minimum — passes
        let mut data = HashMap::new();
        data.insert("field".to_string(), json!("hello")); // 5 chars
        let result = Validator::quick_validate(data, HashMap::from([("field", min_rule())])).await;
        assert!(result.is_ok(), "string of exactly min length should pass");

        // Below minimum — fails
        let mut data = HashMap::new();
        data.insert("field".to_string(), json!("hi")); // 2 chars
        let result = Validator::quick_validate(data, HashMap::from([("field", min_rule())])).await;
        assert!(result.is_err(), "string below min length should fail");

        // Above minimum — passes
        let mut data = HashMap::new();
        data.insert("field".to_string(), json!("hello world")); // 11 chars
        let result = Validator::quick_validate(data, HashMap::from([("field", min_rule())])).await;
        assert!(result.is_ok(), "string above min length should pass");

        // ---- MaxLengthRule ----

        // Exactly at maximum — passes
        let mut data = HashMap::new();
        data.insert("field".to_string(), json!("1234567890")); // 10 chars
        let result = Validator::quick_validate(data, HashMap::from([("field", max_rule())])).await;
        assert!(result.is_ok(), "string of exactly max length should pass");

        // Above maximum — fails
        let mut data = HashMap::new();
        data.insert("field".to_string(), json!("this_is_too_long_string")); // > 10 chars
        let result = Validator::quick_validate(data, HashMap::from([("field", max_rule())])).await;
        assert!(result.is_err(), "string above max length should fail");

        // ---- Custom messages ----
        let mut data = HashMap::new();
        data.insert("username".to_string(), json!("ab")); // too short

        let mut validator = Validator::new(data);
        validator.rules(HashMap::from([(
            "username",
            vec![Box::new(MinLengthRule::new(3)) as Box<dyn Rule>],
        )]));
        validator.messages(HashMap::from([(
            "username.min_length",
            "Username must be at least 3 characters",
        )]));

        let errors = validator.validate().await.unwrap_err();
        let field_errors = errors.get("username").expect("should have username errors");
        assert_eq!(
            field_errors[0].message,
            "Username must be at least 3 characters",
            "custom message should be used"
        );
    }

    #[tokio::test]
    async fn test_multiple_rules_on_same_field() {
        // Test composing multiple rules: required + email
        let rules: Vec<Box<dyn Rule>> = vec![
            Box::new(RequiredRule),
            Box::new(EmailRule),
        ];

        // Valid email passes both
        let mut data = HashMap::new();
        data.insert("email".to_string(), json!("user@example.com"));
        let result = Validator::quick_validate(
            data,
            HashMap::from([("email", rules)]),
        )
        .await;
        assert!(result.is_ok(), "valid email should pass both required and email rules");

        // Empty fails required
        let rules2: Vec<Box<dyn Rule>> = vec![
            Box::new(RequiredRule),
            Box::new(EmailRule),
        ];
        let mut data2 = HashMap::new();
        data2.insert("email".to_string(), json!(""));
        let result2 = Validator::quick_validate(data2, HashMap::from([("email", rules2)])).await;
        assert!(result2.is_err(), "empty email should fail required rule");

        // Invalid email fails email rule
        let rules3: Vec<Box<dyn Rule>> = vec![
            Box::new(RequiredRule),
            Box::new(EmailRule),
        ];
        let mut data3 = HashMap::new();
        data3.insert("email".to_string(), json!("not-an-email"));
        let result3 = Validator::quick_validate(data3, HashMap::from([("email", rules3)])).await;
        assert!(result3.is_err(), "invalid email should fail email rule");
    }
}

// ============================================================================
// Test 4: Collection Operations
// ============================================================================

mod collection_ops {
    use rf_collections::collect;

    #[derive(Debug, Clone, PartialEq)]
    struct Product {
        id: u32,
        name: String,
        category: String,
        price: f64,
        in_stock: bool,
    }

    fn sample_products() -> Vec<Product> {
        vec![
            Product { id: 1, name: "Laptop".to_string(), category: "Electronics".to_string(), price: 999.99, in_stock: true },
            Product { id: 2, name: "Mouse".to_string(), category: "Electronics".to_string(), price: 29.99, in_stock: true },
            Product { id: 3, name: "Desk".to_string(), category: "Furniture".to_string(), price: 349.99, in_stock: false },
            Product { id: 4, name: "Chair".to_string(), category: "Furniture".to_string(), price: 199.99, in_stock: true },
            Product { id: 5, name: "Keyboard".to_string(), category: "Electronics".to_string(), price: 79.99, in_stock: false },
        ]
    }

    #[test]
    fn test_collection_map_filter() {
        let products = collect(sample_products());

        // Filter to only in-stock items and map to their names
        let in_stock_names: Vec<String> = products
            .filter(|p| p.in_stock)
            .map(|p| p.name.clone())
            .to_vec();

        assert_eq!(in_stock_names.len(), 3, "should have 3 in-stock products");
        assert!(in_stock_names.contains(&"Laptop".to_string()));
        assert!(in_stock_names.contains(&"Mouse".to_string()));
        assert!(in_stock_names.contains(&"Chair".to_string()));
        assert!(!in_stock_names.contains(&"Desk".to_string()), "Desk is out of stock");
        assert!(!in_stock_names.contains(&"Keyboard".to_string()), "Keyboard is out of stock");

        // Map prices to cents (i32)
        let prices_in_cents: Vec<i64> = collect(sample_products())
            .map(|p| (p.price * 100.0) as i64)
            .to_vec();

        assert_eq!(prices_in_cents.len(), 5);
        assert_eq!(prices_in_cents[0], 99999); // 999.99 * 100
    }

    #[test]
    fn test_collection_group_by() {
        let products = collect(sample_products());

        let by_category = products.group_by(|p| p.category.clone());

        assert_eq!(by_category.len(), 2, "should have 2 categories");

        let electronics = by_category.get("Electronics").expect("Electronics category should exist");
        assert_eq!(electronics.len(), 3, "should have 3 electronics");

        let furniture = by_category.get("Furniture").expect("Furniture category should exist");
        assert_eq!(furniture.len(), 2, "should have 2 furniture items");

        // Verify the right products are in each group
        assert!(electronics.iter().any(|p| p.name == "Laptop"));
        assert!(electronics.iter().any(|p| p.name == "Mouse"));
        assert!(electronics.iter().any(|p| p.name == "Keyboard"));
        assert!(furniture.iter().any(|p| p.name == "Desk"));
        assert!(furniture.iter().any(|p| p.name == "Chair"));
    }

    #[test]
    fn test_collection_paginate() {
        // Create a collection of 10 items
        let items: Vec<i32> = (1..=10).collect();
        let collection = collect(items);

        // Page 1, 3 per page → items 1, 2, 3
        let page1 = collection.clone().for_page(1, 3).to_vec();
        assert_eq!(page1, vec![1, 2, 3], "page 1 should be items 1-3");

        // Page 2, 3 per page → items 4, 5, 6
        let page2 = collection.clone().for_page(2, 3).to_vec();
        assert_eq!(page2, vec![4, 5, 6], "page 2 should be items 4-6");

        // Page 3, 3 per page → items 7, 8, 9
        let page3 = collection.clone().for_page(3, 3).to_vec();
        assert_eq!(page3, vec![7, 8, 9], "page 3 should be items 7-9");

        // Page 4, 3 per page → item 10 (last partial page)
        let page4 = collection.clone().for_page(4, 3).to_vec();
        assert_eq!(page4, vec![10], "page 4 should have just item 10");

        // Page 5 (beyond end) → empty
        let page5 = collection.for_page(5, 3).to_vec();
        assert!(page5.is_empty(), "page beyond end should be empty");
    }

    #[test]
    fn test_collection_sort_by() {
        let products = collect(sample_products());

        // Sort by price ascending
        let by_price_asc: Vec<String> = products
            .sort_by(|p| (p.price * 100.0) as i64)
            .map(|p| p.name.clone())
            .to_vec();

        assert_eq!(by_price_asc[0], "Mouse",    "Mouse ($29.99) should be cheapest");
        assert_eq!(by_price_asc[4], "Laptop",   "Laptop ($999.99) should be most expensive");
    }

    #[test]
    fn test_collection_reduce() {
        let numbers = collect(vec![1_i64, 2, 3, 4, 5]);

        let sum = numbers.reduce(0_i64, |acc, x| acc + x);
        assert_eq!(sum, 15, "sum of 1..=5 should be 15");
    }

    #[test]
    fn test_collection_chunk() {
        let numbers = collect(vec![1, 2, 3, 4, 5, 6, 7]);

        let chunks = numbers.chunk(3);
        assert_eq!(chunks.len(), 3, "7 items chunked by 3 → 3 chunks");
        assert_eq!(chunks[0].all(), &[1, 2, 3]);
        assert_eq!(chunks[1].all(), &[4, 5, 6]);
        assert_eq!(chunks[2].all(), &[7]);
    }
}

// ============================================================================
// Test 5: Mail Fake
// ============================================================================

mod mail_fake {
    use rf_testing::fakes::{MailFake, MailRecord};
    use rf_testing::fakes::mail::{Address, MailBody, Mailer};

    fn make_record(subject: &str, to: &str, from: &str) -> MailRecord {
        MailRecord {
            to: vec![Address::new(to)],
            cc: vec![],
            bcc: vec![],
            from: Address::new(from),
            reply_to: None,
            subject: subject.to_string(),
            body: MailBody::Text("Body content".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_mail_fake_captures_sent_mails() {
        let fake = MailFake::new();

        // Initially empty
        assert_eq!(fake.count(), 0, "no mails should be captured initially");
        fake.assert_nothing_sent();

        // Send a welcome mail
        let welcome_mail = rf_mail::MailBuilder::new()
            .from(Address::new("noreply@rustforge.dev"))
            .to(Address::new("alice@example.com"))
            .subject("Welcome to RustForge!")
            .text("Welcome aboard, Alice!")
            .build()
            .expect("mail should build successfully");

        fake.send(welcome_mail).await.expect("send should succeed");

        assert_eq!(fake.count(), 1, "one mail should have been captured");
        assert!(fake.has_sent("Welcome to RustForge!"), "should have sent welcome mail");
        assert!(fake.has_sent_to("alice@example.com"), "should have sent to alice");

        // Send a second mail
        let reset_mail = rf_mail::MailBuilder::new()
            .from(Address::new("noreply@rustforge.dev"))
            .to(Address::new("bob@example.com"))
            .subject("Password Reset")
            .text("Click here to reset your password.")
            .build()
            .expect("mail should build successfully");

        fake.send(reset_mail).await.expect("send should succeed");

        assert_eq!(fake.count(), 2, "two mails should have been captured");

        // Assertions by subject
        fake.assert_sent("Welcome to RustForge!");
        fake.assert_sent("Password Reset");
        fake.assert_not_sent("Invoice");

        // Assertions by count
        fake.assert_sent_times("Welcome to RustForge!", 1);
        fake.assert_sent_times("Password Reset", 1);

        // Sent-to assertions
        fake.assert_sent_to("alice@example.com");
        fake.assert_sent_to("bob@example.com");

        // Predicate-based assertion
        fake.assert_sent_with(|record| {
            record.from.email == "noreply@rustforge.dev"
        });
    }

    #[tokio::test]
    async fn test_mail_fake_multiple_recipients_and_cc() {
        let fake = MailFake::new();

        // Record a mail with CC directly
        let record = MailRecord {
            to: vec![Address::new("primary@example.com")],
            cc: vec![Address::new("cc@example.com")],
            bcc: vec![Address::new("bcc@example.com")],
            from: Address::new("sender@example.com"),
            reply_to: None,
            subject: "Newsletter".to_string(),
            body: MailBody::Text("The newsletter content".to_string()),
            attachments: vec![],
            sent_at: chrono::Utc::now(),
        };

        fake.record_send(record);

        // All recipient types should be found by has_sent_to
        assert!(fake.has_sent_to("primary@example.com"), "to address should be found");
        assert!(fake.has_sent_to("cc@example.com"), "cc address should be found");
        assert!(fake.has_sent_to("bcc@example.com"), "bcc address should be found");
        assert!(!fake.has_sent_to("nobody@example.com"), "unrelated address should not be found");
    }

    #[tokio::test]
    async fn test_mail_fake_subject_contains() {
        let fake = MailFake::new();

        fake.record_send(make_record("Your Order Has Shipped!", "user@example.com", "shop@example.com"));
        fake.record_send(make_record("Order Confirmation #1234", "user@example.com", "shop@example.com"));
        fake.record_send(make_record("Welcome!", "user@example.com", "app@example.com"));

        // sent_with_subject_containing
        let order_mails = fake.sent_with_subject_containing("Order");
        assert_eq!(order_mails.len(), 2, "two mails should contain 'Order' in subject");

        let welcome_mails = fake.sent_with_subject_containing("Welcome");
        assert_eq!(welcome_mails.len(), 1, "one mail should contain 'Welcome' in subject");

        let none = fake.sent_with_subject_containing("Invoice");
        assert!(none.is_empty(), "no mail should contain 'Invoice' in subject");
    }

    #[tokio::test]
    async fn test_mail_fake_clear() {
        let fake = MailFake::new();

        fake.record_send(make_record("First", "a@example.com", "b@example.com"));
        fake.record_send(make_record("Second", "a@example.com", "b@example.com"));
        assert_eq!(fake.count(), 2);

        fake.clear();
        assert_eq!(fake.count(), 0, "count should be 0 after clear");
        fake.assert_nothing_sent();
    }

    #[tokio::test]
    async fn test_mail_fake_batch_send() {
        let fake = MailFake::new();

        let mails = vec![
            rf_mail::MailBuilder::new()
                .from(Address::new("sender@example.com"))
                .to(Address::new("user1@example.com"))
                .subject("Batch Mail 1")
                .text("Content 1")
                .build()
                .unwrap(),
            rf_mail::MailBuilder::new()
                .from(Address::new("sender@example.com"))
                .to(Address::new("user2@example.com"))
                .subject("Batch Mail 2")
                .text("Content 2")
                .build()
                .unwrap(),
            rf_mail::MailBuilder::new()
                .from(Address::new("sender@example.com"))
                .to(Address::new("user3@example.com"))
                .subject("Batch Mail 3")
                .text("Content 3")
                .build()
                .unwrap(),
        ];

        fake.send_batch(mails).await.expect("batch send should succeed");

        assert_eq!(fake.count(), 3, "all 3 mails should be captured");
        fake.assert_sent("Batch Mail 1");
        fake.assert_sent("Batch Mail 2");
        fake.assert_sent("Batch Mail 3");
        fake.assert_sent_to("user1@example.com");
        fake.assert_sent_to("user2@example.com");
        fake.assert_sent_to("user3@example.com");
    }
}
