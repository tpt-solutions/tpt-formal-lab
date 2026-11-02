# tpt-fl-det-proptest

[![crates.io](https://img.shields.io/crates/v/tpt-fl-det-proptest.svg)](https://crates.io/crates/tpt-fl-det-proptest)
[![docs.rs](https://docs.rs/tpt-fl-det-proptest/badge.svg)](https://docs.rs/tpt-fl-det-proptest)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Deterministic property-based testing with a fixed-seed PRNG and bisection shrinking.

**Same seed, same test sequence, same shrinking path. Every time.**

## Contents

- [Why deterministic?](#why-deterministic)
- [Quick start](#quick-start)
- [How a run works](#how-a-run-works)
- [Strategies](#strategies)
- [Combinators](#combinators)
- [Shrinking](#shrinking)
- [Writing properties](#writing-properties)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## Why deterministic?

Conventional property-based testing libraries such as `proptest` and `QuickCheck` seed
their random number generator from the operating system. A failure found on one run
can be hard to reproduce on the next.

`tpt-fl-det-proptest` takes the seed as an explicit argument, using the [`Seed`] type.
Given the same seed, the same inputs are generated in the same order, and any failure
shrinks to the same minimal value. A CI failure can be reproduced on a developer's
machine with the seed printed in the log. That property matters for safety-critical
workflows, where a flaky test is a finding that must be explained.

The crate is `#![no_std]` with `alloc`, has no dependencies, and has no `unsafe` code.

## Quick start

```rust
use tpt_fl_det_proptest::{check, IntRange, Seed};

// Check that every value in 0..100 is below 100
let result = check(100, Seed(1), IntRange::<i32>::new(0, 100), |x| *x < 100);
assert!(result.is_ok());
```

Add the crate to your project:

```toml
[dev-dependencies]
tpt-fl-det-proptest = "0.1"
```

## How a run works

`check(iterations, seed, strategy, property)`:

1. Creates an `Xorshift64` generator from `seed`. The seed is mixed through
   splitmix64, so nearby seeds produce unrelated sequences.
2. Draws `iterations` values from `strategy` and calls `property` on each one.
3. If a value fails, shrinks it by bisection to find a smaller failing value.
4. Returns `Ok(())` if every value passed, or `Err(CounterExample { original, minimal })`
   otherwise.

```rust
use tpt_fl_det_proptest::{check, IntRange, Seed};

// Fails for any x >= 50. The first failing draw is shrunk to the boundary.
let result = check(100, Seed(1), IntRange::<i32>::new(0, 100), |x| *x < 50);
let ce = result.unwrap_err();
assert_eq!(ce.minimal, 50);
```

`CounterExample` has two fields. `original` is the first value that failed. `minimal` is
the smallest failing value found by shrinking. Report `minimal` in test output, and keep
the seed so the run can be repeated.

## Strategies

A [`Strategy`] draws values of one type from an `Xorshift64` generator:

| Strategy | Generates |
|----------|-----------|
| `IntRange::new(low, high)` | Integers in the half-open range `[low, high)`. Panics if `low >= high` |
| `IntRange::inclusive(low, high)` | Integers in the closed range `[low, high]`. Panics if `low > high` |
| `AnyBool` | `true` or `false` |
| `optional(inner)` | `None` or `Some` of a value from `inner` |
| `vec_of(element, min_len, max_len)` | A `Vec` whose length is between the two bounds |
| tuples of 2 and 3 strategies | Tuples of values |

`IntRange` supports `i8` to `i128`, `u8` to `u128`, `isize` and `usize`.

Implement `Strategy` yourself for other types. A strategy needs `generate`. It may also
implement `shrink_bounds`, `index_of` and `at_index` to take part in bisection
shrinking. Without them, a failing value is reported as it was first drawn.

## Combinators

`StrategyExt` adds methods to every strategy:

```rust
use tpt_fl_det_proptest::{check, IntRange, Seed, StrategyExt};

// Even numbers only
let evens = IntRange::<i32>::new(0, 100).map(|x| x * 2);

// Odd numbers only, by rejecting the rest
let odds = IntRange::<i32>::new(0, 100).filter(|x| x % 2 != 0);

assert!(check(100, Seed(3), evens, |x| x % 2 == 0).is_ok());
assert!(check(100, Seed(3), odds, |x| x % 2 == 1).is_ok());
```

- `map(f)` transforms each generated value.
- `filter(p)` keeps only values that satisfy `p`. It draws again on rejection. It panics
  after `MAX_FILTER_REJECTIONS` (1,000) rejections in a row, which usually means the
  predicate can never be satisfied.
- `one_of(choices)` picks one of several boxed strategies that produce the same type.

## Shrinking

Shrinking uses bisection over the strategy's index space. For an integer range, the
smallest failing value is found in O(log n) property calls, rather than by trying each
candidate. Shrinking only applies to strategies that expose an index space. The
`map` and `filter` combinators do not, so their failing values are reported as drawn.

## Writing properties

A property is a closure that returns `bool`:

```rust
use tpt_fl_det_proptest::{check, IntRange, Seed};

// For negative x, the absolute value is the negation of x
let result = check(200, Seed(42), IntRange::<i32>::inclusive(-1_000, 1_000), |x| {
    *x >= 0 || x.abs() == -*x
});
assert!(result.is_ok());
```

A property that panics is not caught. The crate is `no_std`, and catching panics needs
`std`. Write properties that return `false` instead, so shrinking can run.

Use `check` in ordinary `#[test]` functions. Choose the number of iterations for the test's
time budget, and keep the seed fixed in the test so that failures reproduce.

## Guarantees

- **Reproducible.** The same `Seed`, strategy and iteration count give the same values
  and the same shrinking path, on every platform. Every `u64` is a valid seed, including
  `0`.
- **Independent of the environment.** No system randomness, clock or thread state is
  read.
- **Bounded shrinking.** Shrinking runs a bounded number of bisection steps.
- **`no_std`, no dependencies, no `unsafe` code.**

## Limitations

- **Not a full property-testing framework.** There is no automatic generation for
  arbitrary types, no persistence of failing seeds between runs, and no stateful or
  model-based testing.
- **Shrinking depends on the strategy.** Values from `map` and `filter` are reported
  without shrinking.
- **Panicking properties are not caught.** Return `false` instead of panicking.
- **Statistical coverage is the caller's job.** A fixed seed explores the same inputs
  each run. Vary the seed in a separate, deliberate test if you want wider coverage.

## Crate features and MSRV

- No optional features and no dependencies.
- `#![no_std]`, using `alloc` for boxed strategies.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-det-proptest
```

The unit tests cover the RNG, the strategies and the combinators. `tests/prop.rs`
contains property tests for the crate itself.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
