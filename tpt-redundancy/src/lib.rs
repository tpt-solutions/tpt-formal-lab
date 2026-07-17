//! N-modular redundancy and voting for fault-tolerant, safety-critical systems.
//!
//! # Overview
//!
//! [`Replicated`] holds `N` independent copies of a value produced by redundant
//! channels (e.g. triple-modular redundancy). A family of voting functions
//! decides the agreed value even when up to `floor(N/2)` channels are faulty:
//!
//! - [`Replicated::majority_vote`] — value-based majority voting, distinguishing
//!   [`VoteResult::Unanimous`], [`VoteResult::Majority`], and
//!   [`VoteResult::NoMajority`].
//! - [`Replicated::median_vote`] — the median value (for odd `N`, this rejects
//!   outliers robustly).
//! - [`Replicated::bitwise_vote`] — bit-by-bit majority over unsigned integers,
//!   robust against stuck-at faults on a data bus.
//!
//! # Safety posture
//!
//! This crate is `#![no_std]` and contains no `unsafe` code, making it suitable
//! for deployment in bare-metal safety-critical environments.
//!
//! # Example
//!
//! ```rust
//! use tpt_redundancy::Replicated;
//!
//! let channels = Replicated::new([10u32, 10u32, 12u32]);
//! let result = channels.majority_vote();
//! assert!(result.is_majority());
//! assert_eq!(result.value(), Some(&10u32));
//! ```

#![no_std]
#![deny(missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/tpt-redundancy/0.1.0/")]

mod vote;

pub use vote::{Replicated, VoteResult};
