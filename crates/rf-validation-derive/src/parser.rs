//! Parser for struct and field attributes
//!
//! This module extracts validation information from derive input.

use crate::rules::ValidationRule;
use syn::{Attribute, Data, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type};

/// Information about a struct to be validated
#[derive(Debug)]
pub struct StructInfo {
    pub name: syn::Ident,
    pub fields: Vec<FieldInfo>,
}

/// Information about a field to be validated
#[derive(Debug)]
#[allow(dead_code)]
pub struct FieldInfo {
    pub name: syn::Ident,
    pub ty: Type,
    pub rules: Vec<ValidationRule>,
    pub custom_message: Option<String>,
    pub is_optional: bool,
}

impl StructInfo {
    /// Parse struct information from DeriveInput
    pub fn from_derive_input(input: &DeriveInput) -> Result<Self, syn::Error> {
        let name = input.ident.clone();

        // Only support structs with named fields
        let fields = match &input.data {
            Data::Struct(data) => match &data.fields {
                Fields::Named(fields) => fields
                    .named
                    .iter()
                    .filter_map(|f| FieldInfo::from_field(f).transpose())
                    .collect::<Result<Vec<_>, _>>()?,
                _ => {
                    return Err(syn::Error::new_spanned(
                        input,
                        "Validate can only be derived for structs with named fields",
                    ))
                }
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "Validate can only be derived for structs",
                ))
            }
        };

        Ok(Self { name, fields })
    }
}

impl FieldInfo {
    /// Parse field information from Field
    /// Returns None if field has no validation attributes
    pub fn from_field(field: &Field) -> Result<Option<Self>, syn::Error> {
        let name = field.ident.as_ref()
            .ok_or_else(|| syn::Error::new_spanned(
                field,
                "Validate derive only supports named struct fields, not tuple fields",
            ))?
            .clone();
        let ty = field.ty.clone();

        // Check if field type is Option<T>
        let is_optional = Self::is_option_type(&ty);

        // Find all #[validate(...)] attributes
        let validate_attrs: Vec<&Attribute> = field
            .attrs
            .iter()
            .filter(|attr| attr.path().is_ident("validate"))
            .collect();

        if validate_attrs.is_empty() {
            return Ok(None);
        }

        let mut all_rules = Vec::new();
        let mut custom_message = None;

        for attr in validate_attrs {
            match &attr.meta {
                syn::Meta::Path(_) => {
                    // #[validate] - nested validation
                    all_rules.push(ValidationRule::Nested);
                }
                syn::Meta::List(_) | syn::Meta::NameValue(_) => {
                    // Parse rules from the attribute
                    let rules = ValidationRule::from_meta(&attr.meta)?;
                    all_rules.extend(rules);
                }
            }

            // Check for custom message in the same attribute
            // #[validate(required, message = "Custom message")]
            if let syn::Meta::List(list) = &attr.meta {
                for nested in list.parse_args_with(
                    syn::punctuated::Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated,
                )? {
                    if let syn::Meta::NameValue(nv) = nested {
                        if nv.path.is_ident("message") {
                            if let syn::Expr::Lit(lit) = &nv.value {
                                if let syn::Lit::Str(s) = &lit.lit {
                                    custom_message = Some(s.value());
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(Some(Self {
            name,
            ty,
            rules: all_rules,
            custom_message,
            is_optional,
        }))
    }

    /// Check if a type is Option<T>
    fn is_option_type(ty: &Type) -> bool {
        if let Type::Path(type_path) = ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    return true;
                }
            }
        }
        false
    }

    /// Extract the inner type from Option<T>
    pub fn inner_type(&self) -> Option<&Type> {
        if !self.is_optional {
            return None;
        }

        if let Type::Path(type_path) = &self.ty {
            if let Some(segment) = type_path.path.segments.last() {
                if segment.ident == "Option" {
                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                            return Some(inner_ty);
                        }
                    }
                }
            }
        }

        None
    }

    /// Check if field has the `required` rule
    pub fn is_required(&self) -> bool {
        self.rules
            .iter()
            .any(|r| matches!(r, ValidationRule::Required))
    }

    /// Check if field is nullable (has `nullable` attribute)
    pub fn is_nullable(&self) -> bool {
        self.rules
            .iter()
            .any(|r| matches!(r, ValidationRule::Nullable))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_parse_simple_struct() {
        let input: DeriveInput = parse_quote! {
            struct CreateUser {
                #[validate(required, email)]
                email: String,

                #[validate(required, min = 8)]
                password: String,
            }
        };

        let info = StructInfo::from_derive_input(&input).unwrap();
        assert_eq!(info.name, "CreateUser");
        assert_eq!(info.fields.len(), 2);
    }

    #[test]
    fn test_optional_field() {
        let input: DeriveInput = parse_quote! {
            struct User {
                #[validate(url)]
                website: Option<String>,
            }
        };

        let info = StructInfo::from_derive_input(&input).unwrap();
        assert_eq!(info.fields.len(), 1);
        assert!(info.fields[0].is_optional);
    }

    #[test]
    fn test_nested_validation() {
        let input: DeriveInput = parse_quote! {
            struct Post {
                #[validate]
                tags: Vec<Tag>,
            }
        };

        let info = StructInfo::from_derive_input(&input).unwrap();
        assert_eq!(info.fields.len(), 1);
        assert!(matches!(
            info.fields[0].rules.first(),
            Some(ValidationRule::Nested)
        ));
    }
}
