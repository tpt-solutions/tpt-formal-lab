# tpt-exact-math

[![crates.io](https://img.shields.io/crates/v/tpt-exact-math.svg)](https://crates.io/crates/tpt-exact-math)
[![docs.rs](https://docs.rs/tpt-exact-math/badge.svg)](https://docs.rs/tpt-exact-math)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

Arbitrary-precision rational arithmetic and interval arithmetic for Rust.

**Eliminates floating-point rounding errors entirely.**

## The problem

```rust
// Standard f64 arithmetic silently introduces errors:
assert!(0.1_f64 + 0.2_f64 != 0.3_f64); // true — this is 0.30000000000000004
```

In flight control, financial calculations, or formal proofs, these silent errors
are catastrophic. `tpt-exact-math` makes them impossible.

## Solution

```rust
use tpt_exact_math::Rational;

let a = Rational::from_frac(1, 10); // exactly 1/10
let b = Rational::from_frac(2, 10); // exactly 2/10
assert_eq!(a + b, Rational::from_frac(3, 10)); // exactly 3/10 — always
```

## Interval arithmetic

Track worst-case error bounds through a computation:

```rust
use tpt_exact_math::{Interval, Rational};

// x is uncertain: somewhere in [0.1, 0.3]
let x = Interval::new(
    Rational::from_frac(1, 10),
    Rational::from_frac(3, 10),
);
// y is exactly 0.5
let y = Interval::point(Rational::from_frac(1, 2));

// z = x + y is guaranteed to be in [0.6, 0.8]
let z = x + y;
assert!(z.contains(&Rational::from_frac(7, 10)));
```

## Features

- `#![no_std]` — works on embedded targets (requires `alloc`)
- Backed by `num-bigint` — arbitrary precision, no overflow ever
- Always fully reduced — `4/6` is immediately stored as `2/3`
- Full `Add`, `Sub`, `Mul`, `Div`, `Neg`, `Ord`, `Display` implementations
- Zero unsafe code

## Usage

```toml
[dependencies]
tpt-exact-math = "0.1"
```

## License

MIT OR Apache-2.0
