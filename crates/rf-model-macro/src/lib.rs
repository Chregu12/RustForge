use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields};
use inflector::Inflector;

/// RustForge model macro - Laravel-like syntax for Eloquent models
///
/// This macro makes defining models as simple as Laravel:
///
/// ```text
/// // Laravel:  class User extends Model
/// // RustForge:
/// #[model]
/// pub struct User {
///     pub name: String,
///     pub email: String,
///     #[hidden]
///     pub password: String,
/// }
/// ```
///
/// Note: this example is shown as `text` because the `#[model]` macro
/// expands to code that depends on `sea_orm` and `rf_db_facade::Model`,
/// neither wired up as a dev-dependency of this proc-macro crate.
///
/// Then use Laravel-style static methods:
///
/// ```text
/// // Find by ID
/// let user = User::find(1).await?;
///
/// // Query with where clause
/// let admins = User::r#where("role", "admin").get().await?;
///
/// // Create new record
/// let user = User::create(json!({
///     "name": "John",
///     "email": "john@example.com"
/// })).await?;
///
/// // Chain queries
/// let active = User::r#where("active", true)
///     .order_by("name", "asc")
///     .limit(10)
///     .get().await?;
/// ```
///
/// Automatically adds:
/// - `id: i32` primary key field (if not present)
/// - `created_at: DateTime<Utc>` (if not present)
/// - `updated_at: DateTime<Utc>` (if not present)
/// - All necessary derives (DeriveEntityModel, Serialize, Deserialize, etc.)
/// - SeaORM table name (pluralized struct name)
/// - Converts `#[hidden]` to `#[serde(skip_serializing)]`
/// - Implements `rf_db_facade::Model` for static query methods
#[proc_macro_attribute]
pub fn model(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let name = &input.ident;
    let vis = &input.vis;
    let generics = &input.generics;

    // Get table name (pluralized lowercase)
    let table_name = name.to_string().to_table_case().to_plural();

    // Process struct fields
    let fields = match &input.data {
        Data::Struct(data) => {
            match &data.fields {
                Fields::Named(fields) => &fields.named,
                _ => panic!("#[model] only supports structs with named fields"),
            }
        }
        _ => panic!("#[model] only supports structs"),
    };

    // Check which standard fields exist
    let has_id = fields.iter().any(|f| f.ident.as_ref().unwrap() == "id");
    let has_created_at = fields.iter().any(|f| f.ident.as_ref().unwrap() == "created_at");
    let has_updated_at = fields.iter().any(|f| f.ident.as_ref().unwrap() == "updated_at");

    // Process existing fields and convert #[hidden] to #[serde(skip_serializing)]
    let processed_fields: Vec<_> = fields.iter().map(|field| {
        let field_name = &field.ident;
        let field_type = &field.ty;
        let field_vis = &field.vis;

        // Check for #[hidden] attribute
        let has_hidden = field.attrs.iter().any(|attr| {
            attr.path().is_ident("hidden")
        });

        // Filter out #[hidden] from original attributes
        let other_attrs: Vec<_> = field.attrs.iter()
            .filter(|attr| !attr.path().is_ident("hidden"))
            .collect();

        // Check if this is the id field
        let is_id = field_name.as_ref().unwrap() == "id";

        if has_hidden {
            quote! {
                #(#other_attrs)*
                #[serde(skip_serializing)]
                #field_vis #field_name: #field_type
            }
        } else if is_id {
            quote! {
                #(#other_attrs)*
                #[sea_orm(primary_key)]
                #field_vis #field_name: #field_type
            }
        } else {
            quote! {
                #(#other_attrs)*
                #field_vis #field_name: #field_type
            }
        }
    }).collect();

    // Add standard fields if missing
    let id_field = if !has_id {
        quote! {
            #[sea_orm(primary_key)]
            pub id: i32,
        }
    } else {
        quote! {}
    };

    let created_at_field = if !has_created_at {
        quote! {
            pub created_at: chrono::DateTime<chrono::Utc>,
        }
    } else {
        quote! {}
    };

    let updated_at_field = if !has_updated_at {
        quote! {
            pub updated_at: chrono::DateTime<chrono::Utc>,
        }
    } else {
        quote! {}
    };

    // Generate the expanded model
    let expanded = quote! {
        #[derive(Clone, Debug, PartialEq, Eq, sea_orm::DeriveEntityModel, serde::Serialize, serde::Deserialize)]
        #[sea_orm(table_name = #table_name)]
        #vis struct #name #generics {
            #id_field
            #(#processed_fields,)*
            #created_at_field
            #updated_at_field
        }

        #[derive(Copy, Clone, Debug, sea_orm::EnumIter, sea_orm::DeriveRelation)]
        pub enum Relation {}

        impl sea_orm::ActiveModelBehavior for ActiveModel {}

        // Implement rf_db_facade::Model trait for Laravel-style static methods
        // This enables User::where(), User::find(), User::create(), etc.
        impl rf_db_facade::Model for #name {
            const TABLE: &'static str = #table_name;
        }
    };

    TokenStream::from(expanded)
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
