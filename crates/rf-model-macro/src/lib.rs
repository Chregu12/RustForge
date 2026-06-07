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

/// Relations macro for defining model relationships
///
/// # Example
/// ```text
/// #[relations]
/// impl User {
///     fn posts() -> HasMany<Post> {
///         self.has_many()
///     }
///
///     fn profile() -> HasOne<Profile> {
///         self.has_one()
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn relations(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as syn::ItemImpl);

    let _model_name = &input.self_ty;
    let items = &input.items;

    // Extract relation definitions from methods
    let mut has_many_relations = Vec::new();
    let mut has_one_relations = Vec::new();
    let mut belongs_to_relations = Vec::new();

    for item in items {
        if let syn::ImplItem::Fn(method) = item {
            let method_name = &method.sig.ident;
            let return_type = &method.sig.output;

            // Parse return type to detect relation type
            if let syn::ReturnType::Type(_, ty) = return_type {
                let type_str = quote!(#ty).to_string();

                if type_str.contains("HasMany") {
                    // Extract the related model from HasMany<Post>
                    if let syn::Type::Path(type_path) = &**ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(related_model)) = args.args.first() {
                                    has_many_relations.push((method_name.clone(), related_model.clone()));
                                }
                            }
                        }
                    }
                } else if type_str.contains("HasOne") {
                    if let syn::Type::Path(type_path) = &**ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(related_model)) = args.args.first() {
                                    has_one_relations.push((method_name.clone(), related_model.clone()));
                                }
                            }
                        }
                    }
                } else if type_str.contains("BelongsTo") {
                    if let syn::Type::Path(type_path) = &**ty {
                        if let Some(segment) = type_path.path.segments.last() {
                            if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                                if let Some(syn::GenericArgument::Type(related_model)) = args.args.first() {
                                    belongs_to_relations.push((method_name.clone(), related_model.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Generate relation enum variants
    let relation_variants: Vec<_> = has_many_relations.iter().map(|(name, _model)| {
        let variant_name = syn::Ident::new(&name.to_string().to_pascal_case(), name.span());
        quote! {
            #[sea_orm(has_many = "super::#_model::Entity")]
            #variant_name
        }
    }).chain(has_one_relations.iter().map(|(name, _model)| {
        let variant_name = syn::Ident::new(&name.to_string().to_pascal_case(), name.span());
        quote! {
            #[sea_orm(has_one = "super::#_model::Entity")]
            #variant_name
        }
    })).chain(belongs_to_relations.iter().map(|(name, _model)| {
        let variant_name = syn::Ident::new(&name.to_string().to_pascal_case(), name.span());
        quote! {
            #[sea_orm(belongs_to = "super::#_model::Entity")]
            #variant_name
        }
    })).collect();

    // Generate Related implementations
    let related_impls: Vec<_> = has_many_relations.iter().map(|(_, model)| {
        quote! {
            impl sea_orm::Related<super::#model::Entity> for Entity {
                fn to() -> sea_orm::RelationDef {
                    Relation::#model.def()
                }
            }
        }
    }).chain(has_one_relations.iter().map(|(_, model)| {
        quote! {
            impl sea_orm::Related<super::#model::Entity> for Entity {
                fn to() -> sea_orm::RelationDef {
                    Relation::#model.def()
                }
            }
        }
    })).chain(belongs_to_relations.iter().map(|(_, model)| {
        quote! {
            impl sea_orm::Related<super::#model::Entity> for Entity {
                fn to() -> sea_orm::RelationDef {
                    Relation::#model.def()
                }
            }
        }
    })).collect();

    let expanded = quote! {
        #[derive(Copy, Clone, Debug, sea_orm::EnumIter, sea_orm::DeriveRelation)]
        pub enum Relation {
            #(#relation_variants,)*
        }

        #(#related_impls)*
    };

    TokenStream::from(expanded)
}
