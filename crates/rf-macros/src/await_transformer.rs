//! Auto-await insertion for async function calls

use syn::{
    visit_mut::{self, VisitMut},
    Expr, ExprCall, ExprMethodCall, Ident, Stmt,
};

/// Visitor that wraps known framework calls so they resolve transparently,
/// whether the underlying call is synchronous or asynchronous.
///
/// Instead of inserting a bare `.await` (which only compiles for genuine
/// futures), each matched call is wrapped in a tiny "maybe-await" adapter:
/// `(&__rf_w(EXPR)).__rf_resolve().await`. Via autoref specialization the
/// adapter awaits `EXPR` when it is a `Future` and passes it through unchanged
/// when it is a plain value. This is what lets a developer write framework code
/// without ever spelling out `.await` — the macro decides per call.
pub struct AwaitTransformer {
    /// List of known framework call names that should be resolved.
    async_functions: Vec<String>,
    /// Extra call names supplied per use via `#[auto_await(also("a", "b"))]`,
    /// so a developer can cover their own async methods without editing the
    /// framework. Wrapping is name-scoped (not "everything") on purpose: blindly
    /// wrapping every call injects `.await` into synchronous closures (e.g. the
    /// `|x| ...` of `.map`) and breaks inference on calls like `.collect()`.
    extra: Vec<String>,
    /// Set to `true` once at least one call has been wrapped, so the caller
    /// knows whether the adapter prelude needs to be injected.
    pub wrapped: bool,
}

impl AwaitTransformer {
    pub fn new() -> Self {
        Self {
            // Common async functions in the framework
            async_functions: vec![
                // Model methods (rf-db-facade) - snake_case
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
                "insert_many".to_string(),
                "get".to_string(),
                "exists".to_string(),
                "count".to_string(),
                "paginate".to_string(),
                "first_or_create".to_string(),
                "update_or_create".to_string(),
                "pluck".to_string(),
                "value".to_string(),
                // Model methods - Laravel camelCase aliases
                "findOrFail".to_string(),
                "firstOrFail".to_string(),
                "firstOrCreate".to_string(),
                "updateOrCreate".to_string(),
                "updateById".to_string(),
                "insertMany".to_string(),
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
                "dispatch_later".to_string(),
                // Broadcasting / WebSockets / Notifications
                "broadcast".to_string(),
                "publish".to_string(),
                "subscribe".to_string(),
                "notify".to_string(),
                // AI (rf-ai)
                "chat".to_string(),
                "embed".to_string(),
                "complete".to_string(),
                "generate".to_string(),
            ],
            extra: Vec::new(),
            wrapped: false,
        }
    }

    /// Construct with extra method names to resolve, from `#[auto_await(also(..))]`.
    pub fn with_extra(extra: Vec<String>) -> Self {
        let mut t = Self::new();
        t.extra = extra;
        t
    }

    /// Wrap a matched call expression in the maybe-await adapter so it resolves
    /// whether it is sync or async: `(&__rf_w(EXPR)).__rf_resolve().await`.
    fn wrap_resolve(&mut self, expr: &mut Expr) {
        // Already wrapped/awaited — leave it alone.
        if matches!(expr, Expr::Await(_)) {
            return;
        }
        let orig = expr.clone();
        *expr = syn::parse_quote! {
            (&__rf_w(#orig)).__rf_resolve().await
        };
        self.wrapped = true;
    }

    /// The adapter definitions to inject at the top of a transformed function
    /// body. Self-contained (std only) so generated code needs no extra crate.
    pub fn adapter_prelude() -> Vec<Stmt> {
        let block: syn::Block = syn::parse_quote! {{
            #[allow(non_camel_case_types, non_snake_case, dead_code)]
            struct __RfW<T>(::std::cell::Cell<::std::option::Option<T>>);
            #[allow(non_snake_case, dead_code)]
            fn __rf_w<T>(t: T) -> __RfW<T> {
                __RfW(::std::cell::Cell::new(::std::option::Option::Some(t)))
            }
            // More specific: the wrapped value is a Future -> await it.
            trait __RfResolveFut {
                type Out;
                fn __rf_resolve(&self) -> impl ::std::future::Future<Output = Self::Out>;
            }
            impl<F: ::std::future::Future> __RfResolveFut for __RfW<F> {
                type Out = F::Output;
                fn __rf_resolve(&self) -> impl ::std::future::Future<Output = F::Output> {
                    self.0.take().expect("auto_await adapter used twice")
                }
            }
            // Fallback (one extra autoref): a plain value -> pass it through.
            trait __RfResolveVal {
                type Out;
                fn __rf_resolve(&self) -> impl ::std::future::Future<Output = Self::Out>;
            }
            impl<T> __RfResolveVal for &__RfW<T> {
                type Out = T;
                fn __rf_resolve(&self) -> impl ::std::future::Future<Output = T> {
                    ::std::future::ready(self.0.take().expect("auto_await adapter used twice"))
                }
            }
        }};
        block.stmts
    }

    /// Add a custom async function name to the list
    pub fn add_async_function(&mut self, name: String) {
        self.async_functions.push(name);
    }

    /// Check if a method call should be resolved (framework list or user `also`).
    fn should_await_method(&self, method_name: &Ident) -> bool {
        self.async_functions.iter().any(|f| method_name == f)
            || self.extra.iter().any(|f| method_name == f)
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
                    self.wrap_resolve(expr);
                }
            }
            // Handle function calls: function()
            Expr::Call(call) => {
                let matched = if let Expr::Path(path) = &*call.func {
                    path.path
                        .segments
                        .last()
                        .map(|seg| self.should_await_method(&seg.ident))
                        .unwrap_or(false)
                } else {
                    false
                };
                if matched {
                    self.wrap_resolve(expr);
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
