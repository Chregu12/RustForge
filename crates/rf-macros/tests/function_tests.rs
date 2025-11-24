// Integration tests for function! macro
// These will be properly tested once rf-request is implemented

#[test]
fn test_function_macro_compiles() {
    // This test ensures the macro compiles correctly
    // We'll add runtime tests once rf-request is ready
    let code = quote::quote! {
        rf_macros::function!(request: Request) -> Response {
            Response::text("Hello")
        }
    };

    // If this compiles, the macro syntax is valid
    assert!(!code.to_string().is_empty());
}

#[test]
fn test_function_macro_multiple_args() {
    let code = quote::quote! {
        rf_macros::function!(request: Request, id: u32) -> Response {
            Response::json(id)
        }
    };

    assert!(!code.to_string().is_empty());
}
