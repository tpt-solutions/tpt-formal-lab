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
