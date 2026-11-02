# Changelog

All notable changes to `tpt-fl-verified-algorithms` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Not yet published: `publish = false` in `Cargo.toml`. See `todo.md`, Phase 16, for the
publish plan.

---

## [Unreleased]

### Changed (breaking, pre-release)

- The contracts come from `tpt-fl-verify-macros` instead of the external `contracts`
  crate.

### Added

- `lower_bound` and `upper_bound`: insertion points for keys that may occur more than
  once.
- `verified_sort_by` and `verified_sort_by_key`, and the `is_sorted_by` predicate.
- `verified_sort` is now a stable O(n log n) merge sort.
- Kani proof harnesses under `cfg(kani)`. They are written but have not been run,
  because Kani is not installed in the development environment.

---

## Initial implementation

- `is_sorted` and `is_permutation`: predicates.
- `verified_sort`: sorts a slice, and in debug builds asserts that the result is
  sorted and a permutation of the input.
- `verified_binary_search`: binary search with a sorted-slice precondition.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-verified-algorithms
