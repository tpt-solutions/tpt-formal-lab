# tpt-verified-algorithms

[![crates.io](https://img.shields.io/crates/v/tpt-verified-algorithms.svg)](https://crates.io/crates/tpt-verified-algorithms)
[![docs.rs](https://docs.rs/tpt-verified-algorithms/badge.svg)](https://docs.rs/tpt-verified-algorithms)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

Verified sorting and searching with debug-mode contract checks.

**Same inputs in, same outputs out — and the postconditions are checked for you.**

## What this crate does

`tpt-verified-algorithms` provides implementations of fundamental algorithms
whose correctness properties are *checked*, not just hoped for:

- [`is_sorted`] / [`is_permutation`] — building-block predicates.
- [`verified_sort`] — a sort that, in debug builds, asserts the result is
  sorted **and** a permutation of the input.
- [`verified_binary_search`] — binary search guarded by `#[debug_requires]` /
  `#[debug_ensures]` contracts from the [`contracts`](https://crates.io/crates/contracts)
  crate (panics in debug if the precondition — a sorted slice — is violated;
  zero-cost in release).

## Example

```rust
use tpt_verified_algorithms::{verified_sort, verified_binary_search};

let mut data = [3, 1, 2];
verified_sort(&mut data);
assert_eq!(data, [1, 2, 3]);

let idx = verified_binary_search(&data, &2);
assert_eq!(idx, Some(1));
```

In **debug builds**, calling `verified_binary_search` on an unsorted slice
panics. In **release builds** the contracts are zero-cost.

## Usage

```toml
[dependencies]
tpt-verified-algorithms = "0.1"
```

## License

MIT OR Apache-2.0
