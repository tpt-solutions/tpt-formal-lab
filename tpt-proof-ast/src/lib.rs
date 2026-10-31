//! A strongly-typed, proof-native AST where invalid states are compile errors.
//!
//! # Overview
//!
//! `tpt-proof-ast` provides an Abstract Syntax Tree (AST) framework where
//! every node carries its **kind** — Proposition or Term — encoded in the
//! Rust type system via `PhantomData` markers and a sealed [`Kind`] trait.
//! You cannot accidentally attach a `Term` where a `Formula` (proposition) is
//! expected: the compiler rejects it.
//!
//! # Core types
//!
//! - [`Kind`] — marker trait implemented by [`PropKind`] and [`TermKind`]
//! - [`Expr<K>`] — a typed AST expression node parameterised over a [`Kind`]
//! - [`Formula`] — type alias for `Expr<PropKind>` (logical propositions)
//! - [`Term`] — type alias for `Expr<TermKind>` (mathematical terms)
//! - [`AstBuilder`] — fluent builder for constructing well-typed AST nodes
//!
//! # Example
//!
//! ```rust
//! use tpt_proof_ast::{AstBuilder, Formula, Term};
//!
//! let b = AstBuilder::new();
//!
//! // Build the formula: ∀x. (x > 0) → (x * x > 0)
//! let x = b.var_term("x");
//! let zero = b.int_term(0);
//! let x_pos = b.gt(x.clone(), zero.clone());           // x > 0  (Formula)
//! let x_sq  = b.mul(x.clone(), x.clone());             // x * x  (Term)
//! let sq_pos = b.gt(x_sq, zero);                       // x*x > 0 (Formula)
//! let body  = b.implies(x_pos, sq_pos);                // (Formula)
//! let stmt  = b.forall("x", body);                     // ∀x. ... (Formula)
//!
//! println!("{}", stmt);
//! ```
//!
//! # Type safety
//!
//! A [`Formula`] cannot be passed where a [`Term`] is expected, and vice versa —
//! this is rejected at compile time, not at runtime:
//!
//! ```compile_fail
//! use tpt_proof_ast::AstBuilder;
//!
//! let b = AstBuilder::new();
//! let p = b.var_formula("P"); // Formula
//! let x = b.var_term("x");    // Term
//! let bad = b.add(p, x);      // ERROR: `add` expects two `Term`s, not a `Formula`
//! ```
//!
//! The reverse mistake is rejected too: a `Term` where a `Formula` is expected.
//!
//! ```compile_fail
//! use tpt_proof_ast::AstBuilder;
//!
//! let b = AstBuilder::new();
//! let x = b.var_term("x");
//! let y = b.var_term("y");
//! let bad = b.and(x, y);      // ERROR: `and` expects two `Formula`s, not `Term`s
//! ```

#![deny(missing_docs)]
#![forbid(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/tpt-proof-ast/0.1.0/")]

mod builder;
mod expr;
mod kinds;

pub use builder::AstBuilder;
pub use expr::{Binder, BinderSort, Expr, ExprNode};
pub use kinds::{Kind, PropKind, TermKind};

/// A logical proposition (formula) — `Expr<PropKind>`.
///
/// Use [`AstBuilder`] methods like [`AstBuilder::and`], [`AstBuilder::implies`],
/// [`AstBuilder::forall`], etc., to construct formulae.
pub type Formula = Expr<PropKind>;

/// A mathematical term (expression that has a value) — `Expr<TermKind>`.
///
/// Use [`AstBuilder`] methods like [`AstBuilder::var_term`], [`AstBuilder::int_term`],
/// [`AstBuilder::add`], etc., to construct terms.
pub type Term = Expr<TermKind>;
