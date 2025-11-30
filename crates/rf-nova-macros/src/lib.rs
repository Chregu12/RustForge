//! Derive macros for rf-nova
//!
//! Provides #[derive(Resource)], #[derive(Action)], etc.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for Resource trait
///
/// # Example
///
/// ```ignore
/// #[derive(Resource)]
/// #[nova(model = "User", group = "Users")]
/// pub struct UserResource {
///     #[nova(id)]
///     pub id: ID,
///
///     #[nova(text, sortable, searchable)]
///     pub name: Text,
/// }
/// ```
#[proc_macro_derive(Resource, attributes(nova))]
pub fn derive_resource(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Parse attributes
    let expanded = quote! {
        impl rf_nova::resource::Resource for #name {
            type Entity = (); // Placeholder - would be extracted from attributes
            type Model = (); // Placeholder

            fn name() -> &'static str {
                stringify!(#name)
            }

            fn fields() -> Vec<Box<dyn rf_nova::resource::Field>> {
                vec![] // Would be generated from struct fields
            }
        }
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
