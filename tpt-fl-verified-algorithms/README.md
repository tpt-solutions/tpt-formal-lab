# tpt-fl-verified-algorithms

[![crates.io](https://img.shields.io/crates/v/tpt-fl-verified-algorithms.svg)](https://crates.io/crates/tpt-fl-verified-algorithms)
[![docs.rs](https://docs.rs/tpt-fl-verified-algorithms/badge.svg)](https://docs.rs/tpt-fl-verified-algorithms)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Verified sorting and searching with debug-mode contract checks.

**Same inputs in, same outputs out, and the postconditions are checked for you in
debug builds.**

## Contents

- [What this crate does](#what-this-crate-does)
- [Quick start](#quick-start)
- [Sorting](#sorting)
- [Searching](#searching)
- [Predicates](#predicates)
- [Debug and release behaviour](#debug-and-release-behaviour)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## What this crate does

The standard library's sorting and searching functions are correct, but they do not
check their own results. This crate provides the same kinds of operations with their
correctness properties written down and checked:

- **Predicates** that state a property of a slice: `is_sorted`, `is_sorted_by` and
  `is_permutation`.
- **Sorting** that, in debug builds, asserts the output is sorted **and** is a
  permutation of the input: `verified_sort`, `verified_sort_by` and
  `verified_sort_by_key`.
- **Searching** with a sorted-slice precondition, checked in debug builds, and
  postconditions written as `#[ensures]` contracts: `verified_binary_search`,
  `lower_bound` and `upper_bound`.

The contracts come from [`tpt-fl-verify-macros`](https://github.com/tpt-solutions/tpt-formal-lab/tree/master/tpt-fl-verify-macros).

## Quick start

```rust
use tpt_fl_verified_algorithms::{verified_binary_search, verified_sort};

let mut data = [3, 1, 2];
verified_sort(&mut data);
assert_eq!(data, [1, 2, 3]);

assert_eq!(verified_binary_search(&data, &2), Some(1));
assert_eq!(verified_binary_search(&data, &5), None);
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-verified-algorithms = "0.1"
```

## Sorting

`verified_sort` is a stable merge sort with O(n log n) comparisons:

```rust
use tpt_fl_verified_algorithms::verified_sort;

let mut words = ["pear", "fig", "apple"];
verified_sort(&mut words);
assert_eq!(words, ["apple", "fig", "pear"]);
```

Sort by a comparator or by a key. Both are stable, so equal elements keep their
order:

```rust
use tpt_fl_verified_algorithms::{verified_sort_by, verified_sort_by_key};

// Descending order
let mut xs = [1, 3, 2];
verified_sort_by(&mut xs, |a, b| b.cmp(a));
assert_eq!(xs, [3, 2, 1]);

// Sort records by a key; ties keep their input order
let mut people = [("ana", 31), ("bo", 25), ("cy", 31)];
verified_sort_by_key(&mut people, |p| p.1);
assert_eq!(people, [("bo", 25), ("ana", 31), ("cy", 31)]);
```

`verified_sort` and its variants require `T: Clone`, because the debug-mode check
keeps a copy of the input to compare the output against.

## Searching

`verified_binary_search` requires a sorted slice. It returns the index of an element
equal to `key`, or `None`:

```rust
use tpt_fl_verified_algorithms::verified_binary_search;

let data = [1, 2, 3, 4, 5];
assert_eq!(verified_binary_search(&data, &3), Some(2));
assert_eq!(verified_binary_search(&data, &6), None);
```

With duplicate keys, the returned index can be any of the matches. Use `lower_bound`
and `upper_bound` to find the boundaries of the run:

```rust
use tpt_fl_verified_algorithms::{lower_bound, upper_bound};

let data = [1, 2, 2, 2, 3];
let first = lower_bound(&data, &2); // 1: first index where data[i] >= 2
let after = upper_bound(&data, &2); // 4: first index where data[i] > 2
assert_eq!(&data[first..after], &[2, 2, 2]);
```

`lower_bound` and `upper_bound` return an insertion point, so they also work for keys
that are absent: an absent key gives the same index for both.

## Predicates

The predicates can be used in your own assertions and tests:

```rust
use tpt_fl_verified_algorithms::{is_permutation, is_sorted, is_sorted_by};

assert!(is_sorted(&[1, 2, 2, 3]));
assert!(!is_sorted(&[3, 1, 2]));
assert!(is_sorted_by(&[3, 2, 1], |a, b| b.cmp(a)));
assert!(is_permutation(&[1, 2, 3], &[3, 1, 2]));
assert!(!is_permutation(&[1, 2, 3], &[1, 2, 4]));
```

An empty slice and a single-element slice are always sorted.

## Debug and release behaviour

| Function | Debug build | Release build |
|----------|-------------|---------------|
| `verified_sort`, `verified_sort_by`, `verified_sort_by_key` | Asserts that the output is sorted and a permutation of the input | Sorts only. The checks are removed |
| `verified_binary_search`, `lower_bound`, `upper_bound` | Panics if the slice is not sorted (`is_sorted_slice` precondition) | Precondition not checked. The result is unspecified for unsorted input |

Debug builds use `debug_assertions`, which are on by default for `cargo test` and
off for `cargo build --release`. Run your tests in debug mode to exercise the
checks.

## Guarantees

- **Sorted output.** `verified_sort` returns a sorted slice in every build.
- **Stability.** The sort functions keep equal elements in their original order.
- **Permutation.** The sort only reorders elements. It does not add or remove any.
- **Search results.** A `Some(i)` result satisfies `slice[i] == key`. `None` means no
  element is equal to `key`.
- **No `unsafe` code.**

## Limitations

- **Release builds do not check.** The checks are debug-only. A release build with
  unsorted input to a search returns an unspecified result, so make sure the input is
  sorted by construction.
- **Duplicate keys.** `verified_binary_search` does not promise the first or last
  match. Use `lower_bound` and `upper_bound` for that.
- **Sorting cost.** The debug-mode check copies the slice and compares the result with
  the copy. This makes debug builds slower, and it needs `T: Clone`.
- **Not a proof.** The checks run on the inputs and outputs you give the function.
  Kani harnesses exist under `cfg(kani)` for formal checking, but they have been
  written and not yet run.

## Crate features and MSRV

- No optional features and no dependencies beyond `tpt-fl-verify-macros`.
- `#![no_std]`, using `alloc`.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-verified-algorithms
```

The tests in `tests/algorithms.rs` cover the sort, search and predicate functions,
including the debug-mode checks.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
