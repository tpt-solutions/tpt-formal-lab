# Changelog

All notable changes to `tpt-fl-redundancy` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Not yet published: `publish = false` in `Cargo.toml`. See `todo.md`, Phase 16, for the
publish plan.

---

## [Unreleased]

### Changed (breaking, pre-release)

- `median_vote` and `order_statistic` return `Option<T>`, and return `None` when
  there is no such channel (`N == 0`, or `k >= N`).
- `BitNarrow` is sealed, so downstream code cannot implement it.

### Added

- `bitwise_vote` documents how even-`N` ties are resolved: the tied bit is `0`.
- `approx_vote`, which takes a caller-supplied agreement predicate for values that
  are measurements rather than exact.
- `faulty_channels` and `faulty_channels_by`, which report the channels that disagree
  with the majority.
- `plurality_vote`, which returns the most common value even without a strict
  majority.

---

## Initial implementation

- `Replicated<T, N>`: `N` redundant channel outputs held in a fixed-size array.
- `VoteResult`: `Unanimous`, `Majority` and `NoMajority`.
- `majority_vote`, `median_vote`, `order_statistic` and `bitwise_vote`.
- `#![no_std]` and `#![forbid(unsafe_code)]`.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-redundancy
