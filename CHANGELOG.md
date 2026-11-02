# Changelog

All notable changes to the `tpt-formal-lab` workspace are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

The v0.1.0 release batch is the five crates marked below as publishable:
`tpt-fl-exact-math`, `tpt-fl-proof-ast`, `tpt-fl-verify-macros`, `tpt-fl-deterministic-sim`
and `tpt-fl-smt-bridge`. The other five crates are `publish = false` until a
publish plan exists (see `todo.md`, Phase 16).

### Changed (breaking, pre-release)

- `tpt-fl-exact-math`: `Interval` division by a zero-straddling divisor still
  panics, but `Interval::checked_div` now returns `None` in that case.
  `Rational::to_f64` no longer returns NaN for large numerators or denominators.
- `tpt-fl-verified-ode`: `IntervalFn::eval` takes and returns `Interval`. `step`,
  `solve` and `a_priori` return `Result<_, OdeError>`. A step that cannot be
  verified is halved, and no unverified box is returned.
- `tpt-fl-deterministic-sim`: `FixedPoint` multiplication and division use `i128`
  intermediates and panic on overflow of the result. `checked_*` and
  `saturating_*` methods are added. The `exact-math` feature now provides
  `to_rational` and `from_rational`, and no longer changes the simulation type.
  `add_system` takes `FnMut(&mut entities, &mut SimContext)`.
- `tpt-fl-redundancy`: `median_vote` and `order_statistic` return `Option<T>`.
  `BitNarrow` is sealed.
- `tpt-fl-det-proptest`: `Strategy` index space is `u128`. `IntRange::inclusive` is
  added. Shrinking uses an index lookup instead of a linear scan. Range
  sampling is unbiased, and seeds are mixed through splitmix64.
- `tpt-fl-proof-ast`: `TypeKind` is removed (it could not be constructed).
  Quantifiers are parenthesised in `Display`. `ExprNode::Forall` and
  `ExprNode::Exists` now hold a `Binder`, not a bare name.
- `tpt-fl-smt-bridge`: `emit_check` emits only `(check-sat)`. Use
  `emit_check_model` for `(get-model)`. `Expr::div` is real division.
  `Expr::idiv` is integer division. `CeValue::Int` holds an `i128`.
- `tpt-fl-verify-macros`: `#[invariant]` also checks on exit. Contract attributes
  accept methods and impl items, and work on `async`, `const` and `unsafe`
  functions.
- `tpt-fl-verified-algorithms`: uses `tpt-fl-verify-macros` instead of the external
  `contracts` crate.

### Added

#### `tpt-fl-exact-math`
- `Rational::floor`, `ceil`, `round`, `to_i64`, `pow`, `from_f64` (exact)
- `FromStr` for `Rational` (`a/b`, decimals) and `ParseRationalError`
- `Interval::checked_div`, `round_outward`, `intersect`, `abs`, `sqr`, `pow`, `bisect`
- Reference operator impls (`&a + &b` and mixed forms); `From` for `i8`, `i16`,
  `u8`, `u16`, `u32`, `i128`, `u128`

#### `tpt-fl-verified-ode`
- `OdeError`, outward-rounded endpoints, and a Picard search that tightens the
  enclosure
- `solve_until`: integrates to an exact end time, halving steps where needed
- `VectorOdeSolver` and `VectorIntervalFn`: verified integration of vector systems,
  with one enclosure per state component and a step verified for all components at once
- `tpt-fl-verified-ode`: `TaylorOdeSolver` and `TaylorFn`/`TaylorArith`: Taylor-model integration of vector
  systems. The right-hand side is written once and evaluated on intervals and on
  Taylor models, which gives tighter enclosures than the Picard solver for smooth problems

#### `tpt-fl-smt-bridge`
- `from_proof_ast` converts `tpt-fl-proof-ast` formulas and terms to SMT expressions. The crate now depends on `tpt-fl-proof-ast`, so it publishes after it.

#### `tpt-fl-deterministic-sim`
- Checked and saturating arithmetic on `FixedPoint`
- `FixedPoint::to_rational` and `from_rational` behind `exact-math`
- `SimContext`: step index, timestep, and spawn and despawn queues applied after
  the systems run
- `snapshot` / `restore`, and `state_hash` (FNV-1a over little-endian encodings)
- Systems are `FnMut` and take `(entities, ctx)`

#### `tpt-fl-redundancy`
- `bitwise_vote` documents how even-`N` ties are resolved
- `approx_vote` (caller-supplied agreement), `plurality_vote`, `faulty_channels`

#### `tpt-fl-det-proptest`
- `IntRange::inclusive`, support for `usize`, `isize`, `i128` and `u128`
- Splitmix64 seeding, and unbiased sampling
- Tuple (2 and 3) strategies, `optional`, `vec_of`, and `StrategyExt::{map, filter}`,
  plus `one_of`

