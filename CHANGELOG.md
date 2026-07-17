# Changelog

All notable changes to the `tpt-formal-lab` workspace are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] — Unreleased

### Added

#### `tpt-exact-math`
- `Rational`: arbitrary-precision rational arithmetic backed by `num-bigint`
- `Interval`: closed-interval arithmetic with monotone propagation
- `#![no_std]` support (requires `alloc`)

#### `tpt-proof-ast`
- Kind system: `PropKind`, `TermKind`, `TypeKind` markers
- `Expr<K>`: typed AST nodes parameterised over kind
- `Formula` and `Term` type aliases
- `AstBuilder`: fluent API for constructing well-typed AST nodes

#### `tpt-verify-macros`
- `#[requires(expr)]`: precondition macro
- `#[ensures(expr)]`: postcondition macro
- `#[invariant(expr)]`: invariant macro
- `#[pure]`: side-effect-free marker

#### `tpt-deterministic-sim`
- `FixedPoint<DENOM>`: const-generic bitwise-deterministic fixed-point type
- `DeterministicSim<S>`: step-based simulation engine using `BTreeMap` for sorted determinism

#### `tpt-smt-bridge`
- `Sort`: SMT-LIB2 sort enum (`Bool`, `Int`, `Real`, `BitVec`, `Array`)
- `Expr`: SMT expression AST with full propositional, arithmetic, and quantifier support
- `SmtSolver`: fluent builder emitting SMT-LIB2 strings
- `CounterExample`: parser for `(model ...)` solver output
- `emit_smtlib2`: standalone expression serialiser

#### `tpt-redundancy`
- `Replicated<T, const N>`: N-modular redundant channel container
- `majority_vote`: `Unanimous` / `Majority` / `NoMajority` classification
- `median_vote` / `order_statistic`: outlier-robust voting for `Ord` types
- `bitwise_vote`: bit-by-bit majority for unsigned integers
- `#![no_std]` + `#![deny(unsafe_code)]`, zero dependencies

#### `tpt-verified-ode`
- `IntervalFn` trait: right-hand side enclosure of `y' = f(t, y)`
- `OdeSolver<F>`: interval-arithmetic Picard-iteration verified integrator
- `step` / `solve`: carry-forward enclosure guaranteed to contain the true solution
- Depends on `tpt-exact-math`; `#![no_std]` + `alloc`

#### `tpt-trace-macros`
- `#[traces("REQ-…")]`: requirement-tracing attribute macro
- Injects `**Traces:** …` doc comments and a `__TPT_TRACES_<FN>` const slice

#### `tpt-verified-algorithms`
- `is_sorted` / `is_permutation`: predicate building blocks
- `verified_sort`: insertion sort with debug-mode sortedness + permutation checks
- `verified_binary_search`: binary search guarded by `#[requires]` / `#[ensures]`
- Depends on `tpt-verify-macros`; `#![no_std]` + `alloc`

#### `tpt-det-proptest`
- `Xorshift64`: deterministic PRNG
- `Strategy` trait with `IntRange<T>` and `AnyBool` implementations
- `Seed`: reproducible seed for runs and shrinking
- `check`: property runner with bisection shrinking to a minimal counterexample
- `#![no_std]` + `alloc`, zero external dependencies
