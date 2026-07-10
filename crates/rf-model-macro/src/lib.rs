use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

/// DEPRECATED — use the `Model!()` DSL from `rf::prelude` instead.
///
/// `#[model]` is incompatible with SeaORM's `DeriveEntityModel`, which
/// requires the struct to be named exactly `Model`.  Applying `#[model]` to
/// any other name (e.g. `pub struct User`) caused a compile-time panic with
/// the message "Struct name must be Model".
///
/// Every scaffold emitted by `foundry make:model` previously used this macro
/// and was therefore uncompilable against the current framework.
///
/// # Migration
///
/// Replace:
/// ```text
/// #[model]
/// pub struct User {
///     pub name: String,
///     pub email: String,
/// }
/// ```
///
/// With the canonical `Model!()` DSL (from `rf::prelude`):
/// ```text
/// use rf::prelude::*;
/// Model!(User {
///     name: String,
///     email: String,
/// });
/// ```
///
/// The `Model!()` macro handles the inner SeaORM entity wiring correctly and
/// is the tested pattern used by taskflow / blog-slice / rest-crud-resource.
/// See the `rf-macros` docs for relations, scopes, and validated fields.
#[proc_macro_attribute]
pub fn model(_attr: TokenStream, item: TokenStream) -> TokenStream {
    // -----------------------------------------------------------------------
    // DEPRECATED — `#[model]` is incompatible with SeaORM.
    //
    // SeaORM's `DeriveEntityModel` requires the struct to be named `Model`,
    // but `#[model]` was applying it to the user-named struct (e.g. `pub struct
    // User`), which caused a compile-time panic: "Struct name must be Model".
    // Every generated scaffold using `#[model]` was therefore uncompilable.
    //
    // Use the canonical `Model!()` DSL from `rf::prelude` instead, which
    // handles the inner SeaORM entity wiring correctly and is the tested
    // pattern used by taskflow / blog-slice / rest-crud-resource:
    //
    //     use rf::prelude::*;
    //     Model!(User {
    //         name: String,
    //         email: String,
    //     });
    //
    // See the rf-macros docs for relations, scopes, and validations.
    // -----------------------------------------------------------------------
    let item_copy: proc_macro2::TokenStream = item.clone().into();
    let input = parse_macro_input!(item as DeriveInput);
    let name = &input.ident;

    let name_str = name.to_string();
    let msg = format!(
        "`#[model]` is not compatible with SeaORM: `DeriveEntityModel` requires the struct \
         to be named `Model`, but found `{name_str}`. \
         Use the canonical `Model!()` DSL from `rf::prelude` instead:\n\n  \
         use rf::prelude::*;\n\n  \
         Model!({name_str} {{\n      \
         // name: String,\n  \
         }});\n\n\
         See the rf-macros docs for relations, scopes, and validations."
    );
    let err = syn::Error::new(name.span(), msg);
    let error_tokens = err.to_compile_error();
    // Re-emit the original struct (without the broken macro derives) so the
    // type name stays resolvable in the same compilation unit.  The
    // compile_error above still prevents linking.
    quote::quote! {
        #error_tokens
        #item_copy
    }
    .into()
}

/// Relations macro for defining model relationships.
///
/// Annotate an `impl` block whose methods return relationship builders. The
/// macro **preserves your method bodies verbatim** so the accessors you write
/// are emitted and callable — it no longer discards them.
///
/// Each recognized relationship method (return type `HasMany<T>` / `HasOne<T>` /
/// `BelongsTo<T>`) additionally gets a generated `<method>_kind()` companion
/// returning the [`RelationshipKind`](rf_eloquent::relationships::RelationshipKind)
/// name as a `&'static str`, for lightweight introspection.
///
/// # Example
/// ```text
/// #[relations]
/// impl User {
///     // Body is preserved and callable.
///     fn posts(&self) -> HasManyRef<post::Entity> {
///         HasManyRef::new(post::Column::UserId, self.id)
///     }
/// }
///
/// // Generated companion:
/// //   User::posts_kind() == "HasMany"
/// ```
///
/// Note: automatically deriving a SeaORM `Relation` enum + `Related` impls from
/// method signatures alone is intentionally out of scope — foreign-key columns
/// and join tables cannot be inferred from `HasMany<T>` return types, so the
/// macro delegates the actual query construction to the method bodies (which
/// typically use `rf_eloquent`'s `HasManyRef`/`has_many()` helpers).
#[proc_macro_attribute]
pub fn relations(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemImpl);

    // Collect (method_name, relationship_kind_str) for recognized accessors so
    // we can emit lightweight introspection companions WITHOUT dropping bodies.
    let mut kind_companions: Vec<proc_macro2::TokenStream> = Vec::new();

    for it in &input.items {
        if let syn::ImplItem::Fn(method) = it {
            if let syn::ReturnType::Type(_, ty) = &method.sig.output {
                let type_str = quote!(#ty).to_string();
                // Order matters: "BelongsToMany" contains "BelongsTo"; check longest first.
                let kind = if type_str.contains("BelongsToMany") {
                    Some("BelongsToMany")
                } else if type_str.contains("HasManyThrough") {
                    Some("HasManyThrough")
                } else if type_str.contains("HasOneThrough") {
                    Some("HasOneThrough")
                } else if type_str.contains("HasMany") {
                    Some("HasMany")
                } else if type_str.contains("HasOne") {
                    Some("HasOne")
                } else if type_str.contains("BelongsTo") {
                    Some("BelongsTo")
                } else {
                    None
                };

                if let Some(kind) = kind {
                    let companion = syn::Ident::new(
                        &format!("{}_kind", method.sig.ident),
                        method.sig.ident.span(),
                    );
                    kind_companions.push(quote! {
                        /// Relationship kind for the like-named accessor (generated by `#[relations]`).
                        pub fn #companion() -> &'static str { #kind }
                    });
                }
            }
        }
    }

    let self_ty = &input.self_ty;
    let (impl_generics, _ty_generics, where_clause) = input.generics.split_for_impl();

    // Re-emit the ORIGINAL impl block unchanged (preserving all method bodies),
    // then add the generated introspection companions in a sibling impl block.
    let expanded = quote! {
        #input

        impl #impl_generics #self_ty #where_clause {
            #(#kind_companions)*
        }
    };

    TokenStream::from(expanded)
}
