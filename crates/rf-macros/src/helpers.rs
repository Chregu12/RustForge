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
