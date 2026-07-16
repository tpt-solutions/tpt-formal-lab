//! Ergonomic, zero-dependency Rust interface to SMT solvers via SMT-LIB2.
//!
//! # Overview
//!
//! `tpt-smt-bridge` lets you build logical constraints in idiomatic Rust and
//! emit them as valid [SMT-LIB2](https://smtlib.cs.uiowa.edu/) strings that
//! any conforming solver (Z3, CVC5, Bitwuzla, etc.) can consume.
//!
//! There are **no C dependencies and no FFI** — this crate is pure Rust and
//! works anywhere. You pipe the emitted string to a solver binary of your choice.
//!
//! # Quick start
//!
//! ```rust
//! use tpt_smt_bridge::{SmtSolver, Sort, Expr};
//!
//! let mut solver = SmtSolver::new();
//!
//! // Declare constants: x and y are integers
//! solver.declare_const("x", Sort::Int);
//! solver.declare_const("y", Sort::Int);
//!
//! // Assert: x > 0 ∧ y > 0 ∧ x + y = 10
//! let x = Expr::var("x", Sort::Int);
//! let y = Expr::var("y", Sort::Int);
//! let zero = Expr::int(0);
//! let ten  = Expr::int(10);
//!
//! solver.assert(Expr::gt(x.clone(), zero.clone()));
//! solver.assert(Expr::gt(y.clone(), zero.clone()));
//! solver.assert(Expr::eq(Expr::add(x, y), ten));
//!
//! let smtlib2 = solver.emit_check();
//! println!("{smtlib2}");
//! // Pipe this string to: z3 -in < problem.smt2
//! ```
//!
//! # Parsing counterexamples
//!
//! When a solver returns `sat` followed by a model, use [`CounterExample::parse`]
//! to extract the concrete witness values:
//!
//! ```rust
//! use tpt_smt_bridge::CounterExample;
//!
//! let model = r#"(model
//!   (define-fun x () Int 3)
//!   (define-fun y () Int 7)
//! )"#;
//!
//! let ce = CounterExample::parse(model);
//! assert_eq!(ce.get_int("x"), Some(3));
//! assert_eq!(ce.get_int("y"), Some(7));
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/tpt-smt-bridge/0.1.0/")]

mod counter_example;
mod expr;
mod smtlib2;
mod solver;
mod sort;

pub use counter_example::{CeValue, CounterExample};
pub use expr::Expr;
pub use smtlib2::emit_smtlib2;
pub use solver::SmtSolver;
pub use sort::Sort;
