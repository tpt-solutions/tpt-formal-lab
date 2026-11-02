# Changelog

All notable changes to `tpt-fl-verified-ode` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Not yet published: `publish = false` in `Cargo.toml`. See `todo.md`, Phase 16, for the
publish plan.

---

## [Unreleased]

### Changed (breaking, pre-release)

- `IntervalFn::eval` takes and returns an `Interval`, not a point. Corner-only
  evaluation is unsound for functions that are not monotone, so the right-hand side
  must be an interval extension.
- `OdeSolver::step`, `solve`, `solve_until` and `a_priori` return
  `Result<_, OdeError>`. A step that cannot be verified is halved, and no unverified
  box is returned.
- `step` rejects `h <= 0` with `OdeError::NonPositiveStep`.
- The step is tightened to the end-point form `y0 + h · f(t, Y)`, so the state is an
  end-point enclosure rather than the hull of the whole box.

### Added

- `OdeError`, with `NonPositiveStep` and `NoEnclosure`.
- Outward-rounded endpoints, rounded to multiples of 2⁻⁶⁴, so that denominators do
  not grow without bound over long integrations.
- A Picard search that tightens the enclosure, and widens the starting box up to a
  fixed limit.
- `solve_until`: integrates to an exact end time, shortening the last step and
  halving steps where needed.
- `VectorOdeSolver` and `VectorIntervalFn`: verified integration of vector systems.
  There is one enclosure per state component, and a step is verified for all
  components at once.
- `TaylorOdeSolver`, `TaylorFn` and `TaylorArith`: Taylor-model integration of vector
  systems. The right-hand side is written once and evaluated on intervals and on
  Taylor models. This gives tighter enclosures than the Picard solver for smooth
  problems.
- Tests for non-monotone right-hand sides, stiff problems, non-autonomous systems,
  failing fixpoints, enclosure widths, blow-up, and `solve_until` end times.

### Fixed

- `step` could return a box that had not been verified.
- The Picard iteration could grow without bound near a blow-up. It now returns
  `NoEnclosure` instead.

---

## Initial implementation

- `OdeSolver`: a verified scalar solver for `y' = f(t, y)` using Picard–Lindelöf
  iteration on interval boxes.
- `IntervalFn`: the right-hand side as an interval extension.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-verified-ode
