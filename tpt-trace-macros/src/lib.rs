//! Attribute macro that records which requirements a function implements.
//!
//! # Overview
//!
//! [`traces`] is an attribute macro that annotates a function with one or more
//! requirement identifiers (e.g. `"REQ-123"`). It works on free functions and
//! on methods in `impl` blocks. It performs two actions:
//!
//! 1. Injects a structured doc comment `**Traces:** REQ-123, REQ-456` so the
//!    links appear in generated documentation.
//! 2. Generates a `const __TPT_TRACES_<FN_NAME_UPPER>: &[&str]` whose value is
//!    the exact slice of requirement ids passed to the macro. This is
//!    machine-readable and can be consumed by trace-coverage tooling.
//!
//! The constant has the same visibility as the function. A method's constant
//! is an associated constant, so methods with the same name on different types
//! do not collide.
//!
//! # Example
//!
//! ```rust
//! use tpt_trace_macros::traces;
//!
//! /// Returns the calibrated throttle position.
//! #[traces("REQ-THR-001", "REQ-SAFE-002")]
//! pub fn throttle(position: u8) -> u8 {
//!     position.min(100)
//! }
//! ```
//!
//! The expansion above also defines:
//!
//! ```rust,ignore
//! pub const __TPT_TRACES_THROTTLE: &[&str] = &["REQ-THR-001", "REQ-SAFE-002"];
//! ```
//!
//! # Generated constants are queryable
//!
//! ```rust
//! use tpt_trace_macros::traces;
//!
//! #[traces("REQ-DOC-1", "REQ-DOC-2")]
//! pub fn example() {}
//!
//! assert_eq!(__TPT_TRACES_EXAMPLE, &["REQ-DOC-1", "REQ-DOC-2"]);
//! ```
//!
//! # Errors
//!
//! The macro reports errors at the offending token: no requirement ids, a
//! non-string argument, an empty id, or a duplicate id.

#![deny(missing_docs)]
#![forbid(unsafe_code)]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::Parser;
use syn::punctuated::Punctuated;
use syn::{Expr, ExprLit, Ident, ImplItemFn, ItemFn, Lit, Token};

/// Marks a function as implementing the given requirement identifiers.
///
/// Accepts one or more non-empty, distinct string-literal requirement ids. A
/// doc comment `**Traces:** …` is injected, and a
/// `const __TPT_TRACES_<FN_NAME_UPPER>` is generated holding the ids verbatim.
///
/// # Example
///
/// ```rust
/// use tpt_trace_macros::traces;
///
/// #[traces("REQ-A", "REQ-B")]
/// pub fn do_work() {}
/// ```
#[proc_macro_attribute]
pub fn traces(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let reqs = parse_requirements(attr)?;
    let joined = reqs.join(", ");
    let lit_exprs = reqs.iter().map(|s| quote! { #s });

    let doc_attr = quote! {
        #[doc = concat!("**Traces:** ", #joined)]
    };

    let target = parse_target(item)?;
    let (vis, ident) = match &target {
        Target::Function(f) => (&f.vis, &f.sig.ident),
        Target::Method(m) => (&m.vis, &m.sig.ident),
    };
    let const_name = format!(
        "__TPT_TRACES_{}",
        to_upper_snake(&ident.unraw().to_string())
    );
    let const_ident = Ident::new(&const_name, Span::call_site());

    Ok(match &target {
        Target::Function(f) => quote! {
            #doc_attr
            #f

            /// Requirement identifiers traced by the preceding function.
            #[doc(hidden)]
            #[allow(dead_code)]
            #vis const #const_ident: &[&str] = &[ #(#lit_exprs),* ];
        },
        Target::Method(m) => quote! {
            #doc_attr
            #m

            /// Requirement identifiers traced by the preceding method.
            #[doc(hidden)]
            #[allow(dead_code)]
            #vis const #const_ident: &[&str] = &[ #(#lit_exprs),* ];
        },
    })
}

/// The item the attribute is attached to.
enum Target {
    Function(ItemFn),
    Method(ImplItemFn),
}

fn parse_target(item: TokenStream2) -> syn::Result<Target> {
    match syn::parse2::<ItemFn>(item.clone()) {
        Ok(f) => Ok(Target::Function(f)),
        Err(free_err) => match syn::parse2::<ImplItemFn>(item) {
            Ok(m) => Ok(Target::Method(m)),
            // Report the free-function error: it is the usual form, and the
            // method parse also fails on any non-function item.
            Err(_) => Err(free_err),
        },
    }
}

/// Parses and validates the requirement ids given to `#[traces(...)]`.
fn parse_requirements(attr: TokenStream2) -> syn::Result<Vec<String>> {
    let exprs = Punctuated::<Expr, Token![,]>::parse_terminated.parse2(attr)?;
    if exprs.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "#[traces] requires at least one requirement id",
        ));
    }

    let mut ids: Vec<String> = Vec::with_capacity(exprs.len());
    for expr in &exprs {
        let lit = match expr {
            Expr::Lit(ExprLit {
                lit: Lit::Str(lit), ..
            }) => lit,
            _ => {
                return Err(syn::Error::new_spanned(
                    expr,
                    "#[traces] expects string-literal requirement ids",
                ))
            }
        };
        let id = lit.value();
        if id.trim().is_empty() {
            return Err(syn::Error::new_spanned(
                lit,
                "requirement id must not be empty",
            ));
        }
        if ids.contains(&id) {
            return Err(syn::Error::new_spanned(
                lit,
                format!("duplicate requirement id `{id}`"),
            ));
        }
        ids.push(id);
    }
    Ok(ids)
}

/// Converts an identifier (e.g. `do_work`, `Throttle`) to UPPER_SNAKE_CASE.
fn to_upper_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    let mut prev_lower = false;
    for ch in name.chars() {
        if ch.is_uppercase() {
            if prev_lower {
                out.push('_');
            }
            out.extend(ch.to_uppercase());
            prev_lower = false;
        } else if ch == '_' {
            out.push('_');
            prev_lower = false;
        } else {
            out.extend(ch.to_uppercase());
            prev_lower = true;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(src: &str) -> syn::Result<Vec<String>> {
        parse_requirements(src.parse().unwrap())
    }

    #[test]
    fn accepts_one_or_more_distinct_ids() {
        assert_eq!(ids("\"A\"").unwrap(), ["A"]);
        assert_eq!(ids("\"A\", \"B\",").unwrap(), ["A", "B"]);
    }

    #[test]
    fn rejects_zero_args() {
        let err = ids("").unwrap_err();
        assert!(err.to_string().contains("at least one"));
    }

    #[test]
    fn rejects_non_string_args() {
        assert!(ids("42")
            .unwrap_err()
            .to_string()
            .contains("string-literal"));
        assert!(ids("CONST_NAME")
            .unwrap_err()
            .to_string()
            .contains("string-literal"));
    }

    #[test]
    fn rejects_empty_and_blank_ids() {
        assert!(ids("\"\"").unwrap_err().to_string().contains("empty"));
        assert!(ids("\"  \"").unwrap_err().to_string().contains("empty"));
    }

    #[test]
    fn rejects_duplicate_ids() {
        let err = ids("\"A\", \"B\", \"A\"").unwrap_err();
        assert!(err.to_string().contains("duplicate requirement id `A`"));
    }

    #[test]
    fn upper_snake_conversion() {
        assert_eq!(to_upper_snake("do_work"), "DO_WORK");
        assert_eq!(to_upper_snake("camelCase"), "CAMEL_CASE");
        assert_eq!(to_upper_snake("Throttle"), "THROTTLE");
    }
}
