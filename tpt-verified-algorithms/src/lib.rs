//! Verified sorting and searching with debug-mode contract checks.
//!
//! # Overview
//!
//! This crate provides fundamental algorithms whose correctness properties are
//! actively checked:
//!
//! - [`is_sorted`] and [`is_permutation`] — predicate building blocks.
//! - [`verified_sort`] — sorts a slice and, in debug builds, asserts the output
//!   is sorted **and** a permutation of the input.
//! - [`verified_binary_search`] — binary search guarded by `#[requires]` /
//!   `#[ensures]` contracts (panics in debug on a violated precondition).
//!
//! # Example
//!
//! ```rust
//! use tpt_verified_algorithms::{verified_sort, verified_binary_search};
//!
//! let mut data = [3, 1, 2];
//! verified_sort(&mut data);
//! assert_eq!(data, [1, 2, 3]);
//! assert_eq!(verified_binary_search(&data, &2), Some(1));
//! ```

#![no_std]
#![deny(missing_docs)]
#![doc(html_root_url = "https://docs.rs/tpt-verified-algorithms/0.1.0/")]

extern crate alloc;

mod predicates;
mod search;
mod sort;

pub use predicates::{is_permutation, is_sorted};
pub use search::verified_binary_search;
pub use sort::verified_sort;
