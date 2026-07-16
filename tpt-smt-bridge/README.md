# tpt-smt-bridge

[![crates.io](https://img.shields.io/crates/v/tpt-smt-bridge.svg)](https://crates.io/crates/tpt-smt-bridge)
[![docs.rs](https://docs.rs/tpt-smt-bridge/badge.svg)](https://docs.rs/tpt-smt-bridge)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

Ergonomic, zero-dependency Rust interface to SMT solvers.

**Build constraints in Rust → emit valid SMT-LIB2 → pipe to Z3, CVC5, or any solver.**

## The problem

Interfacing with Z3 from Rust usually means either:
- Heavy FFI bindings with complex C-API lifetime management, or
- Manually writing raw SMT-LIB2 strings — error-prone and hard to debug.

## Solution

`tpt-smt-bridge` is pure Rust with no C dependencies. Build your constraints
using a fluent Rust API, then emit a standards-compliant SMT-LIB2 string.

```rust
use tpt_smt_bridge::{SmtSolver, Sort, Expr};

let mut solver = SmtSolver::new();
solver.set_logic("QF_LIA");
solver.declare_const("x", Sort::Int);
solver.declare_const("y", Sort::Int);

let x = Expr::var("x", Sort::Int);
let y = Expr::var("y", Sort::Int);

solver.assert(Expr::gt(x.clone(), Expr::int(0)));
solver.assert(Expr::gt(y.clone(), Expr::int(0)));
solver.assert(Expr::eq(Expr::add(x, y), Expr::int(10)));

println!("{}", solver.emit_check());
// Outputs:
// (set-logic QF_LIA)
// (declare-const x Int)
// (declare-const y Int)
// (assert (> x 0))
// (assert (> y 0))
// (assert (= (+ x y) 10))
// (check-sat)
// (get-model)
```

## Parsing counterexamples

```rust
use tpt_smt_bridge::CounterExample;

let model = r#"(model
  (define-fun x () Int 3)
  (define-fun y () Int 7)
)"#;

let ce = CounterExample::parse(model);
assert_eq!(ce.get_int("x"), Some(3));
assert_eq!(ce.get_int("y"), Some(7));
```

## Supported sorts

`Bool`, `Int`, `Real`, `BitVec(width)`, `Array(domain, range)`

## Supported operations

Propositional: `not`, `and`, `or`, `implies`, `iff`  
Comparison: `eq`, `distinct`, `lt`, `le`, `gt`, `ge`  
Arithmetic: `add`, `sub`, `mul`, `div`, `mod`, `neg`, `abs`  
Quantifiers: `forall`, `exists`  
Conditional: `ite` (if-then-else)

## Usage

```toml
[dependencies]
tpt-smt-bridge = "0.1"
```

No system libraries required. Bring your own solver binary.

## License

MIT OR Apache-2.0
