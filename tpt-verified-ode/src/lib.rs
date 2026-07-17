//! Rigorously verified ODE integration via interval arithmetic and Picard iteration.
//!
//! # Overview
//!
//! [`OdeSolver`] integrates an initial-value problem `y' = f(t, y)`, `y(t0) = y0`
//! one step at a time. Each [`OdeSolver::step`] returns a closed interval
//! `[y_lo, y_hi]` that is **guaranteed to contain the true solution** at
//! `t0 + h`, assuming the supplied [`IntervalFn`] is a correct enclosure of the
//! right-hand side.
//!
//! Correctness rests on the [Picard–Lindelöf theorem](https://en.wikipedia.org/wiki/Picard%E2%80%93Lindel%C3%B6f_theorem):
//! the Picard iteration
//!
//! ```text
//! y_{k+1}(t) = y0 + ∫_{t0}^{t} f(s, y_k(s)) ds
//! ```
//!
//! converges to the unique solution when it is a contraction on the enclosure.
//! See the [crate README](https://docs.rs/tpt-verified-ode) for the rigor caveat
//! regarding the step-size `h`.
//!
//! # Example
//!
//! ```rust
//! use tpt_exact_math::Rational;
//! use tpt_verified_ode::{IntervalFn, OdeSolver};
//!
//! // y' = c  (constant slope)
//! struct Const;
//! impl IntervalFn for Const {
//!     fn eval(&self, _t: &Rational, _y: &Rational) -> Rational {
//!         Rational::from(2)
//!     }
//! }
//!
//! let mut solver = OdeSolver::new(Const, Rational::from(0), Rational::from(0));
//! let (lo, hi) = solver.step(&Rational::from(1));
//! assert_eq!(lo, Rational::from(2));
//! assert_eq!(hi, Rational::from(2));
//! ```

#![no_std]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/tpt-verified-ode/0.1.0/")]

extern crate alloc;

mod solver;

pub use solver::{IntervalFn, OdeSolver};