#### `tpt-fl-proof-ast`
- A reverse `compile_fail` test: a `Term` passed where a `Formula` is expected
- `Binder` with an optional `BinderSort`; `forall_typed` and `exists_typed`
- Function and predicate application (`app_term`, `pred`)
- `walk`, `free_vars`, and capture-avoiding `substitute`

#### `tpt-fl-verify-macros`
- `#[ensures(|name| ...)]` names the returned value
- `old(expr)` in postconditions
- `loop_invariant!` statement macro
- Trybuild compile-fail tests for invalid attribute arguments

#### `tpt-fl-trace-macros`
- Errors reported at the offending token: no ids, non-string, empty, duplicate
- Raw identifiers, methods in `impl` blocks, and visibility-matching constants

#### `tpt-fl-verified-algorithms`
- `lower_bound`, `upper_bound`
- `verified_sort_by`, `verified_sort_by_key`, `is_sorted_by`
- `verified_sort` is now a stable O(n log n) merge sort
- Kani proof harness under `cfg(kani)` (written, not yet run)

#### `tpt-fl-smt-bridge`
- `check_sat` runs Z3, CVC5 or any SMT-LIB2 program with a timeout
- `SatResult`, `SolverError`, `Backend`, `parse_unsat_core`
- Arrays, bit-vector and string operators, `let`, function application
- `push` and `pop`, named assertions, unsat cores, `set_option`
- `sort_of` sort checking
- `check_values` and `parse_get_value` for `(get-value ...)`
- A real s-expression reader for solver output, with `i128` and `(- N)` support

#### Phase 20 additions
- `tpt-fl-deterministic-sim`: `StepLog` hash-chained record of a run, `step_logged` / `step_n_logged`, `verify_log` replay check, and `first_divergence` (bisection on the chain)
- `tpt-fl-verified-ode`: `audit_float_trajectory` flags float states outside the verified enclosure, and `float_rk4` is a float integrator to audit
- `tpt-fl-det-proptest`: `StateMachine`, `check_stateful`, and `replay_run` for model-based testing with greedy shrinking and replayable run seeds
- `tpt-fl-redundancy`: TMR-in-simulation integration test (dev-dependencies only, so the crate stays unpublished)
- `tpt-assurance-case` (new, unpublished): requirements, evidence and traceability with a GSN-style Markdown rendering. Evidence is registered explicitly, with no external dependency

### Fixed
- `tpt-fl-exact-math`: `Interval` division by a non-point divisor (bounds were not
  swapped for the reciprocal).
- CI: the workflow triggered on `main`, but the branch is `master`.
- `tpt-fl-verified-ode`: the old `step` could return a box that was not verified, and
  its Picard iteration could grow without bound near a blow-up.
- `tpt-fl-smt-bridge`: `CounterExample::parse` did not read multi-line solver
  output, the form Z3 prints.

### Not yet published
- `tpt-fl-redundancy`, `tpt-fl-verified-ode`, `tpt-fl-trace-macros`,
  `tpt-fl-verified-algorithms`, `tpt-fl-det-proptest` are `publish = false`.

---

## [0.1.0] — Unreleased

The first release covers the five crates of the publish batch. Items in
[Unreleased] above that touch these crates are part of this release.

### Added

#### `tpt-fl-exact-math`
- `Rational`: arbitrary-precision rational arithmetic backed by `num-bigint`
- `Interval`: closed-interval arithmetic with monotone propagation
- `#![no_std]` support (requires `alloc`)

#### `tpt-fl-proof-ast`
- Kind system: `PropKind` and `TermKind` markers
- `Expr<K>`: typed AST nodes parameterised over kind
- `Formula` and `Term` type aliases
- `AstBuilder`: fluent API for constructing well-typed AST nodes

#### `tpt-fl-verify-macros`
- `#[requires(expr)]`: precondition macro
- `#[ensures(expr)]`: postcondition macro
- `#[invariant(expr)]`: invariant macro
- `#[pure]`: side-effect-free marker

#### `tpt-fl-deterministic-sim`
- `FixedPoint<DENOM>`: const-generic bitwise-deterministic fixed-point type
- `DeterministicSim<S>`: step-based simulation engine using `BTreeMap` for sorted determinism

#### `tpt-fl-smt-bridge`
- `Sort`: SMT-LIB2 sort enum (`Bool`, `Int`, `Real`, `BitVec`, `String`, `Array`)
- `Expr`: SMT expression AST with propositional, arithmetic, quantifier, array and bit-vector support
- `SmtSolver`: fluent builder emitting SMT-LIB2 strings, with `check_sat`
- `CounterExample`: parser for solver model output
- `emit_smtlib2`: standalone expression serialiser

### Not part of 0.1.0
- `tpt-fl-redundancy`, `tpt-fl-verified-ode`, `tpt-fl-trace-macros`,
  `tpt-fl-verified-algorithms` and `tpt-fl-det-proptest` are `publish = false` and
  will be released separately, if at all.

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master
[0.1.0]: https://github.com/tpt-solutions/tpt-formal-lab/releases/tag/v0.1.0
