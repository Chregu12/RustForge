//! `#[derive(Job)]` — ergonomic background job definitions.
//!
//! The `rf_queue::Job` trait requires a user body (`handle`) plus a lot of
//! mechanical wiring: `job_type()` (the type name) and the
//! `queue`/`max_retries`/`timeout`/`priority` accessors. A plain `#[derive]`
//! cannot supply the `handle` body, so this derive splits it out: the user
//! writes only `impl JobHandler` (one method, the body that actually matters),
//! and `#[derive(Job)]` generates the entire `Job` impl, delegating `handle`
//! to the `JobHandler` impl.
//!
//! ```ignore
//! #[derive(Serialize, Deserialize, Job)]
//! #[job(queue = "emails", retries = 5, timeout = 120, priority = 3)]
//! struct SendEmail { to: String }
//!
//! #[async_trait]
//! impl JobHandler for SendEmail {
//!     async fn handle(&self) -> Result<(), QueueError> { Ok(()) }
//! }
//! ```
//!
//! expands to a full `#[async_trait] impl rf_queue::Job for SendEmail` whose
//! `job_type()` returns `"SendEmail"`, `queue()` returns `"emails"`, etc., and
//! whose `handle()` calls `<Self as JobHandler>::handle`.

use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, punctuated::Punctuated, DeriveInput, Expr, MetaNameValue, Token,
};

pub fn derive_job(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

    // Optional `#[job(..)]` configuration.
    let mut job_type_val: Option<Expr> = None;
    let mut queue_val: Option<Expr> = None;
    let mut retries_val: Option<Expr> = None;
    let mut timeout_val: Option<Expr> = None;
    let mut priority_val: Option<Expr> = None;

    for attr in &input.attrs {
        if !attr.path().is_ident("job") {
            continue;
        }
        let nested = match attr
            .parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
        {
            Ok(n) => n,
            Err(e) => return e.to_compile_error().into(),
        };
        for nv in nested {
            let key = nv
                .path
                .get_ident()
                .map(|i| i.to_string())
                .unwrap_or_default();
            match key.as_str() {
                "job_type" | "name" => job_type_val = Some(nv.value),
                "queue" => queue_val = Some(nv.value),
                "retries" | "max_retries" => retries_val = Some(nv.value),
                "timeout" => timeout_val = Some(nv.value),
                "priority" => priority_val = Some(nv.value),
                other => {
                    return syn::Error::new_spanned(
                        nv.path,
                        format!(
                            "unknown #[job] option `{other}`; expected one of: \
                             job_type, queue, retries, timeout, priority"
                        ),
                    )
                    .to_compile_error()
                    .into();
                }
            }
        }
    }

    // `job_type()` defaults to the struct name; overridable via `#[job(job_type = ..)]`.
    let name_str = name.to_string();
    let job_type_impl = match job_type_val {
        Some(v) => quote! { fn job_type(&self) -> &'static str { #v } },
        None => quote! { fn job_type(&self) -> &'static str { #name_str } },
    };

    let queue_impl = queue_val
        .map(|v| quote! { fn queue(&self) -> &str { #v } })
        .unwrap_or_default();
    let retries_impl = retries_val
        .map(|v| quote! { fn max_retries(&self) -> u32 { #v } })
        .unwrap_or_default();
    let timeout_impl = timeout_val
        .map(|v| {
            quote! {
                fn timeout(&self) -> ::std::time::Duration {
                    ::std::time::Duration::from_secs(#v)
                }
            }
        })
        .unwrap_or_default();
    let priority_impl = priority_val
        .map(|v| quote! { fn priority(&self) -> i32 { #v } })
        .unwrap_or_default();

    let expanded = quote! {
        #[::rf_queue::async_trait::async_trait]
        impl #impl_generics ::rf_queue::Job for #name #ty_generics #where_clause {
            async fn handle(&self) -> ::std::result::Result<(), ::rf_queue::QueueError> {
                <Self as ::rf_queue::JobHandler>::handle(self).await
            }

            #job_type_impl
            #queue_impl
            #retries_impl
            #timeout_impl
            #priority_impl
        }
    };

    expanded.into()
}
