# tpt-redundancy

[![crates.io](https://img.shields.io/crates/v/tpt-redundancy.svg)](https://crates.io/crates/tpt-redundancy)
[![docs.rs](https://docs.rs/tpt-redundancy/badge.svg)](https://docs.rs/tpt-redundancy)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

N-modular redundancy and voting framework for fault-tolerant, safety-critical Rust.

**Tolerate `floor(N/2)` faulty channels by voting over `N` independent replicas.**

## The problem

In safety-critical systems (aerospace, medical, automotive), a single computation
may be corrupted by a hardware fault. Run the same computation `N` times on
independent hardware and vote over the results: as long as fewer than half the
channels are faulty, the majority outcome is correct.

## Solution

```rust
use tpt_redundancy::Replicated;

// Three redundant channels of a sensor reading.
let channels = Replicated::new([10u32, 10u32, 12u32]);

// Majority vote accepts the value held by the two healthy channels.
let result = channels.majority_vote();
assert!(result.is_majority());
assert_eq!(result.value(), Some(&10u32));

// Bitwise voting is robust against stuck-at faults on unsigned buses.
let bit = Replicated::new([0b1010u8, 0b1010u8, 0b0010u8]).bitwise_vote();
assert_eq!(bit, 0b1010u8);
```

## Voting strategies

| Strategy | Function | Use case |
|----------|----------|----------|
| Majority (value) | [`Replicated::majority_vote`](crate::Replicated::majority_vote) | General comparable values |
| Median | [`Replicated::median_vote`] | Odd `N`, reject outliers |
| Bitwise | [`Replicated::bitwise_vote`] | Unsigned integer buses |

## Features

- `#![no_std]` and `#![deny(unsafe_code)]` — suitable for bare-metal safety systems.
- Zero external dependencies.
- `const N: usize` generic — vote over any number of channels at compile time.

## Usage

```toml
[dependencies]
tpt-redundancy = "0.1"
```

## License

MIT OR Apache-2.0
