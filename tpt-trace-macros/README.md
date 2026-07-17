# tpt-trace-macros

[![crates.io](https://img.shields.io/crates/v/tpt-trace-macros.svg)](https://crates.io/crates/tpt-trace-macros)
[![docs.rs](https://docs.rs/tpt-trace-macros/badge.svg)](https://docs.rs/tpt-trace-macros)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

The `#[traces("REQ-…")]` attribute macro — link code to the requirements it implements.

## The problem

In regulated, safety-critical development (DO-178C, ISO 26262, IEC 61508) every
line of code must be traceable to a requirement. Manually keeping that trace
matrix in sync with the code is error-prone.

## Solution

Annotate a function with `#[traces("REQ-123", "REQ-456")]`. The macro:

1. Injects a **`**Traces:** REQ-123, REQ-456`** doc comment so the links appear
   in `cargo doc`.
2. Generates a `const __TPT_TRACES_<FN_NAME_UPPER>: &[&str]` listing the exact
   requirement ids — machine-readable, so tooling can verify coverage.

```rust
use tpt_trace_macros::traces;

/// Computes the safe braking distance.
#[traces("REQ-BRAKE-001", "REQ-SAFE-009")]
pub fn braking_distance(speed: f64) -> f64 {
    0.5 * speed * speed
}
```

The macro generates:

```rust,ignore
const __TPT_TRACES_BRAKING_DISTANCE: &[&str] = &["REQ-BRAKE-001", "REQ-SAFE-009"];
```

## Usage

```toml
[dependencies]
tpt-trace-macros = "0.1"
```

## License

MIT OR Apache-2.0
