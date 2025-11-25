//! Laravel-style helper macros
//!
//! Provides familiar Laravel helper functions and macros:
//!
//! ```rust,ignore
//! // Collection helper
//! let collection = collect![1, 2, 3, 4, 5];
//! let doubled = collection.map(|x| x * 2);
//!
//! // Config helper
//! let db_host = config!("database.host");
//!
//! // Environment helper
//! let app_env = env!("APP_ENV", "production");
//!
//! // Route helper
//! let url = route!("users.show", id = 123);
//!
//! // Response helpers
//! return response!(json: data);
//! return response!(redirect: "/home");
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse::{Parse, ParseStream}, parse_macro_input, punctuated::Punctuated, Expr, Ident, LitStr, Token};

/// Parse collect! arguments: collect![1, 2, 3]
struct CollectArgs {
    items: Punctuated<Expr, Token![,]>,
}

impl Parse for CollectArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let items = Punctuated::parse_terminated(input)?;
        Ok(CollectArgs { items })
    }
}

/// Implements the collect! macro for creating Laravel-style collections
pub fn collect_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as CollectArgs);
    let items = args.items.iter();

    let expanded = quote! {
        {
            let items = vec![#(#items),*];
            rf_collection::Collection::new(items)
        }
    };

    TokenStream::from(expanded)
}

/// Parse config! arguments: config!("key") or config!("key", default)
struct ConfigArgs {
    key: LitStr,
    default: Option<Expr>,
}

impl Parse for ConfigArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;
        let default = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(ConfigArgs { key, default })
    }
}

/// Implements the config! macro for accessing configuration values
pub fn config_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ConfigArgs);
    let key = &args.key;

    let expanded = if let Some(default) = args.default {
        quote! {
            rf_config::Config::get(#key).unwrap_or_else(|| #default.into())
        }
    } else {
        quote! {
            rf_config::Config::get(#key)
        }
    };

    TokenStream::from(expanded)
}

/// Parse env_helper! arguments: env_helper!("KEY") or env_helper!("KEY", "default")
struct EnvArgs {
    key: LitStr,
    default: Option<LitStr>,
}

impl Parse for EnvArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;
        let default = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(EnvArgs { key, default })
    }
}

/// Implements the env_helper! macro for environment variables
pub fn env_helper_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as EnvArgs);
    let key = &args.key;

    let expanded = if let Some(default) = args.default {
        quote! {
            std::env::var(#key).unwrap_or_else(|_| #default.to_string())
        }
    } else {
        quote! {
            std::env::var(#key).ok()
        }
    };

    TokenStream::from(expanded)
}

/// Parse route! arguments: route!("name") or route!("name", param = value)
struct RouteArgs {
    name: LitStr,
    params: Vec<(Ident, Expr)>,
}

impl Parse for RouteArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: LitStr = input.parse()?;
        let mut params = Vec::new();

        while input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            if input.is_empty() {
                break;
            }
            let param_name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let param_value: Expr = input.parse()?;
            params.push((param_name, param_value));
        }

        Ok(RouteArgs { name, params })
    }
}

/// Implements the route! macro for generating named route URLs
pub fn route_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as RouteArgs);
    let name = &args.name;

    let expanded = if args.params.is_empty() {
        quote! {
            rf_route_facade::Route::url(#name, &[])
        }
    } else {
        let param_array: Vec<TokenStream2> = args.params.iter().map(|(k, v)| {
            let key_str = k.to_string();
            quote! { (#key_str, &#v.to_string()) }
        }).collect();

        quote! {
            rf_route_facade::Route::url(#name, &[#(#param_array),*])
        }
    };

    TokenStream::from(expanded)
}

/// Parse response! arguments
enum ResponseType {
    Json(Expr),
    Text(Expr),
    Redirect(Expr),
    View(LitStr, Option<Expr>),
    Status(Expr),
    Download(Expr),
    File(Expr),
}

struct ResponseArgs {
    response_type: ResponseType,
}

impl Parse for ResponseArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let type_name: Ident = input.parse()?;
        input.parse::<Token![:]>()?;

        let response_type = match type_name.to_string().as_str() {
            "json" => ResponseType::Json(input.parse()?),
            "text" => ResponseType::Text(input.parse()?),
            "redirect" => ResponseType::Redirect(input.parse()?),
            "view" => {
                let name: LitStr = input.parse()?;
                let data = if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                    Some(input.parse()?)
                } else {
                    None
                };
                ResponseType::View(name, data)
            }
            "status" => ResponseType::Status(input.parse()?),
            "download" => ResponseType::Download(input.parse()?),
            "file" => ResponseType::File(input.parse()?),
            other => return Err(syn::Error::new(
                type_name.span(),
                format!("Unknown response type: {}. Use json, text, redirect, view, status, download, or file", other)
            )),
        };

        Ok(ResponseArgs { response_type })
    }
}

