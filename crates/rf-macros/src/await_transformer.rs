//! Auto-await insertion for async function calls

use syn::{
    visit_mut::{self, VisitMut},
    Expr, ExprAwait, ExprCall, ExprMethodCall, Ident,
};

/// Visitor that adds `.await` to known async function calls
pub struct AwaitTransformer {
    /// List of known async function names that should be awaited
    async_functions: Vec<String>,
}

impl AwaitTransformer {
    pub fn new() -> Self {
        Self {
            // Common async functions in the framework
            async_functions: vec![
                // Model methods (rf-db-facade)
                "find".to_string(),
                "find_or_fail".to_string(),
                "all".to_string(),
                "first".to_string(),
                "first_or_fail".to_string(),
                "create".to_string(),
                "update".to_string(),
                "update_by_id".to_string(),
                "delete".to_string(),
                "destroy".to_string(),
                "save".to_string(),
                "insert".to_string(),
                "get".to_string(),
                "exists".to_string(),
                "count".to_string(),
                "paginate".to_string(),
                "first_or_create".to_string(),
                "update_or_create".to_string(),
                // Cache methods (rf-cache-facade)
                "put".to_string(),
                "forget".to_string(),
                "flush".to_string(),
                "has".to_string(),
                "pull".to_string(),
                "add".to_string(),
                "forever".to_string(),
                "remember".to_string(),
                "remember_forever".to_string(),
                "increment".to_string(),
                "decrement".to_string(),
                "tags".to_string(),
                // Auth methods (rf-auth-facade)
                "attempt".to_string(),
                "login".to_string(),
                "logout".to_string(),
                "check".to_string(),
                "guest".to_string(),
                "user".to_string(),
                "id".to_string(),
                // Request methods
                "validate".to_string(),
                // Database
                "execute".to_string(),
                "fetch".to_string(),
                "fetch_one".to_string(),
                "fetch_all".to_string(),
                "fetch_optional".to_string(),
                "begin_transaction".to_string(),
                "commit".to_string(),
                "rollback".to_string(),
                // Storage
                "store".to_string(),
                "download".to_string(),
                // Mail
                "send".to_string(),
                // Queue
                "push".to_string(),
                "dispatch".to_string(),
            ],
        }
    }

    /// Add a custom async function name to the list
    pub fn add_async_function(&mut self, name: String) {
        self.async_functions.push(name);
    }

    /// Check if a method call should be awaited
    fn should_await_method(&self, method_name: &Ident) -> bool {
        self.async_functions.iter().any(|f| method_name == f)
    }

    /// Transform an expression by adding .await where necessary
    pub fn transform_expr(&mut self, expr: &mut Expr) {
        self.visit_expr_mut(expr);
    }
}

impl VisitMut for AwaitTransformer {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // First, visit children
        visit_mut::visit_expr_mut(self, expr);

        // Then, check if we need to add .await
        match expr {
            // Handle method calls: object.method()
            Expr::MethodCall(method_call) => {
                if self.should_await_method(&method_call.method) {
                    // Check if it's already awaited
                    let is_already_awaited = matches!(
                        expr,
                        Expr::Await(_)
                    );

                    if !is_already_awaited {
                        // Wrap in .await
                        let awaited = ExprAwait {
                            attrs: Vec::new(),
                            base: Box::new(expr.clone()),
                            dot_token: Default::default(),
                            await_token: Default::default(),
                        };
                        *expr = Expr::Await(awaited);
                    }
                }
            }
            // Handle function calls: function()
            Expr::Call(call) => {
                // Check if it's a path (e.g., User::find())
                if let Expr::Path(path) = &*call.func {
                    if let Some(last_segment) = path.path.segments.last() {
                        let func_name = &last_segment.ident;
                        if self.should_await_method(func_name) {
                            let is_already_awaited = matches!(
                                expr,
                                Expr::Await(_)
                            );

                            if !is_already_awaited {
                                let awaited = ExprAwait {
                                    attrs: Vec::new(),
                                    base: Box::new(expr.clone()),
                                    dot_token: Default::default(),
                                    await_token: Default::default(),
                                };
                                *expr = Expr::Await(awaited);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn visit_expr_method_call_mut(&mut self, method_call: &mut ExprMethodCall) {
        // Visit the receiver first
        self.visit_expr_mut(&mut method_call.receiver);

        // Visit the arguments
        for arg in &mut method_call.args {
            self.visit_expr_mut(arg);
        }

        // Don't transform the method call itself here, as it's handled in visit_expr_mut
    }

    fn visit_expr_call_mut(&mut self, call: &mut ExprCall) {
        // Visit the function
        self.visit_expr_mut(&mut call.func);

        // Visit the arguments
        for arg in &mut call.args {
            self.visit_expr_mut(arg);
        }
    }
}

impl Default for AwaitTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_transform_method_call() {
        let mut transformer = AwaitTransformer::new();
        let mut expr: Expr = parse_quote! { user.save() };

        transformer.transform_expr(&mut expr);

        // Should be transformed to user.save().await
        assert!(matches!(expr, Expr::Await(_)));
    }

    #[test]
    fn test_transform_static_call() {
        let mut transformer = AwaitTransformer::new();
        let mut expr: Expr = parse_quote! { User::find(1) };

        transformer.transform_expr(&mut expr);

        // Should be transformed to User::find(1).await
        assert!(matches!(expr, Expr::Await(_)));
    }

    #[test]
    fn test_no_transform_non_async() {
        let mut transformer = AwaitTransformer::new();
        let mut expr: Expr = parse_quote! { user.name() };

        transformer.transform_expr(&mut expr);

        // Should NOT be transformed
        assert!(matches!(expr, Expr::MethodCall(_)));
    }

    #[test]
    fn test_nested_calls() {
        let mut transformer = AwaitTransformer::new();
        let mut expr: Expr = parse_quote! { User::find(1).update(data) };

        transformer.transform_expr(&mut expr);

        // Both should be awaited
        assert!(matches!(expr, Expr::Await(_)));
    }
}
