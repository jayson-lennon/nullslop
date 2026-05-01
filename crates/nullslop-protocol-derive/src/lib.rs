//! Derive macros for `EventMsg` and `CommandMsg` routing traits.
//!
//! These macros generate compile-time routing strings from a module scope and
//! the struct name, producing values like `"chat_input::ChatEntrySubmitted"`.
//!
//! # `#[derive(EventMsg)]`
//!
//! Requires a `#[event_msg("module")]` helper attribute:
//!
//! ```ignore
//! #[derive(EventMsg)]
//! #[event_msg("chat_input")]
//! struct ChatEntrySubmitted;
//! ```
//!
//! Generates:
//! ```ignore
//! impl EventMsg for ChatEntrySubmitted {
//!     const TYPE_NAME: &'static str = "chat_input::ChatEntrySubmitted";
//! }
//! ```
//!
//! # `#[derive(CommandMsg)]`
//!
//! Requires a `#[cmd("module")]` helper attribute:
//!
//! ```ignore
//! #[derive(CommandMsg)]
//! #[cmd("chat_input")]
//! struct InsertChar;
//! ```
//!
//! Generates:
//! ```ignore
//! impl CommandMsg for InsertChar {
//!     const NAME: &'static str = "chat_input::InsertChar";
//! }
//! ```

use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::quote;
use syn::{Data, DeriveInput, Expr, ExprLit, Lit, parse_macro_input};

/// Helper attribute name for `EventMsg` derive.
const EVENT_MSG_ATTR: &str = "event_msg";
/// Helper attribute name for `CommandMsg` derive.
const CMD_ATTR: &str = "cmd";

/// Extract the module string from a helper attribute like `#[event_msg("module")]`.
///
/// # Errors
///
/// Returns a compile error if:
/// - The attribute value is not a string literal
/// - The attribute appears more than once
fn extract_module(attr_name: &str, attrs: &[syn::Attribute]) -> syn::Result<String> {
    let mut found: Option<String> = None;

    for attr in attrs {
        if !attr.path().is_ident(attr_name) {
            continue;
        }

        if found.is_some() {
            return Err(syn::Error::new(
                attr.path()
                    .get_ident()
                    .map_or_else(Span::call_site, proc_macro2::Ident::span),
                format!("duplicate {attr_name} attribute"),
            ));
        }

        // Parse the attribute as a single string literal: #[attr("value")]
        let value: Expr = attr.parse_args()?;
        let Expr::Lit(ExprLit {
            lit: Lit::Str(lit_str),
            ..
        }) = &value
        else {
            return Err(syn::Error::new(
                Span::call_site(),
                format!("{attr_name} attribute value must be a string literal"),
            ));
        };

        found = Some(lit_str.value());
    }

    found.ok_or_else(|| {
        syn::Error::new(
            Span::call_site(),
            format!("{attr_name} derive requires #[{attr_name}(\"module\")] attribute"),
        )
    })
}

/// Derive macro for `EventMsg` trait.
///
/// See crate-level documentation for usage.
#[proc_macro_derive(EventMsg, attributes(event_msg))]
pub fn derive_event_msg(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Only structs are supported
    match &input.data {
        Data::Struct(_) => {}
        Data::Enum(e) => {
            return syn::Error::new(
                e.enum_token.span,
                "EventMsg can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
        Data::Union(u) => {
            return syn::Error::new(
                u.union_token.span,
                "EventMsg can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    }

    let module = match extract_module(EVENT_MSG_ATTR, &input.attrs) {
        Ok(m) => m,
        Err(e) => return e.to_compile_error().into(),
    };

    let ident = &input.ident;
    let module_lit = &module;

    let expanded = quote! {
        impl EventMsg for #ident {
            const TYPE_NAME: &'static str = concat!(#module_lit, "::", stringify!(#ident));
        }
    };

    expanded.into()
}

/// Derive macro for `CommandMsg` trait.
///
/// See crate-level documentation for usage.
#[proc_macro_derive(CommandMsg, attributes(cmd))]
pub fn derive_command_msg(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    // Only structs are supported
    match &input.data {
        Data::Struct(_) => {}
        Data::Enum(e) => {
            return syn::Error::new(
                e.enum_token.span,
                "CommandMsg can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
        Data::Union(u) => {
            return syn::Error::new(
                u.union_token.span,
                "CommandMsg can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    }

    let module = match extract_module(CMD_ATTR, &input.attrs) {
        Ok(m) => m,
        Err(e) => return e.to_compile_error().into(),
    };

    let ident = &input.ident;
    let module_lit = &module;

    let expanded = quote! {
        impl CommandMsg for #ident {
            const NAME: &'static str = concat!(#module_lit, "::", stringify!(#ident));
        }
    };

    expanded.into()
}
