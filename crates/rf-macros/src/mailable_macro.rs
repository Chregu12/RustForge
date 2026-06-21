//! Mailable Macro - Laravel-style Email System
//!
//! Define mailables with automatic email building:
//!
//! ```rust,ignore
//! mailable! {
//!     pub struct WelcomeEmail {
//!         user: User,
//!     }
//!
//!     fn envelope(&self) -> Envelope {
//!         Envelope::new()
//!             .subject("Welcome to RustForge!")
//!             .from("noreply@rustforge.dev")
//!     }
//!
//!     fn content(&self) -> Content {
//!         Content::view("emails.welcome")
//!             .with("user", &self.user)
//!             .with("url", "https://rustforge.dev/login")
//!     }
//!
//!     fn attachments(&self) -> Vec<Attachment> {
//!         vec![]
//!     }
//! }
//!
//! // Send email
//! Mail::to("user@example.com")
//!     .send(WelcomeEmail { user })
//!     .await?;
//! ```

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_macro_input, Ident, ItemFn, ItemStruct, Token, Type, Visibility,
    braced,
};

/// Parsed mailable definition
struct MailableDef {
    vis: Visibility,
    name: Ident,
    fields: Vec<MailableField>,
    envelope_fn: Option<ItemFn>,
    content_fn: Option<ItemFn>,
    attachments_fn: Option<ItemFn>,
    headers_fn: Option<ItemFn>,
}

struct MailableField {
    vis: Visibility,
    name: Ident,
    ty: Type,
}

impl Parse for MailableDef {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse struct definition
        let vis: Visibility = input.parse()?;
        let _struct_token: Token![struct] = input.parse()?;
        let name: Ident = input.parse()?;

        // Parse fields
        let content;
        braced!(content in input);

        let mut fields = Vec::new();
        while !content.is_empty() && !content.peek(Token![fn]) {
            let field_vis: Visibility = content.parse()?;
            let field_name: Ident = content.parse()?;
            let _colon: Token![:] = content.parse()?;
            let field_ty: Type = content.parse()?;
            let _ = content.parse::<Token![,]>();

            fields.push(MailableField {
                vis: field_vis,
                name: field_name,
                ty: field_ty,
            });
        }

        // Parse methods
        let mut envelope_fn = None;
        let mut content_fn = None;
        let mut attachments_fn = None;
        let mut headers_fn = None;

        while !input.is_empty() {
            let func: ItemFn = input.parse()?;
            let func_name = func.sig.ident.to_string();

            match func_name.as_str() {
                "envelope" => envelope_fn = Some(func),
                "content" => content_fn = Some(func),
                "attachments" => attachments_fn = Some(func),
                "headers" => headers_fn = Some(func),
                _ => {}
            }
        }

        Ok(MailableDef {
            vis,
            name,
            fields,
            envelope_fn,
            content_fn,
            attachments_fn,
            headers_fn,
        })
    }
}