/// Implements the response! macro for creating various response types
pub fn response_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as ResponseArgs);

    let expanded = match args.response_type {
        ResponseType::Json(data) => quote! {
            rf_response::Response::json(#data)
        },
        ResponseType::Text(text) => quote! {
            rf_response::Response::text(#text)
        },
        ResponseType::Redirect(url) => quote! {
            rf_response::Response::redirect(#url)
        },
        ResponseType::View(name, data) => {
            if let Some(data) = data {
                quote! {
                    rf_response::Response::view(#name, #data)
                }
            } else {
                quote! {
                    rf_response::Response::view(#name, serde_json::json!({}))
                }
            }
        },
        ResponseType::Status(code) => quote! {
            rf_response::Response::status(#code)
        },
        ResponseType::Download(path) => quote! {
            rf_response::Response::download(#path)
        },
        ResponseType::File(path) => quote! {
            rf_response::Response::file(#path)
        },
    };

    TokenStream::from(expanded)
}

/// Parse abort! arguments: abort!(404) or abort!(404, "Not found")
struct AbortArgs {
    code: Expr,
    message: Option<Expr>,
}

impl Parse for AbortArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let code: Expr = input.parse()?;
        let message = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(AbortArgs { code, message })
    }
}

/// Implements the abort! macro for HTTP error responses
pub fn abort_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as AbortArgs);
    let code = &args.code;

    let expanded = if let Some(message) = args.message {
        quote! {
            return rf_response::Response::error(#code, #message)
        }
    } else {
        quote! {
            return rf_response::Response::status(#code)
        }
    };

    TokenStream::from(expanded)
}

/// Parse dd! (dump and die) arguments
struct DdArgs {
    values: Punctuated<Expr, Token![,]>,
}

impl Parse for DdArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let values = Punctuated::parse_terminated(input)?;
        Ok(DdArgs { values })
    }
}

/// Implements the dd! macro (dump and die) for debugging
pub fn dd_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DdArgs);
    let values = args.values.iter();

    let debug_stmts: Vec<TokenStream2> = args.values.iter().enumerate().map(|(i, v)| {
        let v_str = quote!(#v).to_string();
        quote! {
            eprintln!("[{}] {} = {:#?}", #i, #v_str, #v);
        }
    }).collect();

    let expanded = quote! {
        {
            eprintln!("=== DD (Dump & Die) ===");
            #(#debug_stmts)*
            eprintln!("=======================");
            std::process::exit(1);
        }
    };

    TokenStream::from(expanded)
}

/// Implements the dump! macro for debugging without stopping
pub fn dump_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as DdArgs);

    let debug_stmts: Vec<TokenStream2> = args.values.iter().enumerate().map(|(i, v)| {
        let v_str = quote!(#v).to_string();
        quote! {
            eprintln!("[{}] {} = {:#?}", #i, #v_str, #v);
        }
    }).collect();

    let expanded = quote! {
        {
            eprintln!("=== DUMP ===");
            #(#debug_stmts)*
            eprintln!("============");
        }
    };

    TokenStream::from(expanded)
}

/// Parse old! arguments for form data: old!("field") or old!("field", "default")
struct OldArgs {
    field: LitStr,
    default: Option<Expr>,
}

impl Parse for OldArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let field: LitStr = input.parse()?;
        let default = if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(OldArgs { field, default })
    }
}

/// Implements the old! macro for retrieving old form input
pub fn old_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as OldArgs);
    let field = &args.field;

    let expanded = if let Some(default) = args.default {
        quote! {
            rf_session::Session::get_old(#field).unwrap_or_else(|| #default.to_string())
        }
    } else {
        quote! {
            rf_session::Session::get_old(#field).unwrap_or_default()
        }
    };

    TokenStream::from(expanded)
}

/// Implements the asset! macro for asset URLs
pub fn asset_impl(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);

    let expanded = quote! {
        format!("/assets/{}", #path.trim_start_matches('/'))
    };

    TokenStream::from(expanded)
}

/// Implements the url! macro for full URLs
pub fn url_impl(input: TokenStream) -> TokenStream {
    let path = parse_macro_input!(input as LitStr);

    let expanded = quote! {
        rf_config::Config::get("app.url")
            .map(|base: String| format!("{}/{}", base.trim_end_matches('/'), #path.trim_start_matches('/')))
            .unwrap_or_else(|| format!("/{}", #path.trim_start_matches('/')))
    };

    TokenStream::from(expanded)
}
