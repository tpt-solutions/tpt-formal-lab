# tpt-verify-macros

[![crates.io](https://img.shields.io/crates/v/tpt-verify-macros.svg)](https://crates.io/crates/tpt-verify-macros)
[![docs.rs](https://docs.rs/tpt-verify-macros/badge.svg)](https://docs.rs/tpt-verify-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

Procedural macros for embedding formal verification contracts in Rust code.

**Zero runtime cost in release. Panic-on-violation in debug. Always documented.**

## Macros

| Macro | Effect |
|-------|--------|
| `#[requires(expr)]` | Precondition — must hold when function is called |
| `#[ensures(expr)]`  | Postcondition — must hold when function returns (`result` is the return value) |
| `#[invariant(expr)]` | Invariant — checked at function entry in debug |
| `#[pure]`           | Marker — function has no observable side effects |

## Example

```rust
use tpt_verify_macros::{requires, ensures, pure};

#[requires(x > 0)]
#[ensures(result > 0)]
pub fn double_positive(x: i32) -> i32 {
    x * 2
}

#[pure]
pub fn square(x: i32) -> i32 {
    x * x
}
```

In **debug builds**: calling `double_positive(-1)` panics with
`Precondition violated: x > 0`.

In **release builds**: the macros expand to nothing — zero overhead.

## Documentation injection

Contracts are also injected as structured doc comments:

```
**Requires:** `x > 0`
**Ensures:** `result > 0`
```

These appear in your `cargo doc` output and are parseable by verification
tools such as [Kani](https://model-checking.github.io/kani/) and
[Prusti](https://viperproject.github.io/prusti-dev/).

## Usage

```toml
[dependencies]
tpt-verify-macros = "0.1"
```

## License

MIT OR Apache-2.0
