# tpt-fl-smt-bridge

[![crates.io](https://img.shields.io/crates/v/tpt-fl-smt-bridge.svg)](https://crates.io/crates/tpt-fl-smt-bridge)
[![docs.rs](https://docs.rs/tpt-fl-smt-bridge/badge.svg)](https://docs.rs/tpt-fl-smt-bridge)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Ergonomic, zero-dependency Rust interface to SMT solvers.

**Build constraints in Rust, emit valid SMT-LIB2, and run Z3, CVC5 or any
SMT-LIB2 solver without any C bindings.**

## Contents

- [The problem](#the-problem)
- [Features](#features)
- [Quick start](#quick-start)
- [Building expressions](#building-expressions)
- [Declaring symbols and asserting](#declaring-symbols-and-asserting)
- [Running a solver](#running-a-solver)
- [Reading models and counterexamples](#reading-models-and-counterexamples)
- [Unsat cores and named assertions](#unsat-cores-and-named-assertions)
- [Incremental solving](#incremental-solving)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

There are two common ways to use an SMT solver from Rust:

- **Heavy FFI bindings** to a C API, with lifetime management that is hard to get
  right and a build that needs the solver's libraries.
- **Hand-written SMT-LIB2 strings**, which are error-prone and hard to debug.

`tpt-fl-smt-bridge` takes a third route. Constraints are built as typed Rust values,
emitted as standard SMT-LIB2, and passed to a solver binary you already have. The
crate itself is pure Rust with no dependencies, and it has no `unsafe` code.

## Features

- **Typed expressions.** `Expr` covers booleans, integers, reals, bit-vectors,
  strings, arrays, quantifiers, `let` and function application.
- **Fluent solver builder.** `SmtSolver` declares symbols, asserts constraints and
  emits SMT-LIB2 with `(check-sat)`, `(get-model)`, `(get-unsat-core)` or
  `(get-value ...)` as needed.
- **Sort checking.** `SmtSolver::sort_of` checks an expression against the declared
  symbols before anything reaches a solver.
- **Solver runner.** `check_sat` runs Z3, CVC5 or any command with a timeout and
  parses the verdict.
- **Model parsing.** `CounterExample` reads `(model ...)` output, including the
  multi-line form Z3 prints, and returns typed values.
- **Unsat cores and named assertions**, `push` and `pop`, and `set_option`.

## Quick start

```rust
use tpt_fl_smt_bridge::{Expr, Sort, SmtSolver};

let mut solver = SmtSolver::new();

// Declare two integer constants
solver.declare_const("x", Sort::Int);
solver.declare_const("y", Sort::Int);

// Assert x > 0, y > 0 and x + y = 10
let x = Expr::var("x", Sort::Int);
let y = Expr::var("y", Sort::Int);
solver.assert(Expr::gt(x.clone(), Expr::int(0)));
solver.assert(Expr::gt(y.clone(), Expr::int(0)));
solver.assert(Expr::eq(Expr::add(x, y), Expr::int(10)));

// The problem as SMT-LIB2, ready for a solver
let problem = solver.emit_check();
println!("{problem}");
// Pipe it to a solver: z3 -in < problem.smt2
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-smt-bridge = "0.1"
```

## Building expressions

`Expr` constructors map one-to-one onto SMT-LIB2 operators:

```rust
use tpt_fl_smt_bridge::{Expr, Sort};

let p = Expr::var("p", Sort::Bool);
let q = Expr::var("q", Sort::Bool);
let formula = Expr::implies(Expr::and(vec![p.clone(), q.clone()]), Expr::not(p));

let n = Expr::var("n", Sort::Int);
let bounded = Expr::ite(
    Expr::lt(n.clone(), Expr::int(0)),
    Expr::neg(n.clone()),   // |n| when n < 0
    n,
);

let r = Expr::var("r", Sort::Real);
let half = Expr::div(r, Expr::real(2, 1)); // real division: r / 2.0
```

| Group | Constructors |
|-------|--------------|
| Literals | `bool`, `int`, `int128`, `real`, `bitvec`, `string` |
| Symbols | `var(name, sort)`, `apply(name, args)` |
| Boolean | `not`, `and`, `or`, `implies`, `iff`, `eq`, `distinct`, `ite` |
| Comparison | `lt`, `le`, `gt`, `ge` |
| Arithmetic | `add`, `add_many`, `sub`, `mul`, `neg`, `abs`, `rdiv`, `div`, `idiv`, `modulo` |
| Quantifiers | `forall(vars, body)`, `exists(vars, body)` |
| Arrays | `select`, `store` |
| Bit-vectors | `bvadd`, `bvsub`, `bvmul`, `bvand`, `bvor`, `bvxor`, `bvnot`, `bvult`, `bvslt` |
| Strings | `str_concat`, `str_len` |
| Binding | `let_in(bindings, body)` |

Division is explicit. `div` (and its alias `rdiv`) is real division, emitted as
`(/ a b)`. `idiv` is integer division, emitted as `(div a b)`.

## Declaring symbols and asserting

```rust
use tpt_fl_smt_bridge::{Sort, SmtSolver};

let mut solver = SmtSolver::new();
solver
    .set_logic("QF_LIA")
    .set_option("produce-models", "true")
    .declare_const("x", Sort::Int)
    .declare_fun("f", &[Sort::Int], Sort::Int); // f : Int -> Int
```

- `set_logic` emits `(set-logic ...)`.
- `set_option` emits `(set-option ...)`.
- `declare_const` and `declare_fun` emit declarations. Use `Sort::Array(...)` for
  arrays and `Sort::BitVec(width)` for bit-vectors.
- `assert` adds a constraint. `assert_named` adds one that can appear in an unsat
  core.
- `push` and `pop` open and close scopes for incremental solving.

The builder methods return `&mut Self`, so calls can be chained.

## Running a solver

`check_sat` writes the problem to a solver process and returns a verdict:

```rust
use std::time::Duration;
use tpt_fl_smt_bridge::{Backend, Expr, SatResult, Sort, SmtSolver};

let mut solver = SmtSolver::new();
solver.declare_const("x", Sort::Int);
solver.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));

match solver.check_sat(&Backend::Z3, Duration::from_secs(10)) {
    Ok(SatResult::Sat(model)) => println!("x = {:?}", model.get_int("x")),
    Ok(SatResult::Unsat) => println!("no solution"),
    Ok(SatResult::Unknown(reason)) => println!("unknown: {reason}"),
    Err(e) => eprintln!("solver failed: {e}"),
}
```

Backends:

| Backend | Command |
|---------|---------|
| `Backend::Z3` | `z3 -in` |
| `Backend::Cvc5` | `cvc5 --lang smt2` |
| `Backend::Command { program, args }` | Any SMT-LIB2 solver |

Errors are reported as `SolverError`:

| Variant | Meaning |
|---------|---------|
| `NotFound(program)` | The solver binary is not on `PATH` |
| `Timeout(duration)` | The solver did not answer in time |
| `Solver(message)` | The solver reported an error |
| `Io(message)` | Reading or writing the process failed |

`check_values(terms, backend, timeout)` also returns the values of the given terms
from `(get-value ...)`, for sat problems.

## Reading models and counterexamples

`CounterExample::parse` reads `(model ...)` output. The getters return `None` when a
name is missing or has a different type:

```rust
use tpt_fl_smt_bridge::CounterExample;

let model = r#"(model
  (define-fun x () Int 3)
  (define-fun y () Int (- 7))
  (define-fun ok () Bool true)
)"#;

let ce = CounterExample::parse(model);
assert_eq!(ce.get_int("x"), Some(3));
assert_eq!(ce.get_int("y"), Some(-7));
assert_eq!(ce.get_bool("ok"), Some(true));
```

| Getter | Returns |
|--------|---------|
| `get(name)` | The raw `CeValue` |
| `get_int`, `get_int128` | `i64` or `i128` |
| `get_bool` | `bool` |
| `get_real` | The real value as written by the solver, as `&str` |
| `get_bitvec` | `(value, width)` |
| `get_str` | The string value, with escapes resolved |
| `names`, `len`, `is_empty` | Iteration and size |

Use `parse_get_value` and `parse_unsat_core` to read `(get-value ...)` and
`(get-unsat-core)` output directly.

## Unsat cores and named assertions

When a problem is unsat, a core names the assertions that caused it:

```rust
use tpt_fl_smt_bridge::{Expr, Sort, SmtSolver};

let mut solver = SmtSolver::new();
solver.declare_const("x", Sort::Int);
let x = Expr::var("x", Sort::Int);
solver.assert_named("positive", Expr::gt(x.clone(), Expr::int(0)));
solver.assert_named("negative", Expr::lt(x, Expr::int(0)));

// emit_check_core() emits (check-sat) and (get-unsat-core)
let problem = solver.emit_check_core();
```

Run `problem` through a backend, then read the names with `parse_unsat_core`.

## Incremental solving

```rust
use tpt_fl_smt_bridge::{Expr, Sort, SmtSolver};

let mut solver = SmtSolver::new();
solver.declare_const("x", Sort::Int);
let x = Expr::var("x", Sort::Int);

solver.push();
solver.assert(Expr::gt(x.clone(), Expr::int(5)));
// ... check, then discard the assertion
solver.pop();
solver.assert(Expr::lt(x, Expr::int(10)));
```

## Guarantees

- **Standard output.** The emitted text is SMT-LIB2. `Expr` printing is also
  available through `emit_smtlib2`.
- **Sort checking on request.** `sort_of` reports mismatches before a solver runs.
- **Bounded runs.** `check_sat` takes a timeout, and a stuck solver process is
  stopped when it expires.
- **No `unsafe` code, no dependencies.**

## Limitations

- The crate does not link to a solver. You need a solver binary on `PATH` for
  `check_sat`. The integration tests in `tests/z3.rs` skip when Z3 is missing.
- `emit_check` emits only `(check-sat)`. Use `emit_check_model` for
  `(get-model)`, which must only be read after `sat`.
- `CounterExample` reads the output that common solvers print. It is not a complete
  SMT-LIB2 reader.
- Interpreting a solver's answer is the caller's job. A `sat` verdict is only as
  good as the constraints you asserted.

## Crate features and MSRV

- No optional features and no dependencies.
- Uses `std`: it needs processes and timeouts.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-smt-bridge
```

The unit tests cover expression emission, the s-expression reader, result parsing,
sorts and counterexamples. `tests/z3.rs` runs real solver checks when `z3` is on
`PATH`.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
