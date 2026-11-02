# Changelog

All notable changes to `tpt-fl-exact-math` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Target release: **0.1.0** (part of the first crates.io batch).

---

## [Unreleased]

### Changed (breaking, pre-release)

- `Interval` division by a zero-straddling divisor still panics through `/`, but
  `Interval::checked_div` now returns `None` in that case.
- `Rational::to_f64` no longer returns NaN for large numerators or denominators.
- The private `Rational::min` and `Rational::max` are removed. They shadowed
  `Ord::min` and `Ord::max`.

### Added

- `Rational::floor`, `ceil`, `round`, `to_i64`, `pow` and `from_f64` (exact).
- `FromStr` for `Rational`, accepting `a/b` and decimal notation, with
  `ParseRationalError`.
- `Interval::checked_div`, `round_outward`, `intersect`, `abs`, `sqr`, `pow` and
  `bisect`.
- Reference operator implementations (`&a + &b` and mixed owned/borrowed forms).
- `From` implementations for `i8`, `i16`, `u8`, `u16`, `u32`, `i128` and `u128`.
- Tests for big values, the reciprocal of negative numbers, `Hash`/`Ord`
  consistency and the product of two negative numbers.

### Fixed

- `Interval` division by a non-point divisor panicked for any divisor that was not
  a point, because the reciprocal's bounds were not swapped.

---

## Initial implementation

- `Rational`: arbitrary-precision rational arithmetic backed by `num-bigint`,
  reduced to lowest terms.
- `Interval`: closed-interval arithmetic with monotone propagation.
- `#![no_std]` support, using `alloc`.
- `#![forbid(unsafe_code)]`.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-exact-math
