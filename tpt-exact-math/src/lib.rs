//! Arbitrary-precision rational arithmetic and interval arithmetic for Rust.
//!
//! # Overview
//!
//! `tpt-exact-math` eliminates floating-point rounding errors entirely. It provides:
//!
//! - [`Rational`] — exact fractional numbers backed by arbitrary-precision integers.
//!   `0.1 + 0.2` is represented as `3/10`, not `0.30000000000000004`.
//! - [`Interval`] — pairs of `Rational` bounds that track the minimum and maximum
//!   possible value through a chain of operations, guaranteeing worst-case error bounds.
//!
//! # Feature flags
//!
//! This crate is `#![no_std]` and requires only `alloc`.
//!
//! # Quick start
//!
//! ```rust
//! use tpt_exact_math::{Rational, Interval};
//!
//! // Exact arithmetic — no rounding ever occurs
//! let a = Rational::from_frac(1, 10); // 1/10
//! let b = Rational::from_frac(2, 10); // 2/10
//! assert_eq!(a + b, Rational::from_frac(3, 10));
//!
//! // Interval arithmetic — track worst-case bounds
//! let x = Interval::new(
//!     Rational::from_frac(1, 10),
//!     Rational::from_frac(3, 10),
//! );
//! let y = Interval::point(Rational::from_frac(1, 2));
//! let z = x + y;
//! assert!(z.contains(&Rational::from_frac(6, 10)));
//! ```

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/tpt-exact-math/0.1.0/")]

extern crate alloc;

mod interval;
mod rational;

pub use interval::Interval;
pub use rational::Rational;
