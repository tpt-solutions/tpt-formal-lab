# Changelog

All notable changes to `tpt-fl-smt-bridge` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

Target release: **0.1.0** (part of the first crates.io batch).

---

## [Unreleased]

### Changed (breaking, pre-release)

- `SmtSolver::emit_check` emits only `(check-sat)`. Use `emit_check_model` for
  `(get-model)`.
- `Expr::div` is real division. `Expr::idiv` is integer division.
- `CeValue::Int` holds an `i128`.

### Added

- `SmtSolver::check_sat` runs Z3, CVC5 or any SMT-LIB2 command, with a timeout. It
  returns `SatResult`, and `SolverError` for failures. `Backend` selects the solver.
- `SatResult`, `SolverError` and `Backend`, with `parse_unsat_core` for
  `(get-unsat-core)` output.
- Arrays (`select`, `store`), bit-vector operators, string operators with escaping,
  `let` sharing and function application (`declare_fun`, `apply`).
- `push` and `pop`, named assertions (`assert_named`), unsat cores
  (`emit_check_core`) and `set_option`.
- `sort_of` for sort checking of expressions against the declared symbols.
- `check_values` and `parse_get_value` for `(get-value ...)`.
- A real s-expression reader for solver output, with `i128` values and `(- N)`
  negatives.
- Integration tests in `tests/z3.rs`, which skip when `z3` is not on `PATH`.

### Fixed

- `CounterExample::parse` did not read multi-line solver output, which is the form
  Z3 prints.

---

## Initial implementation

- `Sort`: SMT-LIB2 sorts (`Bool`, `Int`, `Real`, `BitVec`, `String`, `Array`).
- `Expr`: SMT expressions with propositional, arithmetic and quantifier support.
- `SmtSolver`: fluent builder that emits SMT-LIB2.
- `CounterExample`: parser for solver model output.
- `emit_smtlib2`: standalone expression serialiser.

---

[Unreleased]: https://github.com/tpt-solutions/tpt-formal-lab/commits/master/tpt-fl-smt-bridge
