//! A bitwise-deterministic simulation engine for safety-critical Rust.
//!
//! # Overview
//!
//! `tpt-deterministic-sim` guarantees that a simulation with identical inputs
//! produces **byte-for-byte identical outputs** regardless of the host CPU
//! architecture, OS, or Rust compiler version — including between x86 and ARM.
//!
//! This is achieved by:
//!
//! 1. **Fixed-point arithmetic** via [`FixedPoint`]: all numeric operations use
//!    integer arithmetic with a compile-time denominator, avoiding all
//!    floating-point non-determinism.
//! 2. **Sorted data structures**: [`DeterministicSim`] uses `BTreeMap` for
//!    entity storage, guaranteeing deterministic iteration order. `HashMap` is
//!    explicitly excluded because it uses random hash seeds.
//! 3. **Ordered system execution**: registered systems are sorted by name and
//!    executed in that order every step.
//!
//! # Feature flags
//!
//! - `exact-math` — enables integration with [`tpt-exact-math`](https://docs.rs/tpt-exact-math)
//!   for lossless `Rational` arithmetic instead of fixed-point.
//!
//! # Quick start
//!
//! ```rust
//! use tpt_deterministic_sim::{DeterministicSim, FixedPoint};
//!
//! type Fp = FixedPoint<1_000_000>; // 6 decimal places of precision
//!
//! let mut sim = DeterministicSim::<Fp>::new();
//!
//! // Spawn an entity with an initial state value
//! let id = sim.spawn(Fp::from_int(10));
//!
//! // Register a system that doubles the entity's value each step
//! sim.add_system("double", |entities| {
//!     for val in entities.values_mut() {
//!         *val = *val + *val;
//!     }
//! });
//!
//! sim.step();
//! assert_eq!(*sim.get(id).unwrap(), Fp::from_int(20));
//! ```

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![doc(html_root_url = "https://docs.rs/tpt-deterministic-sim/0.1.0/")]

mod fixed_point;
mod sim;

pub use fixed_point::FixedPoint;
pub use sim::{DeterministicSim, EntityId};
