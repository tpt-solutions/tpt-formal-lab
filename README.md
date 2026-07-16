# tpt-formal-lab

> Proof-native, verification-first building blocks for safety-critical Rust.

[![CI](https://github.com/tpt-rs/tpt-formal-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-rs/tpt-formal-lab/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

`tpt-formal-lab` is a workspace of five focused crates for developers building
systems where floating-point errors, non-determinism, or logical flaws are
**unacceptable** — flight control, financial engines, cryptographic protocols,
and formally verified compilers.

## Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| [`tpt-exact-math`](tpt-exact-math/) | Arbitrary-precision rational numbers and interval arithmetic | [![crates.io](https://img.shields.io/crates/v/tpt-exact-math.svg)](https://crates.io/crates/tpt-exact-math) |
| [`tpt-proof-ast`](tpt-proof-ast/) | Strongly-typed proof-native AST — invalid states are compile errors | [![crates.io](https://img.shields.io/crates/v/tpt-proof-ast.svg)](https://crates.io/crates/tpt-proof-ast) |
| [`tpt-verify-macros`](tpt-verify-macros/) | Procedural macros for `#[requires]`, `#[ensures]`, `#[invariant]` | [![crates.io](https://img.shields.io/crates/v/tpt-verify-macros.svg)](https://crates.io/crates/tpt-verify-macros) |
| [`tpt-deterministic-sim`](tpt-deterministic-sim/) | Bitwise-deterministic simulation engine | [![crates.io](https://img.shields.io/crates/v/tpt-deterministic-sim.svg)](https://crates.io/crates/tpt-deterministic-sim) |
| [`tpt-smt-bridge`](tpt-smt-bridge/) | Ergonomic Rust → SMT-LIB2 bridge for Z3, CVC5, etc. | [![crates.io](https://img.shields.io/crates/v/tpt-smt-bridge.svg)](https://crates.io/crates/tpt-smt-bridge) |

---

## Quick look

### Exact arithmetic — no rounding ever

```rust
use tpt_exact_math::Rational;

// 0.1 + 0.2 is exactly 3/10 — not 0.30000000000000004
let a = Rational::from_frac(1, 10);
let b = Rational::from_frac(2, 10);
assert_eq!(a + b, Rational::from_frac(3, 10));
```

### Type-safe AST — invalid trees don't compile

```rust
use tpt_proof_ast::{AstBuilder, Formula, Term};

let b = AstBuilder::new();
let x = b.var_term("x");
let stmt = b.forall("x", b.gt(x, b.int_term(0)));
// You cannot accidentally use a Term where a Formula is expected.
```

### Formal contracts — zero runtime cost in release

```rust
use tpt_verify_macros::{requires, ensures};

#[requires(x > 0)]
#[ensures(result > 0)]
fn double(x: i32) -> i32 { x * 2 }
```

### Bitwise-deterministic simulation

```rust
use tpt_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000_000>;
let mut sim = DeterministicSim::<Fp>::new();
// Identical on x86, ARM, RISC-V — guaranteed.
```

### SMT solver interface — pure Rust, no C

```rust
use tpt_smt_bridge::{SmtSolver, Sort, Expr};

let mut solver = SmtSolver::new();
solver.declare_const("x", Sort::Int);
solver.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
println!("{}", solver.emit_check()); // pipe to z3 -in
```

---

## Getting started

Add the crates you need to your `Cargo.toml`:

```toml
[dependencies]
tpt-exact-math      = "0.1"
tpt-proof-ast       = "0.1"
tpt-verify-macros   = "0.1"
tpt-deterministic-sim = "0.1"
tpt-smt-bridge      = "0.1"
```

## MSRV

Rust **1.75** or later.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