pub fn mailable_impl(input: TokenStream) -> TokenStream {
    let def = parse_macro_input!(input as MailableDef);

    let vis = &def.vis;
    let name = &def.name;

    // Generate struct fields
    let struct_fields: Vec<_> = def.fields.iter().map(|f| {
        let fvis = &f.vis;
        let fname = &f.name;
        let fty = &f.ty;
        quote! { #fvis #fname: #fty }
    }).collect();

    // Generate envelope implementation
    let envelope_impl = if let Some(func) = &def.envelope_fn {
        let block = &func.block;
        quote! {
            fn envelope(&self) -> rf_mail::Envelope {
                #block
            }
        }
    } else {
        quote! {
            fn envelope(&self) -> rf_mail::Envelope {
                rf_mail::Envelope::new()
            }
        }
    };

    // Generate content implementation
    let content_impl = if let Some(func) = &def.content_fn {
        let block = &func.block;
        quote! {
            fn content(&self) -> rf_mail::Content {
                #block
            }
        }
    } else {
        quote! {
            fn content(&self) -> rf_mail::Content {
                rf_mail::Content::text("")
            }
        }
    };

    // Generate attachments implementation
    let attachments_impl = if let Some(func) = &def.attachments_fn {
        let block = &func.block;
        quote! {
            fn attachments(&self) -> Vec<rf_mail::Attachment> {
                #block
            }
        }
    } else {
        quote! {
            fn attachments(&self) -> Vec<rf_mail::Attachment> {
                vec![]
            }
        }
    };

    // Generate headers implementation
    let headers_impl = if let Some(func) = &def.headers_fn {
        let block = &func.block;
        quote! {
            fn headers(&self) -> rf_mail::Headers {
                #block
            }
        }
    } else {
        quote! {
            fn headers(&self) -> rf_mail::Headers {
                rf_mail::Headers::new()
            }
        }
    };

    let expanded = quote! {
        #[derive(Debug, Clone)]
        #vis struct #name {
            #(#struct_fields),*
        }

        impl rf_mail::Mailable for #name {
            #envelope_impl
            #content_impl
            #attachments_impl
            #headers_impl

            fn build(&self) -> rf_mail::Message {
                let envelope = self.envelope();
                let content = self.content();
                let attachments = self.attachments();
                let headers = self.headers();

                rf_mail::Message::new()
                    .envelope(envelope)
                    .content(content)
                    .attachments(attachments)
                    .headers(headers)
            }
        }

        impl #name {
            pub fn new(#(#struct_fields),*) -> Self {
                Self { #(#struct_fields.name),* }
            }

            pub async fn send_to(self, to: &str) -> Result<(), rf_mail::MailError> {
                rf_mail::Mail::to(to).send(self).await
            }

            pub fn queue(self) -> rf_mail::PendingMail<Self> {
                rf_mail::PendingMail::new(self)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Attribute macro for simpler mailable definition
///
/// ```rust,ignore
/// #[mailable(
///     subject = "Welcome!",
///     view = "emails.welcome"
/// )]
/// pub struct WelcomeEmail {
///     pub user: User,
/// }
/// ```
pub fn mailable_attr_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    struct MailableAttr {
        subject: Option<String>,
        view: Option<String>,
        from: Option<String>,
        markdown: Option<String>,
    }

    impl Parse for MailableAttr {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let mut subject = None;
            let mut view = None;
            let mut from = None;
            let mut markdown = None;

            while !input.is_empty() {
                let key: Ident = input.parse()?;
                input.parse::<Token![=]>()?;
                let value: syn::LitStr = input.parse()?;

                match key.to_string().as_str() {
                    "subject" => subject = Some(value.value()),
                    "view" => view = Some(value.value()),
                    "from" => from = Some(value.value()),
                    "markdown" => markdown = Some(value.value()),
                    _ => {}
                }

                let _ = input.parse::<Token![,]>();
            }

            Ok(MailableAttr { subject, view, from, markdown })
        }
    }

    let attrs = parse_macro_input!(attr as MailableAttr);
    let input = parse_macro_input!(item as ItemStruct);

    let vis = &input.vis;
    let name = &input.ident;
    let struct_attrs = &input.attrs;
    let fields = &input.fields;

    let subject = attrs.subject.unwrap_or_else(|| format!("{}", name));
    let from = attrs.from.unwrap_or_else(|| "noreply@example.com".to_string());

    let content_impl = if let Some(view) = &attrs.view {
        quote! {
            fn content(&self) -> rf_mail::Content {
                rf_mail::Content::view(#view)
                    .with_data(serde_json::to_value(self).unwrap_or_default())
            }
        }
    } else if let Some(markdown) = &attrs.markdown {
        quote! {
            fn content(&self) -> rf_mail::Content {
                rf_mail::Content::markdown(#markdown)
                    .with_data(serde_json::to_value(self).unwrap_or_default())
            }
        }
    } else {
        quote! {
            fn content(&self) -> rf_mail::Content {
                rf_mail::Content::text("")
            }
        }
    };

    let expanded = quote! {
        #(#struct_attrs)*
        #[derive(Debug, Clone, serde::Serialize)]
        #vis struct #name #fields

        impl rf_mail::Mailable for #name {
            fn envelope(&self) -> rf_mail::Envelope {
                rf_mail::Envelope::new()
                    .subject(#subject)
                    .from(#from)
            }

            #content_impl

            fn attachments(&self) -> Vec<rf_mail::Attachment> {
                vec![]
            }

            fn headers(&self) -> rf_mail::Headers {
                rf_mail::Headers::new()
            }

            fn build(&self) -> rf_mail::Message {
                let envelope = self.envelope();
                let content = self.content();
                let attachments = self.attachments();
                let headers = self.headers();

                rf_mail::Message::new()
                    .envelope(envelope)
                    .content(content)
                    .attachments(attachments)
                    .headers(headers)
            }
        }
    };

    TokenStream::from(expanded)
}

/// Notification macro - Laravel-style notifications
///
/// ```rust,ignore
/// notification! {
///     pub struct OrderShipped {
///         order: Order,
///     }
///
///     fn via(&self) -> Vec<Channel> {
///         vec![Channel::Mail, Channel::Database]
///     }
///
///     fn to_mail(&self) -> Mailable {
///         Mailable::new()
///             .subject("Your order has shipped!")
///             .view("emails.order_shipped")
///             .with("order", &self.order)
///     }
///
///     fn to_database(&self) -> Value {
///         json!({
///             "order_id": self.order.id,
///             "message": "Your order has shipped!"
///         })
///     }
/// }
///
/// // Send notification
/// user.notify(OrderShipped { order }).await?;
/// ```
pub fn notification_impl(input: TokenStream) -> TokenStream {
    struct NotificationDef {
        vis: Visibility,
        name: Ident,
        fields: Vec<(Visibility, Ident, Type)>,
        via_fn: Option<ItemFn>,
        to_mail_fn: Option<ItemFn>,
        to_database_fn: Option<ItemFn>,
        // Parsed from macro input but not yet consumed in codegen (WIP).
        #[allow(dead_code)]
        to_slack_fn: Option<ItemFn>,
        #[allow(dead_code)]
        to_broadcast_fn: Option<ItemFn>,
    }

    impl Parse for NotificationDef {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let vis: Visibility = input.parse()?;
            let _struct_token: Token![struct] = input.parse()?;
            let name: Ident = input.parse()?;

            let content;
            braced!(content in input);

            let mut fields = Vec::new();
            while !content.is_empty() && !content.peek(Token![fn]) {
                let field_vis: Visibility = content.parse()?;
                let field_name: Ident = content.parse()?;
                let _colon: Token![:] = content.parse()?;
                let field_ty: Type = content.parse()?;
                let _ = content.parse::<Token![,]>();
                fields.push((field_vis, field_name, field_ty));
            }

            let mut via_fn = None;
            let mut to_mail_fn = None;
            let mut to_database_fn = None;
            let mut to_slack_fn = None;
            let mut to_broadcast_fn = None;

            while !input.is_empty() {
                let func: ItemFn = input.parse()?;
                let func_name = func.sig.ident.to_string();

                match func_name.as_str() {
                    "via" => via_fn = Some(func),
                    "to_mail" | "toMail" => to_mail_fn = Some(func),
                    "to_database" | "toDatabase" => to_database_fn = Some(func),
                    "to_slack" | "toSlack" => to_slack_fn = Some(func),
                    "to_broadcast" | "toBroadcast" => to_broadcast_fn = Some(func),
                    _ => {}
                }
            }

            Ok(NotificationDef {
                vis,
                name,
                fields,
                via_fn,
                to_mail_fn,
                to_database_fn,
                to_slack_fn,
                to_broadcast_fn,
            })
        }
    }

    let def = parse_macro_input!(input as NotificationDef);

    let vis = &def.vis;
    let name = &def.name;

    let struct_fields: Vec<_> = def.fields.iter().map(|(fvis, fname, fty)| {
        quote! { #fvis #fname: #fty }
    }).collect();

    let via_impl = if let Some(func) = &def.via_fn {
        let block = &func.block;
        quote! {
            fn via(&self) -> Vec<rf_notification::Channel> {
                #block
            }
        }
    } else {
        quote! {
            fn via(&self) -> Vec<rf_notification::Channel> {
                vec![rf_notification::Channel::Mail]
            }
        }
    };

    let to_mail_impl = if let Some(func) = &def.to_mail_fn {
        let block = &func.block;
        quote! {
            fn to_mail(&self) -> Option<rf_mail::Message> {
                Some(#block)
            }
        }
    } else {
        quote! {
            fn to_mail(&self) -> Option<rf_mail::Message> {
                None
            }
        }
    };

    let to_database_impl = if let Some(func) = &def.to_database_fn {
        let block = &func.block;
        quote! {
            fn to_database(&self) -> Option<serde_json::Value> {
                Some(#block)
            }
        }
    } else {
        quote! {
            fn to_database(&self) -> Option<serde_json::Value> {
                None
            }
        }
    };

    let expanded = quote! {
        #[derive(Debug, Clone)]
        #vis struct #name {
            #(#struct_fields),*
        }

        impl rf_notification::Notification for #name {
            #via_impl
            #to_mail_impl
            #to_database_impl
        }
    };

    TokenStream::from(expanded)
}

/// Markdown email content helper
///
/// ```rust,ignore
/// let content = markdown! {
///     # Welcome {{ user.name }}!
///
///     Thanks for joining us. Here's what you can do:
///
///     - Create projects
///     - Invite team members
///     - Start building
///
///     @component("button", url: "https://app.rustforge.dev")
///         Get Started
///     @endcomponent
///
///     Thanks,
///     The RustForge Team
/// };
/// ```
pub fn markdown_impl(input: TokenStream) -> TokenStream {
    let input2: TokenStream2 = input.into();

    let expanded = quote! {
        {
            let markdown_content = stringify!(#input2);
            rf_mail::Content::markdown(markdown_content)
        }
    };

    TokenStream::from(expanded)
}
