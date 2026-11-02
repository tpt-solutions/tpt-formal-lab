# Changelog

All notable changes to `tpt-fl-det-proptest` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Not yet published: `publish = false` in `Cargo.toml`. See `todo.md`, Phase 16, for the
publish plan.

---

## [Unreleased]

### Changed (breaking, pre-release)

- `Strategy` index space is `u128`, so every integer range fits, including the full
  range of `u128`.
- Shrinking looks up the index of a failing value directly, instead of scanning
  linearly.
- Range sampling is unbiased.
- Seeds are mixed through splitmix64 before they are used.

### Added

- `IntRange::inclusive(low, high)` for the closed range `[low, high]`.
- `IntRange` support for `usize`, `isize`, `i128` and `u128`.
- Tuple strategies for 2 and 3 elements.
- `optional`, `vec_of`, and `StrategyExt::{map, filter}`.
- `one_of`, which picks one of several boxed strategies.

---

## Initial implementation

- `Xorshift64`: a deterministic PRNG.
- `Seed`: the seed type for a property-test run.
- `Strategy`: a trait for value generators, with `IntRange` and `AnyBool` as the
  provided implementations.
- `check`: runs a property over generated values, shrinking a counterexample by
  bisection.
- `#![no_std]` with `alloc`.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-det-proptest
