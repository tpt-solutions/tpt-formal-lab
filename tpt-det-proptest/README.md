# tpt-det-proptest

[![crates.io](https://img.shields.io/crates/v/tpt-det-proptest.svg)](https://crates.io/crates/tpt-det-proptest)
[![docs.rs](https://docs.rs/tpt-det-proptest/badge.svg)](https://docs.rs/tpt-det-proptest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

Deterministic property-based testing with a fixed-seed PRNG and bisection shrinking.

**Same seed → same test sequence → same shrinking path. Every time.**

## Why deterministic?

Conventional property-based testing (`proptest`, `QuickCheck`) seeds a PRNG from
entropy, so a failure can be hard to reproduce. `tpt-det-proptest` defaults to a
**fixed seed** (override with [`Seed`](crate::Seed)) and produces a byte-for-byte
identical sequence of inputs on every run — essential for debugging flaky
property failures in CI and safety-critical workflows.

## Example

```rust
use tpt_det_proptest::{check, IntRange, Seed};

// A property that holds for all i16 in [0, 1000).
let result = check(100, Seed(42), IntRange::<i16>::new(0, 1000), |x| {
    *x >= 0 && *x < 1000
});
assert!(result.is_ok());
```

When a property fails, [`check`] **shrinks** the counterexample by bisection to
the minimal value that still falsifies the property, then returns it.

## Usage

```toml
[dependencies]
tpt-det-proptest = "0.1"
```

## License

MIT OR Apache-2.0
