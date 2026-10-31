//! Procedural macros for embedding formal contracts directly in Rust code.
//!
//! # Overview
//!
//! `out-verify-macros` provides attribute macros that annotate functions and
//! methods with verification contracts, plus a statement macro for loop
//! invariants:
//!
//! | Macro | Meaning |
//! |-------|---------|
//! | [`requires`] | Precondition — must hold when the function is called |
//! | [`ensures`]  | Postcondition — must hold on the value the function returns |
//! | [`invariant`] | Invariant — must hold on entry and on exit |
//! | [`loop_invariant!`] | Loop invariant — must hold where the statement appears |
//! | [`pure`]     | Marker — function has no observable side effects |
//!
//! # Runtime behaviour
//!
//! In **debug builds** (`debug_assertions` on), each contract is checked via
//! `debug_assert!` and will panic with a descriptive message if violated.
//!
//! In **release builds** (`debug_assertions` off), the checks are removed and
//! add no runtime overhead.
//!
//! Additionally, all macros inject structured `#[doc]` attributes so that
//! the contracts appear clearly in the generated documentation.
//!
//! # Supported items
//!
//! The attribute macros accept free functions, methods in `impl` blocks,
//! generic functions, `async fn`, `const fn` and `unsafe fn`. Within
//! [`ensures`]:
//!
//! - `old(expr)` refers to the value of `expr` on entry. It is captured with
//!   `Clone::clone`, so `expr` must be `Clone`.
//! - The returned value is named `result` by default. Use a closure to choose
//!   another name: `#[ensures(|total| total >= 0)]`.
//! - `return` and `?` in the body behave as usual; the check still runs on the
//!   value that is returned.
//!
//! `const fn` with an early exit (`return` or `?`) cannot be instrumented,
//! because the instrumentation calls a closure, which is not allowed in a
//! `const fn`. Such functions fail to compile. Write them without an early exit.
//!
//! # Usage
//!
//! ```rust
//! use out_verify_macros::{requires, ensures, pure};
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
#![forbid(unsafe_code)]
extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::{Delimiter, Group, Ident, Span, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{parse_quote, Block, Expr, FnArg, ImplItemFn, ItemFn, Pat, ReturnType, Signature, Type};

// ── Function abstraction ─────────────────────────────────────────────────────

/// A function-like item with a body: a free function or a method.
enum Func {
    Free(ItemFn),
    Method(ImplItemFn),
}

impl Func {
    fn parse(item: TokenStream2) -> syn::Result<Self> {
        match syn::parse2::<ItemFn>(item.clone()) {
            Ok(f) => Ok(Func::Free(f)),
            Err(_) => syn::parse2::<ImplItemFn>(item)
                .map(Func::Method)
                .map_err(|_| {
                    syn::Error::new(
                        Span::call_site(),
                        "contract attributes apply to functions and methods with a body",
                    )
                }),
        }
    }

    fn sig(&self) -> &Signature {
        match self {
            Func::Free(f) => &f.sig,
            Func::Method(m) => &m.sig,
        }
    }

    fn block_mut(&mut self) -> &mut Block {
        match self {
            Func::Free(f) => &mut f.block,
            Func::Method(m) => &mut m.block,
        }
    }
}

impl ToTokens for Func {
    fn to_tokens(&self, tokens: &mut TokenStream2) {
        match self {
            Func::Free(f) => f.to_tokens(tokens),
            Func::Method(m) => m.to_tokens(tokens),
        }
    }
}

/// A check run after the body, on the value the body produced.
struct Epilogue {
    /// Name the returned value is bound to, so the checks can refer to it.
    binding: Ident,
    /// Whether to bind the value when the function returns `()`.
    bind_unit: bool,
    checks: Vec<TokenStream2>,
}

/// Rewrites the body of `func` so that `prologue` runs first. When `epilogue`
/// is given, its checks run on the value the body returns, and that value is
/// then returned.
fn instrument(func: &mut Func, prologue: &[TokenStream2], epilogue: Option<&Epilogue>) {
    let sig = func.sig().clone();
    let block = func.block_mut();
    let stmts = std::mem::take(&mut block.stmts);

    let new_block: Block = match epilogue {
        None => parse_quote!({ #(#prologue)* #(#stmts)* }),
        Some(ep) => {
            let original = quote! { { #(#stmts)* } };
            let value = body_value(&original, &sig);
            let binding = &ep.binding;
            let checks = &ep.checks;
            let unit = matches!(sig.output, ReturnType::Default);

            // A unit function with nothing to bind ends with no tail expression:
            // a trailing `()` would be flagged by `clippy::unused_unit`.
            let (let_stmt, tail) = if unit && !ep.bind_unit {
                (quote! { #value; }, None)
            } else {
                let annotation = if unit {
                    Some(quote! { () })
                } else {
                    annotated_return(&sig)
                };
                let let_stmt = match annotation {
                    Some(ty) => quote! { let #binding: #ty = #value; },
                    None => quote! { let #binding = #value; },
                };
                (let_stmt, Some(quote! { #binding }))
            };

            parse_quote!({
                #(#prologue)*
                #let_stmt
                #(#checks)*
                #tail
            })
        }
    };
    *block = new_block;
}

/// Produces the expression that evaluates the body, honouring early exits.
///
/// Without `return` or `?`, the body is used as a plain block, which also works
/// in `const`, `async` and `unsafe` functions. With an early exit the body runs
/// in a closure, or in an `async` block for `async fn`, so that the exit leaves
/// only the body and the checks still run.
fn body_value(original: &TokenStream2, sig: &Signature) -> TokenStream2 {
    if !has_early_exit(original) {
        return original.clone();
    }
    if sig.asyncness.is_some() {
        return quote! { (async #original).await };
    }
    // A closure's return type must be spelled out for `?` to convert the error.
    // Without `?`, the type is inferred from `return`, which also handles
    // `impl Trait` return types.
    let closure_ret = match &sig.output {
        ReturnType::Type(_, ty) if has_question(original) && !is_impl_trait(ty) => {
            quote! { -> #ty }
        }
        _ => quote! {},
    };
    quote! { (|| #closure_ret #original)() }
}

/// The declared return type, unless it is `impl Trait`, which cannot be written
/// as a `let` annotation.
fn annotated_return(sig: &Signature) -> Option<TokenStream2> {
    match &sig.output {
        ReturnType::Type(_, ty) if !is_impl_trait(ty) => Some(quote! { #ty }),
        _ => None,
    }
}

fn is_impl_trait(ty: &Type) -> bool {
    matches!(ty, Type::ImplTrait(_))
}

// ── Token scanning helpers ───────────────────────────────────────────────────

/// Returns `true` if any token in `ts`, including tokens inside groups, matches.
fn any_token(ts: &TokenStream2, pred: &dyn Fn(&TokenTree) -> bool) -> bool {
    ts.clone().into_iter().any(|tt| {
        pred(&tt)
            || match &tt {
                TokenTree::Group(g) => any_token(&g.stream(), pred),
                _ => false,
            }
    })
}

fn is_punct(tt: &TokenTree, ch: char) -> bool {
    matches!(tt, TokenTree::Punct(p) if p.as_char() == ch)
}

fn has_question(ts: &TokenStream2) -> bool {
    any_token(ts, &|tt| is_punct(tt, '?'))
}

fn has_early_exit(ts: &TokenStream2) -> bool {
    any_token(ts, &|tt| {
        matches!(tt, TokenTree::Ident(i) if i == "return") || is_punct(tt, '?')
    })
}

/// Replaces each `old(expr)` in `ts` with a fresh identifier, and records the
/// pair so the caller can snapshot `expr` before the body runs. Identical
/// expressions share one snapshot.
fn rewrite_old(ts: TokenStream2, snapshots: &mut Vec<(Ident, TokenStream2)>) -> TokenStream2 {
    let mut out = TokenStream2::new();
    let mut iter = ts.into_iter().peekable();
    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Ident(ref id) if id == "old" => {
                let is_call = matches!(
                    iter.peek(),
                    Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis
                );
                if is_call {
                    let group = match iter.next() {
                        Some(TokenTree::Group(g)) => g,
                        _ => unreachable!("peeked a parenthesised group"),
                    };
                    let inner = rewrite_old(group.stream(), snapshots);
                    let key = inner.to_string();
                    let name = match snapshots.iter().find(|(_, e)| e.to_string() == key) {
                        Some((name, _)) => name.clone(),
                        None => {
                            let name = Ident::new(
                                &format!("__tpt_old_{}", snapshots.len()),
                                Span::call_site(),
                            );
                            snapshots.push((name.clone(), inner));
                            name
                        }
                    };
                    out.extend(std::iter::once(TokenTree::Ident(name)));
                } else {
                    out.extend(std::iter::once(tt));
                }
            }
            TokenTree::Group(g) => {
                let inner = rewrite_old(g.stream(), snapshots);
                let mut new = Group::new(g.delimiter(), inner);
                new.set_span(g.span());
                out.extend(std::iter::once(TokenTree::Group(new)));
            }
            other => out.extend(std::iter::once(other)),
        }
    }
    out
}

/// Returns the attribute's tokens, or an error naming the expected argument.
fn required_condition(attr: TokenStream2, message: &str) -> syn::Result<TokenStream2> {
    if attr.is_empty() {
        Err(syn::Error::new(Span::call_site(), message))
    } else {
        Ok(attr)
    }
}

fn doc_attr(text: &str) -> TokenStream2 {
    quote! { #[doc = #text] }
}

/// A debug-only check that panics with `"<message>: <condition>"`.
///
/// It uses `panic!` with a `concat!` literal rather than `debug_assert!` with
/// format arguments, because formatting is not allowed in a `const fn`. The
/// condition text is a format string here, so its braces are doubled.
fn check(message: &str, condition: &TokenStream2, cond_str: &str) -> TokenStream2 {
    let escaped = cond_str.replace('{', "{{").replace('}', "}}");
    quote! {
        #[cfg(debug_assertions)]
        if !(#condition) {
            panic!(concat!(#message, ": ", #escaped));
        }
    }
}

// ── #[requires(expr)] ─────────────────────────────────────────────────────────

/// Declares a precondition that must hold when the annotated function is called.
///
/// In debug builds, the condition is checked via `debug_assert!` at the top of
/// the function body. In release builds it is removed.
///
/// A structured doc comment in the form `**Requires:** <condition>` is always emitted.
///
/// # Example
///
/// ```rust
/// use out_verify_macros::requires;
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
    expand_requires(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_requires(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let condition = required_condition(attr, "#[requires] expects a condition")?;
    let cond_str = condition.to_string();
    let mut func = Func::parse(item)?;

    let doc = doc_attr(&format!("**Requires:** `{cond_str}`"));
    let prologue = [check("Precondition violated", &condition, &cond_str)];
    instrument(&mut func, &prologue, None);

    Ok(quote! { #doc #func })
}

// ── #[ensures(expr)] ─────────────────────────────────────────────────────────

/// Declares a postcondition that must hold on the value the annotated function
/// returns.
///
/// By default the returned value is named `result` in the condition. A closure
/// names it instead: `#[ensures(|out| *out >= 0)]`. In debug builds the
/// condition is checked via `debug_assert!` before the value is returned. In
/// release builds it is removed.
///
/// A structured doc comment in the form `**Ensures:** <condition>` is always emitted.
///
/// # Example
///
/// ```rust
/// use out_verify_macros::ensures;
///
/// #[ensures(result >= 0)]
/// pub fn abs_value(x: i32) -> i32 {
///     x.abs()
/// }
///
/// #[ensures(|v| v > 0)]
/// pub fn one() -> i32 {
///     1
/// }
/// ```
#[proc_macro_attribute]
pub fn ensures(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_ensures(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_ensures(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let (binding, condition) = parse_ensures_args(attr)?;
    let cond_str = condition.to_string();

    let mut snapshots = Vec::new();
    let condition = rewrite_old(condition, &mut snapshots);
    let prologue: Vec<TokenStream2> = snapshots
        .iter()
        .map(|(name, expr)| {
            quote! {
                #[cfg(debug_assertions)]
                let #name = ::core::clone::Clone::clone(&(#expr));
            }
        })
        .collect();

    let mut func = Func::parse(item)?;
    let doc = doc_attr(&format!("**Ensures:** `{cond_str}`"));
    let epilogue = Epilogue {
        binding,
        bind_unit: true,
        checks: vec![check("Postcondition violated", &condition, &cond_str)],
    };
    instrument(&mut func, &prologue, Some(&epilogue));

    Ok(quote! { #doc #func })
}

/// Splits `#[ensures(...)]` arguments into the result binding and the condition.
fn parse_ensures_args(attr: TokenStream2) -> syn::Result<(Ident, TokenStream2)> {
    let attr = required_condition(attr, "#[ensures] expects a condition")?;
    let default_binding = || Ident::new("result", Span::call_site());

    if let Ok(Expr::Closure(closure)) = syn::parse2::<Expr>(attr.clone()) {
        if closure.inputs.len() != 1 {
            return Err(syn::Error::new_spanned(
                &closure.inputs,
                "#[ensures] closure takes exactly one parameter: the result",
            ));
        }
        let pat = match &closure.inputs[0] {
            Pat::Type(pat_type) => &*pat_type.pat,
            other => other,
        };
        return match pat {
            Pat::Ident(pat_ident) => Ok((pat_ident.ident.clone(), closure.body.to_token_stream())),
            _ => Err(syn::Error::new_spanned(
                pat,
                "#[ensures] closure parameter must be a plain identifier",
            )),
        };
    }

    Ok((default_binding(), attr))
}

// ── #[invariant(expr)] ───────────────────────────────────────────────────────

/// Declares an invariant that must hold on entry to and on exit from the
/// annotated function.
///
/// The condition is checked in debug builds. If the receiver is taken by value,
/// the exit check is skipped, because `self` has been moved into the body.
///
/// A structured doc comment in the form `**Invariant:** <condition>` is always emitted.
///
/// # Example
///
/// ```rust
/// use out_verify_macros::invariant;
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
    expand_invariant(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_invariant(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let condition = required_condition(attr, "#[invariant] expects a condition")?;
    let cond_str = condition.to_string();
    let mut func = Func::parse(item)?;

    let receiver_by_value = matches!(
        func.sig().inputs.first(),
        Some(FnArg::Receiver(r)) if r.reference.is_none()
    );

    let doc = doc_attr(&format!("**Invariant:** `{cond_str}`"));
    let prologue = [check("Invariant violated on entry", &condition, &cond_str)];

    if receiver_by_value {
        instrument(&mut func, &prologue, None);
    } else {
        let epilogue = Epilogue {
            binding: Ident::new("__tpt_ret", Span::call_site()),
            bind_unit: false,
            checks: vec![check("Invariant violated on exit", &condition, &cond_str)],
        };
        instrument(&mut func, &prologue, Some(&epilogue));
    }

    Ok(quote! { #doc #func })
}

// ── loop_invariant!(expr) ────────────────────────────────────────────────────

/// Asserts a loop invariant at the point where it appears.
///
/// Place it inside a loop body, where the invariant must hold on every
/// iteration. The check is a `debug_assert!`, so it is removed in release
/// builds. The condition is still type-checked there.
///
/// A panic message reads `Loop invariant violated: <condition>`.
///
/// # Example
///
/// ```rust
/// use out_verify_macros::loop_invariant;
///
/// fn count_to(n: u32) -> u32 {
///     let mut i = 0;
///     while i < n {
///         i += 1;
///         loop_invariant!(i <= n);
///     }
///     i
/// }
///
/// assert_eq!(count_to(5), 5);
/// ```
#[proc_macro]
pub fn loop_invariant(input: TokenStream) -> TokenStream {
    let condition: TokenStream2 = input.into();
    match required_condition(condition, "loop_invariant! expects a condition") {
        Ok(condition) => {
            let cond_str = condition.to_string();
            quote! {
                debug_assert!(#condition, "Loop invariant violated: {}", #cond_str);
            }
            .into()
        }
        Err(err) => err.into_compile_error().into(),
    }
}

// ── #[pure] ──────────────────────────────────────────────────────────────────

/// Marks a function as **pure** — it has no observable side effects.
///
/// This is a zero-cost marker attribute: it changes no runtime behaviour.
/// It serves as documentation and as a signal to external formal verification
/// tools (e.g. Prusti) that the function may be called freely in specifications.
///
/// A doc comment `**Pure:** this function has no observable side effects.` is
/// always emitted. The attribute takes no arguments.
///
/// # Example
///
/// ```rust
/// use out_verify_macros::pure;
///
/// #[pure]
/// pub fn square(x: i32) -> i32 {
///     x * x
/// }
/// ```
#[proc_macro_attribute]
pub fn pure(attr: TokenStream, item: TokenStream) -> TokenStream {
    expand_pure(attr.into(), item.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

fn expand_pure(attr: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    if !attr.is_empty() {
        return Err(syn::Error::new_spanned(attr, "#[pure] takes no arguments"));
    }
    let func = Func::parse(item)?;
    let doc = doc_attr("**Pure:** this function has no observable side effects.");
    Ok(quote! { #doc #func })
}
