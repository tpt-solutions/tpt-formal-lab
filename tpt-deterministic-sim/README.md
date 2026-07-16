# tpt-deterministic-sim

[![crates.io](https://img.shields.io/crates/v/tpt-deterministic-sim.svg)](https://crates.io/crates/tpt-deterministic-sim)
[![docs.rs](https://docs.rs/tpt-deterministic-sim/badge.svg)](https://docs.rs/tpt-deterministic-sim)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

A bitwise-deterministic simulation engine for safety-critical Rust.

**Identical inputs → byte-for-byte identical outputs on x86, ARM, RISC-V, everywhere.**

## The problem

Standard simulation engines use floating-point arithmetic and `HashMap` (with
random hash seeds), so the same simulation diverges slightly depending on the
CPU architecture and compiler flags. This makes formal verification and replay
debugging impossible.

## Solution

`tpt-deterministic-sim` enforces determinism at every level:

- **`FixedPoint<DENOM>`** — all arithmetic uses pure integer operations, no
  floating-point. Results are bitwise identical on every target.
- **`BTreeMap`** — entity storage is always sorted. No random hash seeds, no
  non-deterministic iteration.
- **Named systems run alphabetically** — execution order is always the same,
  regardless of registration order.

## Quick start

```rust
use tpt_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000_000>; // 6 decimal places

let mut sim = DeterministicSim::<Fp>::new();
let entity = sim.spawn(Fp::from_int(10));

sim.add_system("gravity", |entities| {
    for v in entities.values_mut() {
        *v = *v - FixedPoint::from_raw(9_800); // -9.800 m/s²
    }
});

sim.step();
// Exact same result on a Mac, a Linux server, and an embedded ARM chip.
```

## Feature flags

| Feature | Description |
|---------|-------------|
| `exact-math` | Use `tpt-exact-math::Rational` for lossless arithmetic |

## Usage

```toml
[dependencies]
tpt-deterministic-sim = "0.1"
```

## License

MIT OR Apache-2.0
