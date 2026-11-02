# tpt-fl-exact-math

[![crates.io](https://img.shields.io/crates/v/tpt-fl-exact-math.svg)](https://crates.io/crates/tpt-fl-exact-math)
[![docs.rs](https://docs.rs/tpt-fl-exact-math/badge.svg)](https://docs.rs/tpt-fl-exact-math)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Arbitrary-precision rational arithmetic and interval arithmetic for Rust.

**Eliminates floating-point rounding errors entirely.**

`tpt-fl-exact-math` is the numeric foundation of the `tpt-formal-lab` workspace.
Other crates in the workspace (for example [`tpt-fl-verified-ode`](https://github.com/tpt-solutions/tpt-formal-lab/tree/master/tpt-fl-verified-ode))
use it so that their results can be trusted without a floating-point error budget.

## Contents

- [The problem](#the-problem)
- [Features](#features)
- [Quick start](#quick-start)
- [Rational numbers](#rational-numbers)
- [Interval arithmetic](#interval-arithmetic)
- [Parsing and conversion](#parsing-and-conversion)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

```rust
// Standard f64 arithmetic silently introduces errors:
assert!(0.1_f64 + 0.2_f64 != 0.3_f64); // true: the result is 0.30000000000000004
```

In flight control, financial calculations, or machine-checked proofs, errors like
this accumulate silently and can change a decision. `tpt-fl-exact-math` removes
them at the source: values are stored exactly, and any uncertainty is expressed as
an explicit interval with rigorous bounds.

## Features

- **`Rational`**: exact fractions over arbitrary-precision integers (`num-bigint`).
  Values are always stored in lowest terms with a positive denominator.
- **`Interval`**: closed intervals `[lo, hi]` of `Rational` bounds, with operations
  that propagate worst-case bounds through a chain of arithmetic.
- **Outward rounding**: `Interval::round_outward` widens bounds to a dyadic grid so
  that denominators stay bounded during long computations.
- **Checked division**: `Interval::checked_div` returns `None` instead of panicking
  when the divisor straddles zero.
- **Operator overloading**: `+`, `-`, `*`, `/` and unary `-` on owned and borrowed
  values, and mixed forms such as `&a + &b`.
- **`#![no_std]`** with `alloc`. The crate has no `unsafe` code.

## Quick start

```rust
use tpt_fl_exact_math::{Interval, Rational};

// Exact arithmetic: no rounding ever occurs
let a = Rational::from_frac(1, 10); // 1/10
let b = Rational::from_frac(2, 10); // 2/10
assert_eq!(a + b, Rational::from_frac(3, 10));

// Interval arithmetic: track worst-case bounds
let x = Interval::new(
    Rational::from_frac(1, 10),
    Rational::from_frac(3, 10),
);
let y = Interval::point(Rational::from_frac(1, 2));
let z = x + y;
assert!(z.contains(&Rational::from_frac(6, 10)));
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-exact-math = "0.1"
```

## Rational numbers

[`Rational`](https://docs.rs/tpt-fl-exact-math/latest/tpt_fl_exact_math/struct.Rational.html)
supports the usual field operations, comparison and hashing.

```rust
use tpt_fl_exact_math::Rational;

let third = Rational::from_frac(1, 3);
let sum = third.clone() + third.clone() + third;
assert_eq!(sum, Rational::one());

let half = Rational::from_frac(1, 2);
assert_eq!(half.recip(), Rational::from(2));
assert_eq!(Rational::from_frac(-7, 2).floor(), Rational::from(-4));
assert_eq!(Rational::from_frac(7, 2).ceil(), Rational::from(4));
assert_eq!(Rational::from_frac(2, 3).pow(3), Rational::from_frac(8, 27));
```

Useful methods:

| Method | Purpose |
|--------|---------|
| `from_frac(n, d)` | Construct `n/d` from `i64` parts |
| `new(numer, denom)` | Construct from `BigInt` parts |
| `zero()`, `one()` | Identity elements |
| `is_zero`, `is_negative`, `is_positive` | Sign tests |
| `abs`, `recip`, `pow` | Arithmetic helpers |
| `numer`, `denom` | Access the reduced numerator and denominator |
| `floor`, `ceil`, `round` | Integer rounding, returned as `Rational` |
| `to_i64` | `Some` only when the value is an integer that fits in `i64` |
| `to_f64` | Approximate `f64` for display. This is lossy and must not feed verified results |
| `from_f64` | Exact conversion of a finite `f64` |

## Interval arithmetic

An [`Interval`](https://docs.rs/tpt-fl-exact-math/latest/tpt_fl_exact_math/struct.Interval.html)
is a closed pair `[lo, hi]` with `lo <= hi`. Every operation returns an interval
guaranteed to contain all possible results for inputs drawn from the operand
intervals.

```rust
use tpt_fl_exact_math::{Interval, Rational};

let x = Interval::new(Rational::from(1), Rational::from(2));  // [1, 2]
let y = Interval::new(Rational::from(-1), Rational::from(3)); // [-1, 3]

let sum = x.clone() + y.clone();      // [0, 5]
let prod = x.clone() * y.clone();     // [-2, 6]
assert!(prod.contains(&Rational::from(6)));

// Point intervals behave like exact values
let p = Interval::point(Rational::from_frac(1, 2));
assert_eq!(p.width(), Rational::zero());

// checked_div returns None when the divisor contains zero
let zero_in = Interval::new(Rational::from(-1), Rational::from(1));
assert!(x.checked_div(&zero_in).is_none());
```

Interval methods:

| Method | Purpose |
|--------|---------|
| `new(lo, hi)` / `point(x)` | Construct an interval. `new` panics if `lo > hi` |
| `lo()`, `hi()` | Bounds |
| `width()`, `midpoint()` | Size and centre |
| `contains(&x)`, `contains_interval(&other)` | Membership and inclusion tests |
| `hull(&other)` | Smallest interval containing both |
| `intersect(&other)` | Overlap, or `None` if disjoint |
| `abs`, `sqr`, `pow(n)` | Tight bounds for these functions |
| `bisect()` | Split into two halves at the midpoint |
| `round_outward(bits)` | Widen bounds to a dyadic grid with `bits` of precision |
| `checked_div(&other)` | Division, returning `None` when the divisor contains zero |

Interval arithmetic gives **enclosures**, not exact values. Dependency between
operands widens the result (`x - x` on `[1, 2]` is `[-1, 1]`, not `[0, 0]`), so
keep expressions simple and use `bisect` when a bound is too loose.

## Parsing and conversion

`Rational` implements `FromStr`, accepting both fractions and decimals:

```rust
use tpt_fl_exact_math::Rational;

let a: Rational = "3/4".parse().unwrap();
let b: Rational = "0.125".parse().unwrap();
assert_eq!(a, Rational::from_frac(3, 4));
assert_eq!(b, Rational::from_frac(1, 8));
```

Malformed input returns `ParseRationalError`. Conversions from primitive
integers are provided for `i8`, `i16`, `i32`, `i64`, `i128`, `u8`, `u16`, `u32`,
`u64` and `u128`. `Rational::from_f64` is exact: it returns the binary value of
the float, so `from_f64(0.5)` is `1/2`, while `from_f64(0.1)` is not `1/10`.

## Guarantees

- **Exactness**: `Rational` arithmetic never rounds. Equal values compare equal.
- **Enclosure**: for every interval operation, the result contains every value
  obtainable from values inside the operands.
- **No `unsafe`**: enforced with `#![forbid(unsafe_code)]`.
- **No floating point in the core types.** `to_f64` exists only for display.

## Limitations

- Exact rationals grow with each operation. Use `round_outward` in long
  computations, and expect slower arithmetic than `f64`.
- Interval arithmetic overestimates through dependency. There is no automatic
  monotonicity analysis.
- Transcendental functions (`exp`, `sin`, ...) are not provided. Build them from
  Taylor or Picard bounds, as [`tpt-fl-verified-ode`](https://github.com/tpt-solutions/tpt-formal-lab/tree/master/tpt-fl-verified-ode) does.

## Crate features and MSRV

- No optional features and no default features.
- `#![no_std]`: depends on `alloc`.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-exact-math
```

The unit tests cover big-integer values, negative reciprocals, `Hash`/`Ord`
consistency and multiplication of two negatives.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
