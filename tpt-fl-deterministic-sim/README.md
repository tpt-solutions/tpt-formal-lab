# tpt-fl-deterministic-sim

[![crates.io](https://img.shields.io/crates/v/tpt-fl-deterministic-sim.svg)](https://crates.io/crates/tpt-fl-deterministic-sim)
[![docs.rs](https://docs.rs/tpt-fl-deterministic-sim/badge.svg)](https://docs.rs/tpt-fl-deterministic-sim)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A bitwise-deterministic simulation engine for safety-critical Rust.

**Identical inputs produce byte-for-byte identical outputs on x86, ARM and RISC-V.**

## Contents

- [The problem](#the-problem)
- [How determinism is achieved](#how-determinism-is-achieved)
- [Quick start](#quick-start)
- [Fixed-point numbers](#fixed-point-numbers)
- [Entities and systems](#entities-and-systems)
- [Structural changes](#structural-changes)
- [Snapshots and state hashing](#snapshots-and-state-hashing)
- [Feature flags](#feature-flags)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

Common simulation engines use floating-point arithmetic and hash maps with random
seeds. The same simulation can then diverge slightly between CPU architectures,
compiler versions or even runs. That makes replay debugging and formal reasoning
about the simulation very hard.

## How determinism is achieved

`tpt-fl-deterministic-sim` removes each source of divergence:

1. **Fixed-point arithmetic.** [`FixedPoint<DENOM>`](#fixed-point-numbers) stores a
   value as an `i64` scaled by a compile-time denominator. Every operation is integer
   arithmetic, so there is no rounding difference between CPUs.
2. **Ordered storage.** Entities live in a `BTreeMap`, so iteration is always in
   [`EntityId`] order. `HashMap` is not used, because its iteration order depends on
   a random seed.
3. **Ordered systems.** Systems run in alphabetical order of their names, on every
   step.
4. **Deferred structural changes.** Spawns and despawns requested by systems are
   applied after all systems have run, in the order they were requested.
5. **Stable hashing.** `state_hash` writes integers in little-endian order with
   FNV-1a, so the same state gives the same hash on every platform.

## Quick start

```rust
use tpt_fl_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000_000>; // six decimal places of precision

let mut sim = DeterministicSim::<Fp>::new();

// Spawn an entity with an initial state value
let id = sim.spawn(Fp::from_int(10));

// Register a system that doubles every entity's value each step
sim.add_system("double", |entities, _ctx| {
    for value in entities.values_mut() {
        *value = *value + *value;
    }
});

sim.step();
assert_eq!(*sim.get(id).unwrap(), Fp::from_int(20));
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-deterministic-sim = "0.1"
```

## Fixed-point numbers

`FixedPoint<DENOM>` represents `raw / DENOM`. The denominator is a `const` generic,
so the scale is part of the type:

```rust
use tpt_fl_deterministic_sim::FixedPoint;

type Money = FixedPoint<100>; // two decimal places

let price = Money::from_raw(1_050);     // 10.50
let total = price * Money::from_int(2); // 21.00
assert_eq!(total.raw(), 2_100);
```

Arithmetic comes in three forms:

| Form | Overflow behaviour |
|------|--------------------|
| `+`, `-`, `*`, `/`, unary `-` | Panic if the result does not fit in `i64` |
| `checked_add`, `checked_sub`, `checked_mul`, `checked_div`, `checked_neg`, `checked_abs` | Return `None` |
| `saturating_add`, `saturating_sub`, `saturating_mul` | Clamp to the `i64` range |

Multiplication and division use `i128` intermediates, and then round toward zero
when they rescale by `DENOM`.

Construction and inspection:

| Method | Purpose |
|--------|---------|
| `from_raw(r)` / `raw()` | Construct from, or read, the scaled integer |
| `from_int(n)` | The integer `n` |
| `zero()`, `one()` | Identity values |
| `is_zero`, `is_negative`, `is_positive`, `abs` | Sign tests and absolute value |
| `to_f64` | Lossy conversion for display only |
| `from_f64_truncating` | Lossy conversion for input. The result depends on floating-point rounding, so use `from_raw` or `from_int` inside the deterministic core |

## Entities and systems

An entity is a value of the state type `S`, identified by an `EntityId`. A system is
a closure over the whole entity map:

```rust
use tpt_fl_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000>;

let mut sim = DeterministicSim::<Fp>::with_timestep(Fp::from_raw(100)); // dt = 0.1

let ball = sim.spawn(Fp::from_int(0));

sim.add_system("integrate", |entities, ctx| {
    let dt = *ctx.timestep().expect("timestep set");
    for position in entities.values_mut() {
        *position = *position + dt;
    }
});

sim.step_n(3);
assert_eq!(sim.get(ball), Some(&Fp::from_raw(300)));
```

Key points:

- Systems are `FnMut(&mut Entities<S>, &mut SimContext<S>)`, so they can keep their
  own state between steps.
- Systems run in alphabetical name order. Choose names to control the order.
- `add_system` replaces a system with the same name. `remove_system` returns `true`
  if one was removed.
- `SimContext` gives each system the current `step()` index and the `timestep()`
  set with `with_timestep`.
- `step_n(n)` runs `n` steps. `iter()` visits entities in ID order.

## Structural changes

Systems can create and remove entities through the context. Both take effect after
every system has run in the current step:

```rust
use tpt_fl_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000>;

let mut sim = DeterministicSim::<Fp>::new();
sim.spawn(Fp::from_int(3)); // lives for three steps

sim.add_system("age", |entities, ctx| {
    let expired: Vec<_> = entities
        .iter()
        .filter(|(_, v)| **v <= Fp::from_int(1))
        .map(|(id, _)| *id)
        .collect();
    for id in expired {
        ctx.despawn(id);
    }
    for v in entities.values_mut() {
        *v = *v - Fp::from_int(1);
    }
});

sim.step_n(3);
assert_eq!(sim.entity_count(), 0);
```

`SimContext::spawn` returns the new `EntityId` straight away, so later systems in
the same step can refer to it. The entity itself is added to the map only after
the step.

## Snapshots and state hashing

```rust
use tpt_fl_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000>;

let mut sim = DeterministicSim::<Fp>::new();
let id = sim.spawn(Fp::from_int(1));
let saved = sim.snapshot();
let hash_before = sim.state_hash();

sim.add_system("double", |entities, _ctx| {
    for value in entities.values_mut() {
        *value = *value + *value;
    }
});
sim.step();
assert_eq!(sim.get(id), Some(&Fp::from_int(2)));

sim.restore(&saved); // back to the state before the step
assert_eq!(sim.get(id), Some(&Fp::from_int(1)));
assert_eq!(sim.state_hash(), hash_before);
```

- `snapshot()` copies the entities and counters. Systems are not part of a
  snapshot, so they stay registered across `restore`.
- `state_hash()` hashes the entities and counters with FNV-1a over little-endian
  encodings. Two simulations with equal state have equal hashes.

## Feature flags

| Feature | Default | Effect |
|---------|---------|--------|
| `exact-math` | off | Adds `FixedPoint::to_rational` and `FixedPoint::from_rational`, which convert to and from [`tpt-fl-exact-math`](https://github.com/tpt-solutions/tpt-formal-lab/tree/master/tpt-fl-exact-math) `Rational`. The simulation state stays fixed-point |

```toml
[dependencies]
tpt-fl-deterministic-sim = { version = "0.1", features = ["exact-math"] }
```

## Guarantees

- **Integer-only core.** `FixedPoint` arithmetic never uses floating point.
- **Ordered iteration.** Entities are visited in `EntityId` order, and systems run
  in name order.
- **Deferred structural changes.** Spawns and despawns do not change the map while a
  system is iterating it.
- **Stable hashing.** `state_hash` uses explicit little-endian encodings.
- **No `unsafe` code.**

## Limitations

- Determinism depends on `S`. A state type that uses floating point, or a
  `HashMap`, is not deterministic. Use `FixedPoint` for numeric state.
- `FixedPoint` has a finite range. Results that do not fit panic, or return `None`
  from the `checked_` methods, so choose the denominator to suit the range.
- Systems run sequentially in one thread. There is no parallel scheduler.

## Crate features and MSRV

- Optional feature: `exact-math`.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-deterministic-sim
cargo test -p tpt-fl-deterministic-sim --features exact-math
```

The unit tests cover fixed-point arithmetic (`src/fixed_point.rs`) and simulation
behaviour, including systems, structural changes, snapshots and hashing
(`src/sim.rs`).

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
