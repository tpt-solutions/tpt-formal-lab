# Changelog

All notable changes to `tpt-fl-verify-macros` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Target release: **0.1.0** (part of the first crates.io batch).

---

## [Unreleased]

### Changed (breaking, pre-release)

- `#[invariant]` is checked on exit as well as on entry.
- Contract attributes accept methods and items in `impl` blocks, and work on
  `async`, `const` and `unsafe` functions.

### Added

- `#[ensures(|name| ...)]` names the returned value.
- `old(expr)` in postconditions, captured with `Clone::clone`.
- `loop_invariant!` statement macro.
- `trybuild` compile-fail tests for invalid attribute arguments, including
  `#[pure]` with arguments and `#[requires]` with no condition.

---

## Initial implementation

- `#[requires(expr)]`: precondition.
- `#[ensures(expr)]`: postcondition, with `result` for the returned value.
- `#[invariant(expr)]`: invariant.
- `#[pure]`: side-effect-free marker.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-verify-macros
