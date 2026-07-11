//! **EXPERIMENTAL — not part of the RustForge 1.0 supported surface; API may change without a SemVer bump.**
//!
//! Derive macros for rf-nova
//!
//! Provides #[derive(Action)], #[derive(Filter)], etc.
//!
//! # Note on `#[derive(Resource)]`
//!
//! The `Resource` derive is **not yet usable** — the attribute-based code
//! generation for `type Entity` and `type Model` is unimplemented. Using it
//! silently produced `type Entity = ()` and `type Model = ()` which break any
//! code that passes `R::Entity` to the generic crud functions.
//!
//! Implement the [`rf_nova::resource::Resource`] trait manually instead:
//!
//! ```ignore
//! pub struct UserResource;
//!
//! impl rf_nova::resource::Resource for UserResource {
//!     type Entity = user::Entity;
//!     type Model = user::Model;
//!
//!     fn name() -> &'static str { "User" }
//!
//!     fn fields() -> Vec<Box<dyn rf_nova::resource::Field>> {
//!         vec![
//!             Box::new(rf_nova::resource::field::ID::new("id")),
//!             Box::new(rf_nova::resource::field::Text::new("name")),
//!         ]
//!     }
//! }
//! ```

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// **Unimplemented** — emits a compile error directing you to the manual impl.
///
/// The attribute-based `Entity` / `Model` extraction needed to make this derive
/// useful is not yet implemented. Please implement the `Resource` trait manually.
/// See the crate-level docs for an example.
#[proc_macro_derive(Resource, attributes(nova))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;
    let name_str = name.to_string();

    let expanded = quote! {
        compile_error!(concat!(
            "#[derive(Resource)] on `",
            #name_str,
            "` is not yet usable: the attribute-based `type Entity` / `type Model` \
             generation is unimplemented and would silently produce broken `()` types. \
             Please implement the `rf_nova::resource::Resource` trait manually. \
             See the rf-nova-macros crate docs for an example."
        ));
    };

    TokenStream::from(expanded)
}

/// Derive macro for Action trait
///
/// # Example
///
/// ```ignore
/// #[derive(Action)]
/// #[nova(name = "Deactivate User", destructive = true)]
/// pub struct DeactivateUser;
/// ```
#[proc_macro_derive(Action, attributes(nova))]
pub fn derive_action(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[async_trait::async_trait]
        impl rf_nova::action::Action for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            async fn handle(
                &self,
                models: Vec<serde_json::Value>,
                fields: rf_nova::action::ActionFields,
            ) -> rf_nova::action::ActionResult {
                rf_nova::action::ActionResponse::success("Action completed")
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for Filter trait
///
/// # Example
///
/// ```ignore
/// #[derive(Filter)]
/// #[nova(name = "User Type")]
/// pub struct UserTypeFilter;
/// ```
#[proc_macro_derive(Filter, attributes(nova))]
pub fn derive_filter(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl rf_nova::filter::Filter for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            fn apply(&self, value: &str) -> rf_nova::filter::FilterCondition {
                rf_nova::filter::FilterCondition::Equals {
                    field: "id".to_string(),
                    value: value.to_string(),
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for Lens trait
///
/// # Example
///
/// ```ignore
/// #[derive(Lens)]
/// #[nova(name = "Most Active Users")]
/// pub struct MostActiveUsersLens;
/// ```
#[proc_macro_derive(Lens, attributes(nova))]
pub fn derive_lens(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        impl rf_nova::lens::Lens for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            fn query(&self) -> rf_nova::lens::LensQuery {
                rf_nova::lens::LensQuery::new()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for ValueMetric trait
///
/// # Example
///
/// ```ignore
/// #[derive(ValueMetric)]
/// #[nova(name = "Total Users")]
/// pub struct TotalUsers;
/// ```
#[proc_macro_derive(ValueMetric, attributes(nova))]
pub fn derive_value_metric(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[async_trait::async_trait]
        impl rf_nova::metric::ValueMetric for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            async fn calculate(&self) -> Result<rf_nova::metric::MetricValue, rf_nova::metric::MetricError> {
                Ok(rf_nova::metric::MetricValue::new(0.0))
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for TrendMetric trait
#[proc_macro_derive(TrendMetric, attributes(nova))]
pub fn derive_trend_metric(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[async_trait::async_trait]
        impl rf_nova::metric::TrendMetric for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            async fn calculate(
                &self,
                range: rf_nova::metric::DateRange,
            ) -> Result<rf_nova::metric::TrendData, rf_nova::metric::MetricError> {
                Ok(rf_nova::metric::TrendData::new())
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for PartitionMetric trait
#[proc_macro_derive(PartitionMetric, attributes(nova))]
pub fn derive_partition_metric(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[async_trait::async_trait]
        impl rf_nova::metric::PartitionMetric for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            async fn calculate(&self) -> Result<rf_nova::metric::PartitionData, rf_nova::metric::MetricError> {
                Ok(rf_nova::metric::PartitionData::new())
            }
        }
    };

    TokenStream::from(expanded)
}

/// Derive macro for Card trait
#[proc_macro_derive(Card, attributes(nova))]
pub fn derive_card(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    let expanded = quote! {
        #[async_trait::async_trait]
        impl rf_nova::card::Card for #name {
            fn name(&self) -> &str {
                stringify!(#name)
            }

            fn component(&self) -> &str {
                "custom-card"
            }

            async fn data(&self) -> Result<serde_json::Value, rf_nova::card::CardError> {
                Ok(serde_json::json!({}))
            }
        }
    };

    TokenStream::from(expanded)
}
