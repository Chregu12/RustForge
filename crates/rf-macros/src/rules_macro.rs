use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Expr, Ident, Result, Token,
};

/// Represents a single validation rule like `required`, `min(3)`, `email`
enum Rule {
    Simple(Ident),
    WithArgs(Ident, Vec<Expr>),
}

impl Parse for Rule {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;

        // Check if there are arguments
        if input.peek(syn::token::Paren) {
            let content;
            syn::parenthesized!(content in input);

            let mut args = Vec::new();
            while !content.is_empty() {
                args.push(content.parse()?);
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }

            Ok(Rule::WithArgs(name, args))
        } else {
            Ok(Rule::Simple(name))
        }
    }
}

/// Represents a field with its validation rules: `name: required | min(3)`
struct FieldRules {
    field: Ident,
    rules: Vec<Rule>,
}

impl Parse for FieldRules {
    fn parse(input: ParseStream) -> Result<Self> {
        let field: Ident = input.parse()?;
        input.parse::<Token![:]>()?;

        let mut rules = Vec::new();
        rules.push(input.parse()?);

        while input.peek(Token![|]) {
            input.parse::<Token![|]>()?;
            rules.push(input.parse()?);
        }

        Ok(FieldRules { field, rules })
    }
}

/// Represents the entire rules! macro input
struct RulesInput {
    fields: Vec<FieldRules>,
}

impl Parse for RulesInput {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut fields = Vec::new();

        while !input.is_empty() {
            fields.push(input.parse()?);

            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }

        Ok(RulesInput { fields })
    }
}

pub fn rules_impl(input: TokenStream) -> TokenStream {
    let RulesInput { fields } = parse_macro_input!(input as RulesInput);

    // Generate code for each field's rules
    let field_inserts = fields.iter().map(|field_rule| {
        let field_name = &field_rule.field;
        let field_name_str = field_name.to_string();

        let rules_vec = field_rule.rules.iter().map(|rule| match rule {
            Rule::Simple(name) => {
                let rule_name = name.to_string();
                let rule_ident = Ident::new(
                    &format!("{}Rule", capitalize_first(&rule_name)),
                    name.span(),
                );
                quote! {
                    Box::new(rf_validation::rules::#rule_ident) as Box<dyn rf_validation::Rule>
                }
            }
            Rule::WithArgs(name, args) => {
                let rule_name = name.to_string();
                let rule_ident = Ident::new(
                    &format!("{}Rule", capitalize_first(&rule_name)),
                    name.span(),
                );
                quote! {
                    Box::new(rf_validation::rules::#rule_ident::new(#(#args),*)) as Box<dyn rf_validation::Rule>
                }
            }
        });

        quote! {
            rules_map.insert(#field_name_str, vec![#(#rules_vec),*]);
        }
    });

    let expanded = quote! {
        {
            let mut rules_map: std::collections::HashMap<&'static str, Vec<Box<dyn rf_validation::Rule>>>
                = std::collections::HashMap::new();
            #(#field_inserts)*
            rules_map
        }
    };

    TokenStream::from(expanded)
}

fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_rules_macro_exists() {
        // Proc-macro functions cannot be called directly in unit tests.
        // The macro is tested via compile tests in the integration tests.
        assert!(true);
    }
}
