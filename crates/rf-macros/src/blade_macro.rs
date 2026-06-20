//! Blade-like Template Macros
//!
//! Write templates with Laravel Blade-style directives:
//!
//! ```rust,ignore
//! let html = blade! {
//!     <div class="container">
//!         @if let Some(user) = user {
//!             <h1>Welcome, {{ user.name }}!</h1>
//!
//!             @if user.is_admin {
//!                 <span class="badge">Admin</span>
//!             } @else {
//!                 <span class="badge">User</span>
//!             }
//!
//!             <ul>
//!             @foreach post in user.posts {
//!                 <li>{{ post.title }}</li>
//!             }
//!             </ul>
//!         } @else {
//!             <p>Please log in</p>
//!         }
//!     </div>
//! };
//! ```

use proc_macro::TokenStream;
use proc_macro2::{TokenStream as TokenStream2, TokenTree, Delimiter};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, LitStr,
};

/// Main blade template macro implementation
pub fn blade_impl(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.into();

    // Transform blade syntax to Rust
    let transformed = transform_blade_tokens(input2);

    let expanded = quote! {
        {
            let mut __blade_output = String::new();
            #transformed
            __blade_output
        }
    };

    TokenStream::from(expanded)
}

/// Transform blade tokens to Rust code
fn transform_blade_tokens(input: TokenStream2) -> TokenStream2 {
    let mut output = TokenStream2::new();
    let mut iter = input.into_iter().peekable();

    while let Some(token) = iter.next() {
        match &token {
            // Handle @ directives
            TokenTree::Punct(p) if p.as_char() == '@' => {
                if let Some(TokenTree::Ident(directive)) = iter.peek() {
                    let directive_name = directive.to_string();
                    iter.next(); // consume the identifier

                    match directive_name.as_str() {
                        "if" => {
                            // Collect condition until {
                            let condition = collect_until_brace(&mut iter);
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    if #condition {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "else" => {
                            // Check for @else if or just @else
                            if let Some(TokenTree::Ident(next)) = iter.peek() {
                                if *next == "if" {
                                    iter.next(); // consume "if"
                                    let condition = collect_until_brace(&mut iter);
                                    if let Some(TokenTree::Group(body)) = iter.next() {
                                        let body_transformed = transform_blade_tokens(body.stream());
                                        output.extend(quote! {
                                            else if #condition {
                                                #body_transformed
                                            }
                                        });
                                    }
                                } else if let Some(TokenTree::Group(body)) = iter.next() {
                                    let body_transformed = transform_blade_tokens(body.stream());
                                    output.extend(quote! {
                                        else {
                                            #body_transformed
                                        }
                                    });
                                }
                            } else if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    else {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "foreach" => {
                            // Parse: @foreach item in collection { ... }
                            let loop_var = collect_until_in(&mut iter);
                            // Skip "in"
                            if let Some(TokenTree::Ident(in_kw)) = iter.next() {
                                if in_kw == "in" {
                                    let collection = collect_until_brace(&mut iter);
                                    if let Some(TokenTree::Group(body)) = iter.next() {
                                        let body_transformed = transform_blade_tokens(body.stream());
                                        output.extend(quote! {
                                            for #loop_var in #collection {
                                                #body_transformed
                                            }
                                        });
                                    }
                                }
                            }
                        }
                        "for" => {
                            // Standard for loop
                            let loop_expr = collect_until_brace(&mut iter);
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    for #loop_expr {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "while" => {
                            let condition = collect_until_brace(&mut iter);
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    while #condition {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "match" => {
                            let expr = collect_until_brace(&mut iter);
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    match #expr {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "isset" => {
                            // @isset(var) { ... }
                            if let Some(TokenTree::Group(var_group)) = iter.next() {
                                let var = var_group.stream();
                                if let Some(TokenTree::Group(body)) = iter.next() {
                                    let body_transformed = transform_blade_tokens(body.stream());
                                    output.extend(quote! {
                                        if (#var).is_some() {
                                            #body_transformed
                                        }
                                    });
                                }
                            }
                        }
                        "empty" => {
                            // @empty(collection) { ... }
                            if let Some(TokenTree::Group(var_group)) = iter.next() {
                                let var = var_group.stream();
                                if let Some(TokenTree::Group(body)) = iter.next() {
                                    let body_transformed = transform_blade_tokens(body.stream());
                                    output.extend(quote! {
                                        if (#var).is_empty() {
                                            #body_transformed
                                        }
                                    });
                                }
                            }
                        }
                        "auth" => {
                            // @auth { ... } - content for authenticated users
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    if rf_auth_facade::Auth::check() {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "guest" => {
                            // @guest { ... } - content for guests
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let body_transformed = transform_blade_tokens(body.stream());
                                output.extend(quote! {
                                    if !rf_auth_facade::Auth::check() {
                                        #body_transformed
                                    }
                                });
                            }
                        }
                        "csrf" => {
                            // @csrf - output CSRF token field
                            output.extend(quote! {
                                __blade_output.push_str(&format!(
                                    r#"<input type="hidden" name="_token" value="{}">"#,
                                    rf_session::Session::csrf_token()
                                ));
                            });
                        }
                        "method" => {
                            // @method("PUT") - method spoofing
                            if let Some(TokenTree::Group(method_group)) = iter.next() {
                                let method = method_group.stream();
                                output.extend(quote! {
                                    __blade_output.push_str(&format!(
                                        r#"<input type="hidden" name="_method" value="{}">"#,
                                        #method
                                    ));
                                });
                            }
                        }
                        "include" => {
                            // @include("partial") - include another template
                            if let Some(TokenTree::Group(path_group)) = iter.next() {
                                let path = path_group.stream();
                                output.extend(quote! {
                                    __blade_output.push_str(&rf_view::View::render(#path, serde_json::json!({})));
                                });
                            }
                        }
                        "json" => {
                            // @json(data) - output as JSON
                            if let Some(TokenTree::Group(data_group)) = iter.next() {
                                let data = data_group.stream();
                                output.extend(quote! {
                                    __blade_output.push_str(&serde_json::to_string(&#data).unwrap_or_default());
                                });
                            }
                        }
                        "raw" => {
                            // @raw { ... } - raw HTML without escaping
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let content = body.stream().to_string();
                                output.extend(quote! {
                                    __blade_output.push_str(#content);
                                });
                            }
                        }
                        "verbatim" => {
                            // @verbatim { ... } - output as-is
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let content = body.stream().to_string();
                                output.extend(quote! {
                                    __blade_output.push_str(#content);
                                });
                            }
                        }
                        "php" | "rust" => {
                            // @rust { ... } - execute Rust code
                            if let Some(TokenTree::Group(body)) = iter.next() {
                                let code = body.stream();
                                output.extend(quote! {
                                    { #code }
                                });
                            }
                        }
                        "env" => {
                            // @env("KEY") - output environment variable
                            if let Some(TokenTree::Group(key_group)) = iter.next() {
                                let key = key_group.stream();
                                output.extend(quote! {
                                    __blade_output.push_str(&std::env::var(#key).unwrap_or_default());
                                });
                            }
                        }
                        "class" => {
                            // @class(["active" => is_active, "disabled" => is_disabled])
                            if let Some(TokenTree::Group(classes_group)) = iter.next() {
                                let classes = classes_group.stream();
                                output.extend(quote! {
                                    {
                                        let classes: Vec<(&str, bool)> = vec![#classes];
                                        let class_str: String = classes
                                            .into_iter()
                                            .filter(|(_, active)| *active)
                                            .map(|(name, _)| name)
                                            .collect::<Vec<_>>()
                                            .join(" ");
                                        __blade_output.push_str(&class_str);
                                    }
                                });
                            }
                        }
                        _ => {
                            // Unknown directive, output as-is
                            output.extend(quote! {
                                __blade_output.push_str("@");
                                __blade_output.push_str(#directive_name);
                            });
                        }
                    }
                } else {
                    // Just @ symbol
                    output.extend(quote! {
                        __blade_output.push_str("@");
                    });
                }
            }
            // Handle {{ expr }} - escaped output
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                let inner = g.stream();
                let inner_str = inner.to_string();

                // Check if it's {{ }} (double braces)
                if inner_str.starts_with('{') && inner_str.ends_with('}') {
                    // Check for actual double brace by looking at first token
                    let mut inner_iter = inner.clone().into_iter().peekable();
                    if let Some(TokenTree::Group(inner_group)) = inner_iter.next() {
                        if inner_group.delimiter() == Delimiter::Brace {
                            let expr = inner_group.stream();
                            output.extend(quote! {
                                __blade_output.push_str(&html_escape::encode_text(&format!("{}", #expr)));
                            });
                            continue;
                        }
                    }
                }

                // Regular braces - could be {!! expr !!} for unescaped
                if inner_str.starts_with("!!") && inner_str.ends_with("!!") {
                    // Unescaped output {!! expr !!}
                    let expr_str = inner_str.trim_start_matches("!!").trim_end_matches("!!");
                    let expr: TokenStream2 = expr_str.parse().unwrap_or_default();
                    output.extend(quote! {
                        __blade_output.push_str(&format!("{}", #expr));
                    });
                } else {
                    // Regular block, transform recursively
                    let transformed = transform_blade_tokens(inner);
                    output.extend(quote! {
                        { #transformed }
                    });
                }
            }
            // Handle string literals
            TokenTree::Literal(lit) => {
                let lit_str = lit.to_string();
                if lit_str.starts_with('"') && lit_str.ends_with('"') {
                    // String literal - output as HTML
                    let content = &lit_str[1..lit_str.len()-1];
                    output.extend(quote! {
                        __blade_output.push_str(#content);
                    });
                } else {
                    output.extend(std::iter::once(token));
                }
            }
            // Handle < for HTML tags
            TokenTree::Punct(p) if p.as_char() == '<' => {
                // Collect until > to form HTML tag
                let mut tag_content = String::from("<");
                for next in iter.by_ref() {
                    match &next {
                        TokenTree::Punct(p) if p.as_char() == '>' => {
                            tag_content.push('>');
                            break;
                        }
                        _ => {
                            tag_content.push_str(&next.to_string());
                        }
                    }
                }
                output.extend(quote! {
                    __blade_output.push_str(#tag_content);
                });
            }
            // Pass through other tokens
            _ => {
                output.extend(std::iter::once(token));
            }
        }
    }

    output
}

/// Collect tokens until we hit a brace
fn collect_until_brace(iter: &mut std::iter::Peekable<impl Iterator<Item = TokenTree>>) -> TokenStream2 {
    let mut tokens = TokenStream2::new();
    while let Some(token) = iter.peek() {
        if matches!(token, TokenTree::Group(g) if g.delimiter() == Delimiter::Brace) {
            break;
        }
        tokens.extend(iter.next());
    }
    tokens
}

/// Collect tokens until we hit "in" keyword
fn collect_until_in(iter: &mut std::iter::Peekable<impl Iterator<Item = TokenTree>>) -> TokenStream2 {
    let mut tokens = TokenStream2::new();
    while let Some(token) = iter.peek() {
        if matches!(token, TokenTree::Ident(i) if *i == "in") {
            break;
        }
        tokens.extend(iter.next());
    }
    tokens
}

/// HTML template literal macro - simpler version
///
/// ```rust,ignore
/// let name = "World";
/// let html = html! {
///     <div class="greeting">
///         <h1>Hello, {name}!</h1>
///     </div>
/// };
/// ```
pub fn html_impl(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.into();

    let expanded = quote! {
        {
            let mut __html_output = String::new();
            // Simple pass-through for now
            __html_output.push_str(&format!("{}", stringify!(#input2)));
            __html_output
        }
    };

    TokenStream::from(expanded)
}

/// Component macro for defining reusable Blade components
///
/// ```rust,ignore
/// component!(Alert {
///     props: {
///         message: String,
///         #[default = "info"]
///         type_: String,
///     },
///     template: {
///         <div class="alert alert-{{ type_ }}">
///             {{ message }}
///         </div>
///     }
/// });
///
/// // Usage:
/// let alert = Alert::new()
///     .message("Success!")
///     .type_("success")
///     .render();
/// ```
pub fn component_impl(input: TokenStream) -> TokenStream {
    // Simplified component implementation
    let input2: TokenStream2 = input.into();

    let expanded = quote! {
        // Component placeholder - full implementation would parse and generate struct + render
        #input2
    };

    TokenStream::from(expanded)
}

/// Slot macro for component slots
pub fn slot_impl(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.into();

    let expanded = quote! {
        {
            let __slot_content = || { #input2 };
            __slot_content()
        }
    };

    TokenStream::from(expanded)
}

/// Section/yield macros for layout inheritance
///
/// ```rust,ignore
/// // layout.rs
/// layout! {
///     <html>
///         <head>
///             <title>@yield("title")</title>
///         </head>
///         <body>
///             @yield("content")
///         </body>
///     </html>
/// }
///
/// // page.rs
/// extends!("layout");
///
/// section!("title") {
///     "My Page"
/// }
///
/// section!("content") {
///     <h1>Hello World</h1>
/// }
/// ```
pub fn section_impl(input: TokenStream) -> TokenStream {
    struct SectionArgs {
        name: LitStr,
        content: TokenStream2,
    }

    impl Parse for SectionArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let name: LitStr = input.parse()?;
            let content: TokenStream2 = input.parse()?;
            Ok(SectionArgs { name, content })
        }
    }

    let args = parse_macro_input!(input as SectionArgs);
    let name = &args.name;
    let content = &args.content;

    let expanded = quote! {
        rf_view::View::section(#name, || {
            let mut __section_output = String::new();
            #content
            __section_output
        })
    };

    TokenStream::from(expanded)
}

/// Stack/push macros for adding to stacks
///
/// ```rust,ignore
/// @push("scripts") {
///     <script src="/js/app.js"></script>
/// }
///
/// @stack("scripts")  // Outputs all pushed content
/// ```
pub fn push_impl(input: TokenStream) -> TokenStream {
    struct PushArgs {
        stack: LitStr,
        content: TokenStream2,
    }

    impl Parse for PushArgs {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let stack: LitStr = input.parse()?;
            let content: TokenStream2 = input.parse()?;
            Ok(PushArgs { stack, content })
        }
    }

    let args = parse_macro_input!(input as PushArgs);
    let stack = &args.stack;
    let content = &args.content;

    let expanded = quote! {
        rf_view::View::push_to_stack(#stack, || {
            let mut __push_output = String::new();
            #content
            __push_output
        })
    };

    TokenStream::from(expanded)
}

pub fn stack_impl(input: TokenStream) -> TokenStream {
    let stack: LitStr = parse_macro_input!(input as LitStr);

    let expanded = quote! {
        rf_view::View::render_stack(#stack)
    };

    TokenStream::from(expanded)
}
