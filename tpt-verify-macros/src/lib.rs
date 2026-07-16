//! Procedural macros for embedding formal contracts directly in Rust code.
//!
//! # Overview
//!
//! `tpt-verify-macros` provides four attribute macros that let you annotate
//! functions and loops with formal verification contracts:
//!
//! | Macro | Meaning |
//! |-------|---------|
//! | [`requires`] | Precondition — must hold when the function is called |
//! | [`ensures`]  | Postcondition — must hold when the function returns |
//! | [`invariant`] | Loop/struct invariant — must hold at annotated point |
//! | [`pure`]     | Marker — function has no observable side effects |
//!
//! # Runtime behaviour
//!
//! In **debug builds** (`debug_assertions` on), each contract is checked via
//! `debug_assert!` and will panic with a descriptive message if violated.
//!
//! In **release builds** (`debug_assertions` off), all macros are zero-cost —
//! they emit no code and add no overhead whatsoever.
//!
//! Additionally, all macros inject structured `#[doc]` attributes so that
//! the contracts appear clearly in the generated documentation.
//!
//! # Usage
//!
//! ```rust
//! use tpt_verify_macros::{requires, ensures, pure};
//!
//! #[requires(x > 0)]
//! #[ensures(result > x)]
//! pub fn double_positive(x: i32) -> i32 {
//!     x * 2
//! }
//!
//! #[pure]
//! pub fn square(x: i32) -> i32 {
//!     x * x
//! }
//! ```
//!
//! # Integration with formal verifiers
//!
//! These macros are designed to be readable by external verification tools
//! such as [Kani](https://model-checking.github.io/kani/) and
//! [Prusti](https://viperproject.github.io/prusti-dev/). The generated doc
//! metadata follows a structured `**Requires:**`, `**Ensures:**`, and
//! `**Invariant:**` convention that verifier front-ends can parse.

#![deny(missing_docs)]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

// ── #[requires(expr)] ─────────────────────────────────────────────────────────

/// Declares a precondition that must hold when the annotated function is called.
///
/// In debug builds, the condition is checked via `debug_assert!` at the top of
/// the function body. In release builds it is a no-op.
///
/// A structured doc comment in the form `**Requires:** <condition>` is always emitted.
///
/// # Example
///
/// ```rust
/// use tpt_verify_macros::requires;
///
/// #[requires(x > 0.0)]
/// pub fn sqrt_approx(x: f64) -> f64 {
///     x.sqrt()
/// }
/// ```
///
/// Calling `sqrt_approx(-1.0)` will panic in debug mode with a clear message.
#[proc_macro_attribute]
pub fn requires(attr: TokenStream, item: TokenStream) -> TokenStream {
    let condition: TokenStream2 = attr.into();
    let condition_str = condition.to_string();
    let mut func = parse_macro_input!(item as ItemFn);

    let doc_attr: TokenStream2 = quote! {
        #[doc = concat!("**Requires:** `", #condition_str, "`")]
    };

    let check: TokenStream2 = quote! {
        #[cfg(debug_assertions)]
        debug_assert!(#condition, "Precondition violated: {}", #condition_str);
    };

    // Prepend the debug_assert to the function body
    let orig_stmts = &func.block.stmts;
    func.block = syn::parse_quote! {
        {
            #check
            #(#orig_stmts)*
        }
    };

    quote! {
        #doc_attr
        #func
    }
    .into()
}

// ── #[ensures(expr)] ─────────────────────────────────────────────────────────

/// Declares a postcondition that must hold when the annotated function returns.
///
/// The return value is bound to the name `result` within the condition
/// expression.  In debug builds the condition is checked via `debug_assert!`
/// before the value is returned. In release builds it is a no-op.
///
/// A structured doc comment in the form `**Ensures:** <condition>` is always emitted.
///
/// # Example
///
/// ```rust
/// use tpt_verify_macros::ensures;
///
/// #[ensures(result >= 0)]
/// pub fn abs_value(x: i32) -> i32 {
///     x.abs()
/// }
/// ```
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    let condition: TokenStream2 = attr.into();
    let condition_str = condition.to_string();
    let func = parse_macro_input!(item as ItemFn);

    let doc_attr: TokenStream2 = quote! {
        #[doc = concat!("**Ensures:** `", #condition_str, "`")]
    };

    let vis = &func.vis;
    let sig = &func.sig;
    let attrs = &func.attrs;
    let body = &func.block;

    quote! {
        #doc_attr
        #(#attrs)*
        #vis #sig {
            let result = (move || #body)();
            #[cfg(debug_assertions)]
            debug_assert!(#condition, "Postcondition violated: {}", #condition_str);
            result
        }
    }
    .into()
}

// ── #[invariant(expr)] ───────────────────────────────────────────────────────

/// Declares an invariant that must hold at the annotated point.
///
/// When applied to a function, the condition is checked at function entry
/// in debug builds. This is useful for documenting and enforcing struct or
/// loop invariants expressed as function-level assertions.
///
/// A structured doc comment in the form `**Invariant:** <condition>` is always emitted.
///
/// # Example
///
/// ```rust
/// use tpt_verify_macros::invariant;
///
/// struct Counter { value: u32, max: u32 }
///
/// impl Counter {
///     #[invariant(self.value <= self.max)]
///     pub fn increment(&mut self) {
///         if self.value < self.max {
///             self.value += 1;
///         }
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn invariant(attr: TokenStream, item: TokenStream) -> TokenStream {
    let condition: TokenStream2 = attr.into();
    let condition_str = condition.to_string();
    let mut func = parse_macro_input!(item as ItemFn);

    let doc_attr: TokenStream2 = quote! {
        #[doc = concat!("**Invariant:** `", #condition_str, "`")]
    };

    let check: TokenStream2 = quote! {
        #[cfg(debug_assertions)]
        debug_assert!(#condition, "Invariant violated: {}", #condition_str);
    };

    let orig_stmts = &func.block.stmts;
    func.block = syn::parse_quote! {
        {
            #check
            #(#orig_stmts)*
        }
    };

    quote! {
        #doc_attr
        #func
    }
    .into()
}

// ── #[pure] ──────────────────────────────────────────────────────────────────

/// Marks a function as **pure** — it has no observable side effects.
///
/// This is a zero-cost marker attribute: it changes no runtime behaviour.
/// It serves as documentation and as a signal to external formal verification
/// tools (e.g. Prusti) that the function may be called freely in specifications.
///
/// A doc comment `**Pure:** this function has no observable side effects.` is
/// always emitted.
///
/// # Example
///
/// ```rust
/// use tpt_verify_macros::pure;
///
/// #[pure]
/// pub fn square(x: i32) -> i32 {
///     x * x
/// }
/// ```
#[proc_macro_attribute]
pub fn pure(attr: TokenStream, item: TokenStream) -> TokenStream {
    // The attribute should take no arguments
    let _ = attr;
    let func = parse_macro_input!(item as ItemFn);

    let doc_attr: TokenStream2 = quote! {
        #[doc = "**Pure:** this function has no observable side effects."]
    };

    quote! {
        #doc_attr
        #func
    }
    .into()
}
