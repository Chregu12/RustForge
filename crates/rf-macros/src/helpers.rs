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
    let _values = args.values.iter();

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

// =============================================================================
// Additional Laravel Helper Macros
// =============================================================================

/// Implements the now! macro for current datetime
///
/// ```rust,ignore
/// let current = now!();                    // Current datetime
/// let formatted = now!("%Y-%m-%d");        // Formatted string
/// let with_tz = now!("America/New_York");  // With timezone
/// ```
pub fn now_impl(input: TokenStream) -> TokenStream {
    let expanded = if input.is_empty() {
        quote! {
            chrono::Utc::now()
        }
    } else {
        let format = parse_macro_input!(input as LitStr);
        let format_str = format.value();

        // Check if it's a timezone or format string
        if format_str.contains('%') {
            quote! {
                chrono::Utc::now().format(#format).to_string()
            }
        } else {
            // Assume it's a timezone - for now just use UTC
            quote! {
                chrono::Utc::now()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Implements the bcrypt! macro for password hashing
///
/// ```rust,ignore
/// let hashed = bcrypt!(password);           // Hash with default cost
/// let hashed = bcrypt!(password, 12);       // Hash with custom cost
/// let valid = bcrypt!(verify: password, hash);  // Verify password
/// ```
pub fn bcrypt_impl(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.clone().into();

    // Check for verify: prefix
    let input_str = input2.to_string();
    if input_str.starts_with("verify") {
        // Parse verify: password, hash
        struct VerifyArgs {
            _verify: Ident,
            _colon: Token![:],
            password: Expr,
            _comma: Token![,],
            hash: Expr,
        }

        impl Parse for VerifyArgs {
            fn parse(input: ParseStream) -> syn::Result<Self> {
                Ok(VerifyArgs {
                    _verify: input.parse()?,
                    _colon: input.parse()?,
                    password: input.parse()?,
                    _comma: input.parse()?,
                    hash: input.parse()?,
                })
            }
        }

        let args = parse_macro_input!(input as VerifyArgs);
        let password = &args.password;
        let hash = &args.hash;

        let expanded = quote! {
            bcrypt::verify(#password, #hash).unwrap_or(false)
        };
        return TokenStream::from(expanded);
    }

    // Parse: bcrypt!(password) or bcrypt!(password, cost)
    struct HashArgs {
        password: Expr,
        cost: Option<(Token![,], Expr)>,
    }

    impl Parse for HashArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let password = input.parse()?;
            let cost = if input.peek(Token![,]) {
                Some((input.parse()?, input.parse()?))
            } else {
                None
            };
            Ok(HashArgs { password, cost })
        }
    }

    let args = parse_macro_input!(input as HashArgs);
    let password = &args.password;

    let expanded = if let Some((_, cost)) = args.cost {
        quote! {
            bcrypt::hash(#password, #cost).expect("Failed to hash password")
        }
    } else {
        quote! {
            bcrypt::hash(#password, bcrypt::DEFAULT_COST).expect("Failed to hash password")
        }
    };

    TokenStream::from(expanded)
}

/// Implements the back! macro for redirect back
///
/// ```rust,ignore
/// return back!();                    // Redirect to previous URL
/// return back!("/fallback");         // With fallback URL
/// return back!().with("message", "Success!");  // With flash data
/// ```
pub fn back_impl(input: TokenStream) -> TokenStream {
    let expanded = if input.is_empty() {
        quote! {
            rf_response::Response::back()
        }
    } else {
        let fallback = parse_macro_input!(input as LitStr);
        quote! {
            rf_response::Response::back_or(#fallback)
        }
    };

    TokenStream::from(expanded)
}

/// Implements the view! macro for rendering views
///
/// ```rust,ignore
/// return view!("welcome");                    // Simple view
/// return view!("users.index", users);         // View with data
/// return view!("posts.show", { title, body }); // View with multiple vars
/// ```
pub fn view_impl(input: TokenStream) -> TokenStream {
    struct ViewArgs {
        name: LitStr,
        data: Option<Expr>,
    }

    impl Parse for ViewArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let name: LitStr = input.parse()?;
            let data = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok(ViewArgs { name, data })
        }
    }

    let args = parse_macro_input!(input as ViewArgs);
    let name = &args.name;

    let expanded = if let Some(data) = args.data {
        quote! {
            rf_response::Response::view(#name, #data)
        }
    } else {
        quote! {
            rf_response::Response::view(#name, serde_json::json!({}))
        }
    };

    TokenStream::from(expanded)
}

/// Implements the redirect! macro for redirects
///
/// ```rust,ignore
/// return redirect!("/home");
/// return redirect!("/users/{}", user_id);
/// return redirect!(route: "users.show", id = 1);
/// ```
pub fn redirect_impl(input: TokenStream) -> TokenStream {
    struct RedirectArgs {
        target: RedirectTarget,
    }

    enum RedirectTarget {
        Url(LitStr),
        Formatted(LitStr, Punctuated<Expr, Token![,]>),
        Route(LitStr, Vec<(Ident, Expr)>),
    }

    impl Parse for RedirectArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            // Check for route: prefix
            if input.peek(Ident) {
                let lookahead: Ident = input.fork().parse()?;
                if lookahead == "route" {
                    input.parse::<Ident>()?;  // consume "route"
                    input.parse::<Token![:]>()?;
                    let name: LitStr = input.parse()?;
                    let mut params = Vec::new();
                    while input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                        if input.is_empty() { break; }
                        let param_name: Ident = input.parse()?;
                        input.parse::<Token![=]>()?;
                        let param_value: Expr = input.parse()?;
                        params.push((param_name, param_value));
                    }
                    return Ok(RedirectArgs { target: RedirectTarget::Route(name, params) });
                }
            }

            let url: LitStr = input.parse()?;

            // Check for format arguments
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                let args = Punctuated::parse_terminated(input)?;
                Ok(RedirectArgs { target: RedirectTarget::Formatted(url, args) })
            } else {
                Ok(RedirectArgs { target: RedirectTarget::Url(url) })
            }
        }
    }

    let args = parse_macro_input!(input as RedirectArgs);

    let expanded = match args.target {
        RedirectTarget::Url(url) => {
            quote! {
                rf_response::Response::redirect(#url)
            }
        }
        RedirectTarget::Formatted(format_str, args) => {
            let args_iter = args.iter();
            quote! {
                rf_response::Response::redirect(&format!(#format_str, #(#args_iter),*))
            }
        }
        RedirectTarget::Route(name, params) => {
            if params.is_empty() {
                quote! {
                    rf_response::Response::redirect(&rf_route_facade::Route::url(#name, &[]))
                }
            } else {
                let param_array: Vec<TokenStream2> = params.iter().map(|(k, v)| {
                    let key_str = k.to_string();
                    quote! { (#key_str, &#v.to_string()) }
                }).collect();
                quote! {
                    rf_response::Response::redirect(&rf_route_facade::Route::url(#name, &[#(#param_array),*]))
                }
            }
        }
    };

    TokenStream::from(expanded)
}

/// Implements the session! macro for session access
///
/// ```rust,ignore
/// let value = session!("key");             // Get value
/// session!(set: "key", value);             // Set value
/// session!(forget: "key");                 // Remove value
/// session!(flash: "message", "Success!");  // Flash data
/// ```
pub fn session_impl(input: TokenStream) -> TokenStream {
    struct SessionArgs {
        action: SessionAction,
    }

    enum SessionAction {
        Get(LitStr, Option<Expr>),
        Set(LitStr, Expr),
        Forget(LitStr),
        Flash(LitStr, Expr),
        Has(LitStr),
        All,
        Flush,
    }

    impl Parse for SessionArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            // Check for action prefix
            if input.peek(Ident) {
                let lookahead: Ident = input.fork().parse()?;
                let action_str = lookahead.to_string();

                match action_str.as_str() {
                    "set" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let value: Expr = input.parse()?;
                        return Ok(SessionArgs { action: SessionAction::Set(key, value) });
                    }
                    "forget" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        return Ok(SessionArgs { action: SessionAction::Forget(key) });
                    }
                    "flash" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let value: Expr = input.parse()?;
                        return Ok(SessionArgs { action: SessionAction::Flash(key, value) });
                    }
                    "has" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        return Ok(SessionArgs { action: SessionAction::Has(key) });
                    }
                    "all" => {
                        input.parse::<Ident>()?;
                        return Ok(SessionArgs { action: SessionAction::All });
                    }
                    "flush" => {
                        input.parse::<Ident>()?;
                        return Ok(SessionArgs { action: SessionAction::Flush });
                    }
                    _ => {}
                }
            }

            // Default: get with optional default
            let key: LitStr = input.parse()?;
            let default = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok(SessionArgs { action: SessionAction::Get(key, default) })
        }
    }

    let args = parse_macro_input!(input as SessionArgs);

    let expanded = match args.action {
        SessionAction::Get(key, default) => {
            if let Some(def) = default {
                quote! {
                    rf_session::Session::get(#key).unwrap_or_else(|| #def)
                }
            } else {
                quote! {
                    rf_session::Session::get(#key)
                }
            }
        }
        SessionAction::Set(key, value) => {
            quote! {
                rf_session::Session::put(#key, #value)
            }
        }
        SessionAction::Forget(key) => {
            quote! {
                rf_session::Session::forget(#key)
            }
        }
        SessionAction::Flash(key, value) => {
            quote! {
                rf_session::Session::flash(#key, #value)
            }
        }
        SessionAction::Has(key) => {
            quote! {
                rf_session::Session::has(#key)
            }
        }
        SessionAction::All => {
            quote! {
                rf_session::Session::all()
            }
        }
        SessionAction::Flush => {
            quote! {
                rf_session::Session::flush()
            }
        }
    };

    TokenStream::from(expanded)
}

/// Implements the auth! macro for authentication helpers
///
/// ```rust,ignore
/// let user = auth!();              // Get current user
/// let id = auth!(id);              // Get user ID
/// if auth!(check) { ... }          // Check if logged in
/// if auth!(guest) { ... }          // Check if guest
/// auth!(login: user);              // Log in user
/// auth!(logout);                   // Log out
/// ```
pub fn auth_impl(input: TokenStream) -> TokenStream {
    if input.is_empty() {
        return TokenStream::from(quote! {
            rf_auth_facade::Auth::user::<serde_json::Value>()
        });
    }

    struct AuthArgs {
        action: AuthAction,
    }

    enum AuthAction {
        User,
        Id,
        Check,
        Guest,
        Login(Expr),
        Logout,
        Attempt(Expr),
    }

    impl Parse for AuthArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.peek(Ident) {
                let action: Ident = input.parse()?;
                let action_str = action.to_string();

                match action_str.as_str() {
                    "id" => return Ok(AuthArgs { action: AuthAction::Id }),
                    "check" => return Ok(AuthArgs { action: AuthAction::Check }),
                    "guest" => return Ok(AuthArgs { action: AuthAction::Guest }),
                    "logout" => return Ok(AuthArgs { action: AuthAction::Logout }),
                    "login" => {
                        input.parse::<Token![:]>()?;
                        let user: Expr = input.parse()?;
                        return Ok(AuthArgs { action: AuthAction::Login(user) });
                    }
                    "attempt" => {
                        input.parse::<Token![:]>()?;
                        let credentials: Expr = input.parse()?;
                        return Ok(AuthArgs { action: AuthAction::Attempt(credentials) });
                    }
                    _ => {}
                }
            }

            Ok(AuthArgs { action: AuthAction::User })
        }
    }

    let args = parse_macro_input!(input as AuthArgs);

    let expanded = match args.action {
        AuthAction::User => quote! {
            rf_auth_facade::Auth::user::<serde_json::Value>()
        },
        AuthAction::Id => quote! {
            rf_auth_facade::Auth::id()
        },
        AuthAction::Check => quote! {
            rf_auth_facade::Auth::check()
        },
        AuthAction::Guest => quote! {
            !rf_auth_facade::Auth::check()
        },
        AuthAction::Login(user) => quote! {
            rf_auth_facade::Auth::login(#user)
        },
        AuthAction::Logout => quote! {
            rf_auth_facade::Auth::logout()
        },
        AuthAction::Attempt(credentials) => quote! {
            rf_auth_facade::Auth::attempt(#credentials)
        },
    };

    TokenStream::from(expanded)
}

/// Implements the csrf! macro for CSRF token
///
/// ```rust,ignore
/// let token = csrf!();           // Get token
/// csrf!(field);                  // Hidden input field HTML
/// csrf!(meta);                   // Meta tag HTML
/// ```
pub fn csrf_impl(input: TokenStream) -> TokenStream {
    if input.is_empty() {
        return TokenStream::from(quote! {
            rf_session::Session::csrf_token()
        });
    }

    let action: Ident = parse_macro_input!(input as Ident);
    let action_str = action.to_string();

    let expanded = match action_str.as_str() {
        "field" => quote! {
            format!(r#"<input type="hidden" name="_token" value="{}">"#, rf_session::Session::csrf_token())
        },
        "meta" => quote! {
            format!(r#"<meta name="csrf-token" content="{}">"#, rf_session::Session::csrf_token())
        },
        "token" => quote! {
            rf_session::Session::csrf_token()
        },
        _ => quote! {
            rf_session::Session::csrf_token()
        },
    };

    TokenStream::from(expanded)
}

/// Implements the cache! macro for caching
///
/// ```rust,ignore
/// let value = cache!("key");                    // Get from cache
/// cache!(put: "key", value, 3600);              // Cache for 1 hour
/// cache!(forever: "key", value);                // Cache forever
/// cache!(forget: "key");                        // Remove from cache
/// let value = cache!(remember: "key", 3600, || { ... }); // Remember pattern
/// ```
pub fn cache_impl(input: TokenStream) -> TokenStream {
    struct CacheArgs {
        action: CacheAction,
    }

    enum CacheAction {
        Get(LitStr, Option<Expr>),
        Put(LitStr, Expr, Expr),
        Forever(LitStr, Expr),
        Forget(LitStr),
        Has(LitStr),
        Flush,
        Remember(LitStr, Expr, Expr),      // key, ttl, closure
        RememberForever(LitStr, Expr),     // key, closure
    }

    impl Parse for CacheArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.peek(Ident) {
                let lookahead: Ident = input.fork().parse()?;
                let action_str = lookahead.to_string();

                match action_str.as_str() {
                    "put" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let value: Expr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let ttl: Expr = input.parse()?;
                        return Ok(CacheArgs { action: CacheAction::Put(key, value, ttl) });
                    }
                    "forever" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let value: Expr = input.parse()?;
                        return Ok(CacheArgs { action: CacheAction::Forever(key, value) });
                    }
                    "forget" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        return Ok(CacheArgs { action: CacheAction::Forget(key) });
                    }
                    "has" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        return Ok(CacheArgs { action: CacheAction::Has(key) });
                    }
                    "flush" => {
                        input.parse::<Ident>()?;
                        return Ok(CacheArgs { action: CacheAction::Flush });
                    }
                    "remember" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let ttl: Expr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let closure: Expr = input.parse()?;
                        return Ok(CacheArgs { action: CacheAction::Remember(key, ttl, closure) });
                    }
                    "remember_forever" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let key: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let closure: Expr = input.parse()?;
                        return Ok(CacheArgs { action: CacheAction::RememberForever(key, closure) });
                    }
                    _ => {}
                }
            }

            // Default: get with optional default
            let key: LitStr = input.parse()?;
            let default = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Some(input.parse()?)
            } else {
                None
            };
            Ok(CacheArgs { action: CacheAction::Get(key, default) })
        }
    }

    let args = parse_macro_input!(input as CacheArgs);

    let expanded = match args.action {
        CacheAction::Get(key, default) => {
            if let Some(def) = default {
                quote! {
                    rf_cache_facade::Cache::get(#key).unwrap_or_else(|| #def)
                }
            } else {
                quote! {
                    rf_cache_facade::Cache::get(#key)
                }
            }
        }
        CacheAction::Put(key, value, ttl) => {
            quote! {
                rf_cache_facade::Cache::put(#key, #value, #ttl)
            }
        }
        CacheAction::Forever(key, value) => {
            quote! {
                rf_cache_facade::Cache::forever(#key, #value)
            }
        }
        CacheAction::Forget(key) => {
            quote! {
                rf_cache_facade::Cache::forget(#key)
            }
        }
        CacheAction::Has(key) => {
            quote! {
                rf_cache_facade::Cache::has(#key)
            }
        }
        CacheAction::Flush => {
            quote! {
                rf_cache_facade::Cache::flush()
            }
        }
        CacheAction::Remember(key, ttl, closure) => {
            quote! {
                rf_cache_facade::Cache::remember(#key, #ttl, #closure)
            }
        }
        CacheAction::RememberForever(key, closure) => {
            quote! {
                rf_cache_facade::Cache::remember_forever(#key, #closure)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Implements the logger! macro for logging
///
/// ```rust,ignore
/// logger!(info: "User logged in");
/// logger!(error: "Failed to connect: {}", error);
/// logger!(debug: user);
/// logger!(warn: "Rate limit exceeded for {}", ip);
/// ```
pub fn logger_impl(input: TokenStream) -> TokenStream {
    struct LogArgs {
        level: Ident,
        message: Expr,
        args: Punctuated<Expr, Token![,]>,
    }

    impl Parse for LogArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let level: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let message: Expr = input.parse()?;

            let args = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                Punctuated::parse_terminated(input)?
            } else {
                Punctuated::new()
            };

            Ok(LogArgs { level, message, args })
        }
    }

    let args = parse_macro_input!(input as LogArgs);
    let level = &args.level;
    let message = &args.message;
    let level_str = level.to_string();

    let log_macro = match level_str.as_str() {
        "info" => quote! { tracing::info! },
        "error" => quote! { tracing::error! },
        "warn" | "warning" => quote! { tracing::warn! },
        "debug" => quote! { tracing::debug! },
        "trace" => quote! { tracing::trace! },
        _ => quote! { tracing::info! },
    };

    let expanded = if args.args.is_empty() {
        quote! {
            #log_macro!(#message)
        }
    } else {
        let extra_args = args.args.iter();
        quote! {
            #log_macro!(#message, #(#extra_args),*)
        }
    };

    TokenStream::from(expanded)
}

/// Implements the event! macro for dispatching events
///
/// ```rust,ignore
/// event!(UserCreated { user_id: 123 });
/// event!(dispatch: OrderShipped { order_id: 456 });
/// ```
pub fn event_impl(input: TokenStream) -> TokenStream {
    let event: Expr = parse_macro_input!(input as Expr);

    let expanded = quote! {
        rf_event_facade::Event::dispatch(#event)
    };

    TokenStream::from(expanded)
}

/// Implements the storage! macro for file storage
///
/// ```rust,ignore
/// let contents = storage!("file.txt");           // Get file
/// storage!(put: "file.txt", contents);           // Put file
/// storage!(delete: "file.txt");                  // Delete file
/// let url = storage!(url: "file.txt");           // Get URL
/// storage!(disk: "s3").put("file.txt", data);    // Use specific disk
/// ```
pub fn storage_impl(input: TokenStream) -> TokenStream {
    struct StorageArgs {
        action: StorageAction,
    }

    enum StorageAction {
        Get(LitStr),
        Put(LitStr, Expr),
        Delete(LitStr),
        Url(LitStr),
        Exists(LitStr),
        Disk(LitStr),
    }

    impl Parse for StorageArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            if input.peek(Ident) {
                let lookahead: Ident = input.fork().parse()?;
                let action_str = lookahead.to_string();

                match action_str.as_str() {
                    "put" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let path: LitStr = input.parse()?;
                        input.parse::<Token![,]>()?;
                        let contents: Expr = input.parse()?;
                        return Ok(StorageArgs { action: StorageAction::Put(path, contents) });
                    }
                    "delete" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let path: LitStr = input.parse()?;
                        return Ok(StorageArgs { action: StorageAction::Delete(path) });
                    }
                    "url" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let path: LitStr = input.parse()?;
                        return Ok(StorageArgs { action: StorageAction::Url(path) });
                    }
                    "exists" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let path: LitStr = input.parse()?;
                        return Ok(StorageArgs { action: StorageAction::Exists(path) });
                    }
                    "disk" => {
                        input.parse::<Ident>()?;
                        input.parse::<Token![:]>()?;
                        let name: LitStr = input.parse()?;
                        return Ok(StorageArgs { action: StorageAction::Disk(name) });
                    }
                    _ => {}
                }
            }

            // Default: get file
            let path: LitStr = input.parse()?;
            Ok(StorageArgs { action: StorageAction::Get(path) })
        }
    }

    let args = parse_macro_input!(input as StorageArgs);

    let expanded = match args.action {
        StorageAction::Get(path) => quote! {
            rf_storage_facade::Storage::get(#path)
        },
        StorageAction::Put(path, contents) => quote! {
            rf_storage_facade::Storage::put(#path, #contents)
        },
        StorageAction::Delete(path) => quote! {
            rf_storage_facade::Storage::delete(#path)
        },
        StorageAction::Url(path) => quote! {
            rf_storage_facade::Storage::url(#path)
        },
        StorageAction::Exists(path) => quote! {
            rf_storage_facade::Storage::exists(#path)
        },
        StorageAction::Disk(name) => quote! {
            rf_storage_facade::Storage::disk(#name)
        },
    };

    TokenStream::from(expanded)
}

// =============================================================================
// Laravel-style Eloquent Macros (update!, create!, find!, delete!)
// =============================================================================

/// Parse key = value pairs for update/create macros
struct FieldAssignment {
    key: Ident,
    value: Expr,
}

impl Parse for FieldAssignment {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: Expr = input.parse()?;
        Ok(FieldAssignment { key, value })
    }
}

/// Parse update! arguments: update!(Model, id, field = value, ...)
struct UpdateArgs {
    model: Ident,
    id: Expr,
    fields: Punctuated<FieldAssignment, Token![,]>,
}

impl Parse for UpdateArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let model: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let id: Expr = input.parse()?;
        input.parse::<Token![,]>()?;
        let fields = Punctuated::parse_terminated(input)?;
        Ok(UpdateArgs { model, id, fields })
    }
}

/// Implements the update! macro for Laravel-style updates
///
/// ```rust,ignore
/// // Simple syntax - no json!, no .await needed in macro!
/// update!(User, 1, name = "John", email = "john@example.com");
///
/// // Equivalent to:
/// // User::update_by_id(1, json!({"name": "John", "email": "john@example.com"})).await
/// ```
pub fn update_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as UpdateArgs);
    let model = &args.model;
    let id = &args.id;

    let field_names: Vec<_> = args.fields.iter().map(|f| f.key.to_string()).collect();
    let field_values: Vec<_> = args.fields.iter().map(|f| &f.value).collect();

    let expanded = quote! {
        #model::update_by_id(#id, serde_json::json!({
            #( #field_names: #field_values ),*
        })).await
    };

    TokenStream::from(expanded)
}

/// Parse create! arguments: create!(Model, field = value, ...)
struct CreateArgs {
    model: Ident,
    fields: Punctuated<FieldAssignment, Token![,]>,
}

impl Parse for CreateArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let model: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let fields = Punctuated::parse_terminated(input)?;
        Ok(CreateArgs { model, fields })
    }
}

/// Implements the create! macro for Laravel-style creation
///
/// ```rust,ignore
/// // Simple syntax - no json!, no .await needed in macro!
/// create!(User, name = "John", email = "john@example.com");
///
/// // Equivalent to:
/// // User::create(json!({"name": "John", "email": "john@example.com"})).await
/// ```
pub fn create_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as CreateArgs);
    let model = &args.model;

    let field_names: Vec<_> = args.fields.iter().map(|f| f.key.to_string()).collect();
    let field_values: Vec<_> = args.fields.iter().map(|f| &f.value).collect();

    let expanded = quote! {
        #model::create(serde_json::json!({
            #( #field_names: #field_values ),*
        })).await
    };

    TokenStream::from(expanded)
}

