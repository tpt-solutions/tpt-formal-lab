# tpt-fl-redundancy

[![crates.io](https://img.shields.io/crates/v/tpt-fl-redundancy.svg)](https://crates.io/crates/tpt-fl-redundancy)
[![docs.rs](https://docs.rs/tpt-fl-redundancy/badge.svg)](https://docs.rs/tpt-fl-redundancy)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

N-modular redundancy and voting for fault-tolerant, safety-critical Rust.

**Run a computation on `N` independent channels and vote over the results. A strict
majority of correct channels yields the correct answer, even when some channels are
faulty.**

## Contents

- [The problem](#the-problem)
- [The voting model](#the-voting-model)
- [Quick start](#quick-start)
- [Value voting](#value-voting)
- [Approximate voting](#approximate-voting)
- [Median and order statistics](#median-and-order-statistics)
- [Bitwise voting](#bitwise-voting)
- [Fault reports](#fault-reports)
- [Choosing a voter](#choosing-a-voter)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

In aerospace, medical and automotive systems, a single computation can be corrupted
by a hardware fault such as a bit flip or a stuck sensor. Running the computation
once is not enough to detect this. Running it on several independent channels and
voting over the answers is a standard defence: if fewer than half the channels are
wrong, the majority answer is right.

## The voting model

[`Replicated<T, N>`](https://docs.rs/tpt-fl-redundancy/latest/tpt_fl_redundancy/struct.Replicated.html)
holds the `N` channel outputs. A voter reads them and returns a decision.

| Voter | Returns | Use it when |
|-------|---------|-------------|
| `majority_vote` | `VoteResult<T>` | Values must match exactly |
| `approx_vote` | `VoteResult<T>` | Values are measurements and must agree within a tolerance |
| `plurality_vote` | `Option<T>` | You want the most common value even without a majority |
| `median_vote` | `Option<T>` | Values are ordered and outliers should be ignored |
| `order_statistic(k)` | `Option<T>` | You need the `k`-th smallest channel |
| `bitwise_vote` | `T` | Values are unsigned words on a data bus |

A strict majority tolerates `floor((N - 1) / 2)` faulty channels. Three channels
tolerate one fault, and five tolerate two.

The result of a value vote is a `VoteResult`:

- `Unanimous(v)`: every channel reported `v`.
- `Majority(v)`: more than half reported `v`, but not all.
- `NoMajority`: no value reached a strict majority.

## Quick start

```rust
use tpt_fl_redundancy::{Replicated, VoteResult};

// Three channels; one has a fault
let channels = Replicated::new([10u32, 10, 12]);
let result = channels.majority_vote();

assert!(matches!(result, VoteResult::Majority(10)));
assert_eq!(result.value(), Some(&10u32));
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-redundancy = "0.1"
```

## Value voting

`majority_vote` compares values with `Eq`:

```rust
use tpt_fl_redundancy::{Replicated, VoteResult};

let all_agree = Replicated::new([7u8, 7, 7]).majority_vote();
assert_eq!(all_agree, VoteResult::Unanimous(7));

let split = Replicated::new([1u8, 2, 3]).majority_vote();
assert_eq!(split, VoteResult::NoMajority);
assert_eq!(split.value(), None);
```

With an even `N`, an exact tie is `NoMajority`. A safety monitor should treat
`NoMajority` as a fault, not pick a value.

## Approximate voting

Floating-point channels rarely agree exactly. `approx_vote` takes an `agree`
predicate that decides whether two readings are close enough:

```rust
use tpt_fl_redundancy::{Replicated, VoteResult};

// Three sensors read the same quantity with a tolerance of 0.5
let readings = Replicated::new([10.0_f64, 10.2, 13.0]);
let close = |a: &f64, b: &f64| (a - b).abs() <= 0.5;

assert!(matches!(readings.approx_vote(close), VoteResult::Majority(v) if v == 10.0));
```

Choose the tolerance from the sensor specification. A tolerance that is too loose
lets a faulty channel vote with the good ones.

## Median and order statistics

The median is robust to outliers when `N` is odd:

```rust
use tpt_fl_redundancy::Replicated;

let r = Replicated::new([9u32, 1, 3, 2, 4]); // sorted: 1, 2, 3, 4, 9
assert_eq!(r.median_vote(), Some(3));
assert_eq!(r.order_statistic(0), Some(1)); // smallest
assert_eq!(r.order_statistic(4), Some(9)); // largest
assert_eq!(r.order_statistic(5), None);    // out of range
```

`median_vote` is `order_statistic(N / 2)`. For even `N` it returns the upper of the
two middle values, so the median is not robust there. Prefer odd `N` for a median
voter.

## Bitwise voting

`bitwise_vote` votes on each bit position separately. This tolerates a stuck bit on
one channel's data bus even when the whole word differs:

```rust
use tpt_fl_redundancy::Replicated;

// Two channels agree that bits 1 and 3 are set; one has bit 1 stuck at 0
let r = Replicated::new([0b1010u8, 0b1010, 0b0010]);
assert_eq!(r.bitwise_vote(), 0b1010);
```

Bitwise voting is implemented for `u8`, `u16`, `u32` and `u64`. The trait that
provides it is sealed.

**Even `N` ties.** When `N` is even and exactly half the channels have a bit set, that
bit is `0` and no tie is reported. The result is biased toward zero. Use an odd `N`
when the bitwise result must not depend on tie-breaking.

## Fault reports

Voting tells you the answer. The fault report tells you which channels to blame:

```rust
use tpt_fl_redundancy::Replicated;

let r = Replicated::new([7u8, 7, 1]);
assert_eq!(r.faulty_channels(), Some([false, false, true]));
assert_eq!(r.plurality_vote(), Some(7));

// No strict majority, so no channel can be blamed
assert_eq!(Replicated::new([1u8, 2]).faulty_channels(), None);
```

- `faulty_channels()` returns `None` when no strict majority exists. Otherwise entry
  `i` is `true` if channel `i` disagrees with the majority value.
- `faulty_channels_by(agree)` does the same with an approximate predicate.
- `plurality_vote()` returns the most common value. Ties go to the value that
  appears first.

## Choosing a voter

- **Discrete state** (modes, commands, counters): `majority_vote`, and treat
  `NoMajority` as a fault.
- **Analogue measurements**: `approx_vote` with a tolerance from the sensor spec, or
  `median_vote` with odd `N`.
- **Raw data words**: `bitwise_vote` with odd `N`.
- **Monitoring**: `faulty_channels` to log which channel disagrees.

## Guarantees

- **Deterministic.** Each voter is a pure function of its channels. There is no
  randomness and no ordering dependence beyond the documented tie-breaks.
- **No `unsafe` code.**
- **No allocation.** The crate is `#![no_std]`. Voting uses only the `N` channels in
  the value and fixed-size arrays.

## Limitations

- The crate votes on values. It does not detect a fault in a channel's computation,
  and it does not restart or resynchronise channels. Those are the caller's job.
- Voting assumes the channels are independent. Two channels that share hardware or
  software can fail together, and no voter can detect that.
- `approx_vote` and `median_vote` depend on the tolerance and on `N`. The crate
  cannot pick either for you.
- `bitwise_vote` truncates to `T`, and its even-`N` tie-break is biased toward zero.

## Crate features and MSRV

- No optional features and no dependencies.
- `#![no_std]`, with no `alloc` requirement.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-redundancy
```

The unit tests in `src/vote.rs` and `src/approx.rs` check each voter and its tie
rules. `tests/vote.rs` covers the public voting behaviour, including ties and
even `N`.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
