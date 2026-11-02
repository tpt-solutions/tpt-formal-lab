# tpt-fl-verify-macros

[![crates.io](https://img.shields.io/crates/v/tpt-fl-verify-macros.svg)](https://crates.io/crates/tpt-fl-verify-macros)
[![docs.rs](https://docs.rs/tpt-fl-verify-macros/badge.svg)](https://docs.rs/tpt-fl-verify-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Procedural macros for embedding formal contracts directly in Rust code.

**Preconditions, postconditions and invariants next to the code they describe.
Checked in debug builds, removed in release builds, and documented either way.**

## Contents

- [The problem](#the-problem)
- [Macros at a glance](#macros-at-a-glance)
- [Quick start](#quick-start)
- [Preconditions with `requires`](#preconditions-with-requires)
- [Postconditions with `ensures`](#postconditions-with-ensures)
- [Invariants with `invariant`](#invariants-with-invariant)
- [Loop invariants with `loop_invariant!`](#loop-invariants-with-loop_invariant)
- [Purity markers with `pure`](#purity-markers-with-pure)
- [Runtime behaviour](#runtime-behaviour)
- [Supported items](#supported-items)
- [Documentation output](#documentation-output)
- [Working with verifiers](#working-with-verifiers)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

Preconditions and postconditions usually live in comments or in the heads of the
people who wrote the code. Comments drift, and nothing checks them. Contracts
written as attributes sit next to the function, are checked whenever the code runs
in debug mode, and show up in the generated documentation.

## Macros at a glance

| Macro | Effect |
|-------|--------|
| `#[requires(expr)]` | Precondition: must hold when the function is called |
| `#[ensures(expr)]` | Postcondition: must hold on the value the function returns. `result` names that value |
| `#[ensures(\|name\| expr)]` | Postcondition with the returned value named `name` |
| `#[invariant(expr)]` | Invariant: must hold on entry and on exit |
| `loop_invariant!(expr);` | Invariant that must hold where the statement appears |
| `#[pure]` | Marker: the function has no observable side effects |

## Quick start

```rust
use tpt_fl_verify_macros::{ensures, pure, requires};

#[requires(x > 0)]
#[ensures(result > x)]
pub fn double_positive(x: i32) -> i32 {
    x * 2
}

#[pure]
pub fn square(x: i32) -> i32 {
    x * x
}
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-verify-macros = "0.1"
```

## Preconditions with `requires`

```rust
use tpt_fl_verify_macros::requires;

#[requires(index < values.len())]
pub fn get(values: &[u32], index: usize) -> u32 {
    values[index]
}
```

A call that violates the precondition panics in debug builds with the message
`Precondition violated: <condition>`. The check runs before the body.

## Postconditions with `ensures`

The default name for the returned value is `result`:

```rust
use tpt_fl_verify_macros::ensures;

#[ensures(result >= 0)]
pub fn abs_diff(a: i32, b: i32) -> i32 {
    (a - b).abs()
}
```

Name the returned value with a closure when `result` would be unclear:

```rust
use tpt_fl_verify_macros::ensures;

#[ensures(|total| total <= 100)]
pub fn clamped_sum(a: u32, b: u32) -> u32 {
    (a + b).min(100)
}
```

`old(expr)` refers to the value of `expr` on entry. It is captured with
`Clone::clone`, so `expr` must implement `Clone`:

```rust
use tpt_fl_verify_macros::ensures;

#[ensures(result == old(count) + 1)]
pub fn bump(count: u32) -> u32 {
    count + 1
}
```

`return` and `?` in the body behave as usual. The postcondition still runs on the
value that is actually returned.

## Invariants with `invariant`

An invariant is checked on entry and again on exit. It suits methods that must keep
a property true across every call:

```rust
use tpt_fl_verify_macros::invariant;

pub struct Bounded {
    value: u8,
}

impl Bounded {
    #[invariant(self.value <= 100)]
    pub fn set(&mut self, value: u8) {
        self.value = value.min(100);
    }
}
```

## Loop invariants with `loop_invariant!`

`loop_invariant!` is a statement macro. It states a property that holds at the point
where it appears, so it can be placed at the top of a loop body:

```rust
use tpt_fl_verify_macros::loop_invariant;

pub fn sum_to(n: u32) -> u32 {
    let mut total = 0;
    let mut i = 0;
    while i < n {
        // total is 0 + 1 + ... + (i - 1), so 2 * total + i == i * i
        loop_invariant!(2 * total + i == i * i);
        total += i;
        i += 1;
    }
    total
}
```

Loop invariants are checked at run time in debug builds. They do not prove
anything on their own. They document the reasoning and let a verifier use them.

## Purity markers with `pure`

`#[pure]` records that a function has no observable side effects. It does not
check that claim. Use it on functions that a verifier can treat as mathematical
functions:

```rust
use tpt_fl_verify_macros::pure;

#[pure]
pub fn square(x: i64) -> i64 {
    x * x
}
```

`#[pure]` takes no arguments. `#[pure(extra)]` is a compile error.

## Runtime behaviour

- **Debug builds** (`debug_assertions` on): each contract is checked with
  `debug_assert!` and panics with a message if it is violated.
- **Release builds** (`debug_assertions` off): the checks are removed and add no
  runtime cost.

Contracts are not checked in release builds. Do not rely on them for input
validation that must always run. Return an error for that.

## Supported items

The attribute macros accept:

- free functions and methods in `impl` blocks,
- generic functions,
- `async fn`, `const fn` and `unsafe fn`.

One restriction applies. A `const fn` with an early exit (`return` or `?`) cannot
be instrumented, because the instrumentation calls a closure, which is not allowed
in a `const fn`. Such functions fail to compile. Write them without an early exit.

## Documentation output

Each macro adds a structured `#[doc]` section, so the contracts appear in `cargo doc`
output:

```text
**Requires:** `x > 0`
**Ensures:** `result > x`
**Invariant:** `self.value <= 100`
```

## Working with verifiers

The generated documentation follows the `**Requires:**`, `**Ensures:**` and
`**Invariant:**` convention, which verifier front-ends can parse. The macros are
designed to sit alongside tools such as
[Kani](https://model-checking.github.io/kani/) and
[Prusti](https://viperproject.github.io/prusti-dev/). This crate does not run
either of them.

## Limitations

- Contracts are run-time checks, not proofs. A contract that never fails in tests
  is not thereby verified.
- Contracts are removed in release builds, so a release build does not check them.
- `old(expr)` requires `Clone`, and it copies the value on every call.
- `#[pure]` is a claim the macro does not check.

## Crate features and MSRV

- No optional features.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-verify-macros
```

The test suite covers:

- runtime behaviour of each contract (`tests/contracts.rs`, `tests/extended.rs`),
- compile-fail diagnostics for invalid attribute use (`tests/ui/`, run with `trybuild`).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
