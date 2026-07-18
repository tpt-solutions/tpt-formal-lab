# tpt-formal-lab

> Proof-native, verification-first building blocks for safety-critical Rust.

[![CI](https://github.com/tpt-solutions/tpt-formal-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-formal-lab/actions)
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
| [`out-verify-macros`](out-verify-macros/) | Procedural macros for `#[requires]`, `#[ensures]`, `#[invariant]` | [![crates.io](https://img.shields.io/crates/v/out-verify-macros.svg)](https://crates.io/crates/out-verify-macros) |
| [`tpt-deterministic-sim`](tpt-deterministic-sim/) | Bitwise-deterministic simulation engine | [![crates.io](https://img.shields.io/crates/v/tpt-deterministic-sim.svg)](https://crates.io/crates/tpt-deterministic-sim) |
| [`out-smt-bridge`](out-smt-bridge/) | Ergonomic Rust → SMT-LIB2 bridge for Z3, CVC5, etc. | [![crates.io](https://img.shields.io/crates/v/out-smt-bridge.svg)](https://crates.io/crates/out-smt-bridge) |
| [`tpt-redundancy`](tpt-redundancy/) | N-modular redundancy and voting for fault-tolerant systems | [![crates.io](https://img.shields.io/crates/v/tpt-redundancy.svg)](https://crates.io/crates/tpt-redundancy) |
| [`tpt-verified-ode`](tpt-verified-ode/) | Rigorously verified ODE integration via interval arithmetic | [![crates.io](https://img.shields.io/crates/v/tpt-verified-ode.svg)](https://crates.io/crates/tpt-verified-ode) |
| [`tpt-trace-macros`](tpt-trace-macros/) | `#[traces("REQ-…")]` macro linking code to requirements | [![crates.io](https://img.shields.io/crates/v/tpt-trace-macros.svg)](https://crates.io/crates/tpt-trace-macros) |
| [`tpt-verified-algorithms`](tpt-verified-algorithms/) | Verified sorting and searching with debug-mode contract checks | [![crates.io](https://img.shields.io/crates/v/tpt-verified-algorithms.svg)](https://crates.io/crates/tpt-verified-algorithms) |
| [`tpt-det-proptest`](tpt-det-proptest/) | Deterministic property-based testing with bisection shrinking | [![crates.io](https://img.shields.io/crates/v/tpt-det-proptest.svg)](https://crates.io/crates/tpt-det-proptest) |

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
use out_verify_macros::{requires, ensures};

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
use out_smt_bridge::{SmtSolver, Sort, Expr};

let mut solver = SmtSolver::new();
solver.declare_const("x", Sort::Int);
solver.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
println!("{}", solver.emit_check()); // pipe to z3 -in
```

### Verified voting — tolerate faulty channels

```rust
use tpt_redundancy::Replicated;

let channels = Replicated::new([10u32, 10u32, 12u32]);
assert_eq!(channels.majority_vote().value(), Some(&10u32));
```

### Verified ODE integration — enclosures, not floats

```rust
use tpt_exact_math::Rational;
use tpt_verified_ode::OdeSolver;

let mut solver = OdeSolver::new(|_t, y| y.clone(), Rational::from(0), Rational::from(1));
let (lo, hi) = solver.step(&Rational::from_frac(1, 4)); // contains e^0.25
assert!(lo <= hi);
```

### Requirement tracing — link code to specs

```rust
use tpt_trace_macros::traces;

#[traces("REQ-BRAKE-001")]
pub fn braking_distance(speed: f64) -> f64 { 0.5 * speed * speed }
```

### Verified algorithms — checked postconditions

```rust
use tpt_verified_algorithms::verified_sort;

let mut data = [3, 1, 2];
verified_sort(&mut data);
assert_eq!(data, [1, 2, 3]);
```

### Deterministic property testing — reproducible by seed

```rust
use tpt_det_proptest::{check, IntRange, Seed};

let r = check(100, Seed(1), IntRange::<i32>::new(0, 100), |x| *x < 100);
assert!(r.is_ok());
```

---

## Getting started

Add the crates you need to your `Cargo.toml`:

```toml
[dependencies]
tpt-exact-math      = "0.1"
tpt-proof-ast       = "0.1"
out-verify-macros   = "0.1"
tpt-deterministic-sim = "0.1"
out-smt-bridge      = "0.1"
tpt-redundancy      = "0.1"
tpt-verified-ode    = "0.1"
tpt-trace-macros    = "0.1"
tpt-verified-algorithms = "0.1"
tpt-det-proptest    = "0.1"
```

## MSRV

Rust **1.75** or later.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
