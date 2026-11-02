# Changelog

All notable changes to `tpt-fl-deterministic-sim` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Target release: **0.1.0** (part of the first crates.io batch).

---

## [Unreleased]

### Changed (breaking, pre-release)

- `FixedPoint` multiplication and division use `i128` intermediates and panic if the
  result overflows `i64`. Use the new `checked_*` and `saturating_*` methods to
  handle overflow without a panic.
- The `exact-math` feature now provides `FixedPoint::to_rational` and
  `from_rational`. It no longer changes the simulation type.
- `DeterministicSim::add_system` takes an `FnMut(&mut Entities<S>, &mut SimContext<S>)`.
  Systems keep their own state between steps.

### Added

- Checked arithmetic on `FixedPoint`: `checked_add`, `checked_sub`, `checked_mul`,
  `checked_div`, `checked_neg` and `checked_abs`.
- Saturating arithmetic on `FixedPoint`: `saturating_add`, `saturating_sub` and
  `saturating_mul`.
- `FixedPoint::to_rational` and `FixedPoint::from_rational` behind the `exact-math`
  feature.
- `SimContext`: the step index, the timestep set with `with_timestep`, and queues of
  spawns and despawns applied after the systems run.
- `DeterministicSim::snapshot` and `restore`.
- `DeterministicSim::state_hash`: FNV-1a over little-endian encodings.
- Systems are `FnMut` and take `(entities, ctx)`.

---

## Initial implementation

- `FixedPoint<DENOM>`: a const-generic, bitwise-deterministic fixed-point type.
- `DeterministicSim<S>`: a step-based simulation engine that stores entities in a
  `BTreeMap` for deterministic iteration order.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-deterministic-sim
