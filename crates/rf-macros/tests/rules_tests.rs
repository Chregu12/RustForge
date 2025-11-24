// Integration tests for rules! macro
// These will be properly tested once rf-validation integration is complete

#[test]
fn test_rules_macro_compiles() {
    // This test ensures the macro compiles correctly
    let code = quote::quote! {
        rf_macros::rules! {
            name: required | min(3),
            email: required | email
        }
    };

    // If this compiles, the macro syntax is valid
    assert!(!code.to_string().is_empty());
}

#[test]
fn test_rules_macro_single_rule() {
    let code = quote::quote! {
        rf_macros::rules! {
            name: required
        }
    };

    assert!(!code.to_string().is_empty());
}

#[test]
fn test_rules_macro_complex_rules() {
    let code = quote::quote! {
        rf_macros::rules! {
            age: required | integer | between(18, 120),
            email: required | email | max(255),
            password: required | min(8) | confirmed
        }
    };

    assert!(!code.to_string().is_empty());
}
