//! Deterministic property-based testing for `no_std` + `alloc`.
//!
//! # Overview
//!
//! This crate provides a small, dependency-free property-testing framework that
//! is **fully deterministic**: a given [`Seed`] always produces the same
//! sequence of generated inputs and the same shrinking path. That makes property
//! failures reproducible in CI and safety-critical workflows.
//!
//! Core pieces:
//!
//! - [`Xorshift64`] — a deterministic PRNG.
//! - [`Strategy`] — a trait for value generators; [`IntRange`] and [`AnyBool`]
//!   are provided implementations.
//! - [`check`] — runs a property over many generated values, shrinking any
//!   counterexample by bisection.
//!
//! # Example
//!
//! ```rust
//! use tpt_det_proptest::{check, IntRange, Seed};
//!
//! let result = check(100, Seed(7), IntRange::<i32>::new(0, 100), |x| *x < 100);
//! assert!(result.is_ok());
//! ```

#![no_std]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/tpt-det-proptest/0.1.0/")]

extern crate alloc;

mod check;
mod rng;
mod strategy;

pub use check::{check, CounterExample, TestResult};
pub use rng::Xorshift64;
pub use strategy::{AnyBool, IntRange, Seed, Strategy};