/// Parse find! arguments: find!(Model, id)
struct FindArgs {
    model: Ident,
    id: Expr,
}

impl Parse for FindArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let model: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let id: Expr = input.parse()?;
        Ok(FindArgs { model, id })
    }
}

/// Implements the find! macro for Laravel-style finding
///
/// ```rust,ignore
/// // Simple syntax
/// let user = find!(User, 1);
///
/// // Equivalent to:
/// // User::find(1).await
/// ```
pub fn find_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as FindArgs);
    let model = &args.model;
    let id = &args.id;

    let expanded = quote! {
        #model::find(#id).await
    };

    TokenStream::from(expanded)
}

/// Implements the delete! macro for Laravel-style deletion
///
/// ```rust,ignore
/// // Simple syntax
/// delete!(User, 1);
///
/// // Equivalent to:
/// // User::destroy(1).await
/// ```
pub fn delete_impl(input: TokenStream) -> TokenStream {
    let args = parse_macro_input!(input as FindArgs);  // Same structure as find
    let model = &args.model;
    let id = &args.id;

    let expanded = quote! {
        #model::destroy(#id).await
    };

    TokenStream::from(expanded)
}

// =============================================================================
// MEDIUM PRIORITY Laravel Helper Macros
// =============================================================================

/// Implements the mail! macro for sending emails Laravel-style
///
/// ```rust,ignore
/// // Simple email
/// mail!(to = "user@example.com", subject = "Welcome!", body = "Hello!");
///
/// // With template and data
/// mail!(
///     to = "user@example.com",
///     subject = "Order Confirmation",
///     template = "emails.order",
///     data = { order_id: 123, total: 99.99 },
/// );
///
/// // With attachments and queueing
/// mail!(
///     to = "user@example.com",
///     subject = "Invoice",
///     template = "emails.invoice",
///     data = invoice_data,
///     attach = "/path/to/invoice.pdf",
///     queue,
/// );
/// ```
pub fn mail_impl(input: TokenStream) -> TokenStream {
    struct MailArgs {
        options: Vec<MailOption>,
    }

    enum MailOption {
        To(Expr),
        Subject(Expr),
        Body(Expr),
        Template(Expr),
        Data(Expr),
        Attach(Expr),
        Queue,
    }

    impl Parse for MailArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut options = Vec::new();

            while !input.is_empty() {
                let key: Ident = input.parse()?;
                let key_str = key.to_string();

                match key_str.as_str() {
                    "queue" => {
                        options.push(MailOption::Queue);
                    }
                    _ => {
                        input.parse::<Token![=]>()?;
                        let value: Expr = input.parse()?;

                        match key_str.as_str() {
                            "to" => options.push(MailOption::To(value)),
                            "subject" => options.push(MailOption::Subject(value)),
                            "body" => options.push(MailOption::Body(value)),
                            "template" => options.push(MailOption::Template(value)),
                            "data" => options.push(MailOption::Data(value)),
                            "attach" => options.push(MailOption::Attach(value)),
                            _ => return Err(syn::Error::new(
                                key.span(),
                                format!("Unknown mail option: {}. Use to, subject, body, template, data, attach, or queue", key_str)
                            )),
                        }
                    }
                }

                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                } else {
                    break;
                }
            }

            Ok(MailArgs { options })
        }
    }

    let args = parse_macro_input!(input as MailArgs);

    // Extract individual options
    let mut to_expr = None;
    let mut subject_expr = None;
    let mut body_expr = None;
    let mut template_expr = None;
    let mut data_expr = None;
    let mut attach_expr = None;
    let mut is_queue = false;

    for option in args.options {
        match option {
            MailOption::To(expr) => to_expr = Some(expr),
            MailOption::Subject(expr) => subject_expr = Some(expr),
            MailOption::Body(expr) => body_expr = Some(expr),
            MailOption::Template(expr) => template_expr = Some(expr),
            MailOption::Data(expr) => data_expr = Some(expr),
            MailOption::Attach(expr) => attach_expr = Some(expr),
            MailOption::Queue => is_queue = true,
        }
    }

    // Build the mail chain
    let mut mail_chain = quote! { rf_mail_facade::Mail::new() };

    if let Some(to) = to_expr {
        mail_chain = quote! { #mail_chain.to(#to) };
    }

    if let Some(subject) = subject_expr {
        mail_chain = quote! { #mail_chain.subject(#subject) };
    }

    if let Some(body) = body_expr {
        mail_chain = quote! { #mail_chain.body(#body) };
    }

    if let Some(template) = template_expr {
        if let Some(data) = data_expr {
            mail_chain = quote! { #mail_chain.template(#template, #data) };
        } else {
            mail_chain = quote! { #mail_chain.template(#template, serde_json::json!({})) };
        }
    }

    if let Some(attach) = attach_expr {
        mail_chain = quote! { #mail_chain.attach(#attach) };
    }

    // Add send or queue
    let expanded = if is_queue {
        quote! { #mail_chain.queue() }
    } else {
        quote! { #mail_chain.send() }
    };

    TokenStream::from(expanded)
}

/// Implements the dispatch! macro for event dispatching
///
/// ```rust,ignore
/// // Dispatch event with named fields
/// dispatch!(user.registered, user_id = 1, email = "john@example.com");
///
/// // Dispatch event struct
/// dispatch!(UserRegistered { user_id: 1, email: "john@example.com".into() });
///
/// // Delayed dispatch
/// dispatch!(order.shipped, order_id = 123, delay = 3600);
///
/// // With explicit delay method
/// dispatch!(delay: 3600, OrderShipped { order_id: 123 });
/// ```
pub fn dispatch_impl(input: TokenStream) -> TokenStream {
    struct DispatchArgs {
        delay: Option<Expr>,
        event: DispatchEvent,
    }

    enum DispatchEvent {
        Named(LitStr, Vec<(Ident, Expr)>),  // "event.name", [(field, value), ...]
        Struct(Expr),                        // EventStruct { ... }
    }

    impl Parse for DispatchArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut delay = None;

            // Check for delay: prefix
            if input.peek(Ident) {
                let lookahead: Ident = input.fork().parse()?;
                if lookahead == "delay" {
                    input.parse::<Ident>()?;  // consume "delay"
                    input.parse::<Token![:]>()?;
                    delay = Some(input.parse()?);
                    input.parse::<Token![,]>()?;
                }
            }

            // Try parsing as named event with dot notation
            if input.peek(LitStr) || (input.peek(Ident) && input.peek2(Token![.])) {
                // Try to parse identifier.identifier as a string-like event name
                let event_name = if input.peek(LitStr) {
                    input.parse::<LitStr>()?
                } else {
                    // Parse ident.ident as event name
                    let first: Ident = input.parse()?;
                    input.parse::<Token![.]>()?;
                    let second: Ident = input.parse()?;
                    let combined = format!("{}.{}", first, second);
                    LitStr::new(&combined, first.span())
                };

                let mut fields = Vec::new();
                while input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                    if input.is_empty() {
                        break;
                    }

                    // Check if it's delay field
                    let field_name: Ident = input.parse()?;
                    if field_name == "delay" {
                        input.parse::<Token![=]>()?;
                        delay = Some(input.parse()?);
                        continue;
                    }

                    input.parse::<Token![=]>()?;
                    let field_value: Expr = input.parse()?;
                    fields.push((field_name, field_value));
                }

                return Ok(DispatchArgs {
                    delay,
                    event: DispatchEvent::Named(event_name, fields),
                });
            }

            // Otherwise parse as struct expression
            let event_expr: Expr = input.parse()?;
            Ok(DispatchArgs {
                delay,
                event: DispatchEvent::Struct(event_expr),
            })
        }
    }

    let args = parse_macro_input!(input as DispatchArgs);

    let event_expr = match args.event {
        DispatchEvent::Named(name, fields) => {
            let field_names: Vec<_> = fields.iter().map(|(k, _)| k.to_string()).collect();
            let field_values: Vec<_> = fields.iter().map(|(_, v)| v).collect();
            quote! {
                serde_json::json!({
                    "event": #name,
                    #( #field_names: #field_values ),*
                })
            }
        }
        DispatchEvent::Struct(expr) => {
            quote! { #expr }
        }
    };

    let expanded = if let Some(delay_expr) = args.delay {
        quote! {
            rf_event_facade::Event::dispatch_later(#event_expr, #delay_expr)
        }
    } else {
        quote! {
            rf_event_facade::Event::dispatch(#event_expr)
        }
    };

    TokenStream::from(expanded)
}

/// Implements the job! macro for defining background jobs
///
/// ```rust,ignore
/// job! {
///     SendEmail(to: String, subject: String, body: String) {
///         Mail::to(&self.to).subject(&self.subject).body(&self.body).send().await
///     }
///
///     retries = 3,
///     timeout = 300,
///     backoff = [60, 300, 900],
/// }
///
/// job! {
///     ProcessVideo(video_id: i64, resolution: String) {
///         let video = Video::find(self.video_id).await?;
///         video.process(&self.resolution).await?;
///         Ok(())
///     }
///
///     queue = "video-processing",
///     retries = 5,
/// }
/// ```
pub fn job_impl(input: TokenStream) -> TokenStream {
    use syn::{braced, Type};

    struct JobDefinition {
        name: Ident,
        fields: Punctuated<JobField, Token![,]>,
        body: Vec<syn::Stmt>,
        options: Vec<JobOption>,
    }

    struct JobField {
        name: Ident,
        _colon: Token![:],
        ty: Type,
    }

    enum JobOption {
        Retries(Expr),
        Timeout(Expr),
        Backoff(Expr),
        Queue(Expr),
    }

    impl Parse for JobField {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(JobField {
                name: input.parse()?,
                _colon: input.parse()?,
                ty: input.parse()?,
            })
        }
    }

    impl Parse for JobDefinition {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            // Parse job name
            let name: Ident = input.parse()?;

            // Parse fields in parentheses
            let fields_content;
            syn::parenthesized!(fields_content in input);
            let fields = Punctuated::parse_terminated(&fields_content)?;

            // Parse body in braces
            let body_content;
            braced!(body_content in input);
            let mut body = Vec::new();
            while !body_content.is_empty() {
                body.push(body_content.parse()?);
            }

            // Parse options (comma-separated key = value)
            let mut options = Vec::new();
            while input.peek(Token![,]) || (!input.is_empty() && input.peek(Ident)) {
                if input.peek(Token![,]) {
                    input.parse::<Token![,]>()?;
                }
                if input.is_empty() {
                    break;
                }

                let option_name: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                let option_value: Expr = input.parse()?;

                let option = match option_name.to_string().as_str() {
                    "retries" => JobOption::Retries(option_value),
                    "timeout" => JobOption::Timeout(option_value),
                    "backoff" => JobOption::Backoff(option_value),
                    "queue" => JobOption::Queue(option_value),
                    other => return Err(syn::Error::new(
                        option_name.span(),
                        format!("Unknown job option: {}. Use retries, timeout, backoff, or queue", other)
                    )),
                };
                options.push(option);
            }

            Ok(JobDefinition { name, fields, body, options })
        }
    }

    let job_def = parse_macro_input!(input as JobDefinition);
    let name = &job_def.name;
    let body_stmts = &job_def.body;

    // Extract field names and types
    let field_names: Vec<_> = job_def.fields.iter().map(|f| &f.name).collect();
    let field_types: Vec<_> = job_def.fields.iter().map(|f| &f.ty).collect();

    // Extract options
    let mut retries = None;
    let mut timeout = None;
    let mut backoff = None;
    let mut queue = None;

    for option in job_def.options {
        match option {
            JobOption::Retries(expr) => retries = Some(expr),
            JobOption::Timeout(expr) => timeout = Some(expr),
            JobOption::Backoff(expr) => backoff = Some(expr),
            JobOption::Queue(expr) => queue = Some(expr),
        }
    }

    // Build option implementations
    let retries_impl = if let Some(r) = retries {
        quote! {
            fn retries(&self) -> u32 {
                #r
            }
        }
    } else {
        quote! {}
    };

    let timeout_impl = if let Some(t) = timeout {
        quote! {
            fn timeout(&self) -> u64 {
                #t
            }
        }
    } else {
        quote! {}
    };

    let backoff_impl = if let Some(b) = backoff {
        quote! {
            fn backoff(&self) -> Vec<u64> {
                #b.to_vec()
            }
        }
    } else {
        quote! {}
    };

    let queue_impl = if let Some(q) = queue {
        quote! {
            fn queue(&self) -> &str {
                #q
            }
        }
    } else {
        quote! {}
    };

    let expanded = quote! {
        #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
        pub struct #name {
            #( pub #field_names: #field_types ),*
        }

        impl #name {
            pub fn new(#( #field_names: #field_types ),*) -> Self {
                Self {
                    #( #field_names ),*
                }
            }
        }

        #[async_trait::async_trait]
        impl rf_job_facade::Job for #name {
            async fn handle(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                #( #body_stmts )*
                Ok(())
            }

            #retries_impl
            #timeout_impl
            #backoff_impl
            #queue_impl
        }
    };

    TokenStream::from(expanded)
}
