//! Attribute macro that records which requirements a function implements.
//!
//! # Overview
//!
//! [`traces`] is an attribute macro that annotates a function with one or more
//! requirement identifiers (e.g. `"REQ-123"`). It performs two actions:
//!
//! 1. Injects a structured doc comment `**Traces:** REQ-123, REQ-456` so the
//!    links appear in generated documentation.
//! 2. Generates a `const __TPT_TRACES_<FN_NAME_UPPER>: &[&str]` whose value is
//!    the exact slice of requirement ids passed to the macro. This is
//!    machine-readable and can be consumed by trace-coverage tooling.
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
//! const __TPT_TRACES_THROTTLE: &[&str] = &["REQ-THR-001", "REQ-SAFE-002"];
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

#![deny(missing_docs)]
extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, punctuated::Punctuated, Expr, ItemFn, Token};

/// Marks a function as implementing the given requirement identifiers.
///
/// Accepts one or more string-literal requirement ids. A doc comment
/// `**Traces:** …` is injected and a `const __TPT_TRACES_<FN_NAME_UPPER>` is
/// generated holding the ids verbatim.
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
    let reqs = parse_macro_input!(attr with Punctuated::<Expr, Token![,]>::parse_terminated);
    let func = parse_macro_input!(item as ItemFn);

    let req_strs: Vec<String> = reqs
        .iter()
        .map(|e| match e {
            Expr::Lit(expr_lit) => match &expr_lit.lit {
                syn::Lit::Str(s) => s.value(),
                _ => panic!("traces! expects string-literal requirement ids"),
            },
            _ => panic!("traces! expects string-literal requirement ids"),
        })
        .collect();

    let joined = req_strs.join(", ");

    // Build the UPPER_SNAKE_CASE function-name constant.
    let fn_ident = func.sig.ident.to_string();
    let const_name = format!("__TPT_TRACES_{}", to_upper_snake(&fn_ident));

    let doc_attr = quote! {
        #[doc = concat!("**Traces:** ", #joined)]
    };

    let lit_exprs = req_strs.iter().map(|s| quote! { #s });
    let const_ident = syn::Ident::new(&const_name, proc_macro2::Span::call_site());

    quote! {
        #doc_attr
        #func

        /// Requirement identifiers traced by the preceding function.
        #[doc(hidden)]
        pub const #const_ident: &[&str] = &[ #(#lit_exprs),* ];
    }
    .into()
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
