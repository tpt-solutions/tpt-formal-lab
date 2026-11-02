# tpt-formal-lab

> Proof-native, verification-first building blocks for safety-critical Rust.

[![CI](https://github.com/tpt-solutions/tpt-formal-lab/actions/workflows/ci.yml/badge.svg)](https://github.com/tpt-solutions/tpt-formal-lab/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

---

`tpt-formal-lab` is a workspace of ten focused crates for developers building
systems where floating-point errors, non-determinism, or logical flaws are
**unacceptable** — flight control, financial engines, cryptographic protocols,
and formally verified compilers.

## Crates

| Crate | Description | crates.io |
|-------|-------------|-----------|
| [`tpt-fl-exact-math`](tpt-fl-exact-math/) | Arbitrary-precision rational numbers and interval arithmetic | [![crates.io](https://img.shields.io/crates/v/tpt-fl-exact-math.svg)](https://crates.io/crates/tpt-fl-exact-math) |
| [`tpt-fl-proof-ast`](tpt-fl-proof-ast/) | Strongly-typed proof-native AST — invalid states are compile errors | [![crates.io](https://img.shields.io/crates/v/tpt-fl-proof-ast.svg)](https://crates.io/crates/tpt-fl-proof-ast) |
| [`tpt-fl-verify-macros`](tpt-fl-verify-macros/) | Procedural macros for `#[requires]`, `#[ensures]`, `#[invariant]` | [![crates.io](https://img.shields.io/crates/v/tpt-fl-verify-macros.svg)](https://crates.io/crates/tpt-fl-verify-macros) |
| [`tpt-fl-deterministic-sim`](tpt-fl-deterministic-sim/) | Bitwise-deterministic simulation engine | [![crates.io](https://img.shields.io/crates/v/tpt-fl-deterministic-sim.svg)](https://crates.io/crates/tpt-fl-deterministic-sim) |
| [`tpt-fl-smt-bridge`](tpt-fl-smt-bridge/) | Ergonomic Rust → SMT-LIB2 bridge for Z3, CVC5, etc. | [![crates.io](https://img.shields.io/crates/v/tpt-fl-smt-bridge.svg)](https://crates.io/crates/tpt-fl-smt-bridge) |
| [`tpt-fl-redundancy`](tpt-fl-redundancy/) | N-modular redundancy and voting for fault-tolerant systems | [![crates.io](https://img.shields.io/crates/v/tpt-fl-redundancy.svg)](https://crates.io/crates/tpt-fl-redundancy) |
| [`tpt-fl-verified-ode`](tpt-fl-verified-ode/) | Rigorously verified ODE integration via interval arithmetic | [![crates.io](https://img.shields.io/crates/v/tpt-fl-verified-ode.svg)](https://crates.io/crates/tpt-fl-verified-ode) |
| [`tpt-fl-trace-macros`](tpt-fl-trace-macros/) | `#[traces("REQ-…")]` macro linking code to requirements | [![crates.io](https://img.shields.io/crates/v/tpt-fl-trace-macros.svg)](https://crates.io/crates/tpt-fl-trace-macros) |
| [`tpt-fl-verified-algorithms`](tpt-fl-verified-algorithms/) | Verified sorting and searching with debug-mode contract checks | [![crates.io](https://img.shields.io/crates/v/tpt-fl-verified-algorithms.svg)](https://crates.io/crates/tpt-fl-verified-algorithms) |
| [`tpt-fl-det-proptest`](tpt-fl-det-proptest/) | Deterministic property-based testing with bisection shrinking | [![crates.io](https://img.shields.io/crates/v/tpt-fl-det-proptest.svg)](https://crates.io/crates/tpt-fl-det-proptest) |

Five crates (`tpt-fl-exact-math`, `tpt-fl-proof-ast`, `tpt-fl-verify-macros`,
`tpt-fl-deterministic-sim`, `tpt-fl-smt-bridge`) are in the planned v0.1.0 crates.io
release. The other five are `publish = false` until a publish plan exists, so
their crates.io badges show as missing until they are published.

---

## Quick look

### Exact arithmetic — no rounding ever

```rust
use tpt_fl_exact_math::Rational;

// 0.1 + 0.2 is exactly 3/10 — not 0.30000000000000004
let a = Rational::from_frac(1, 10);
let b = Rational::from_frac(2, 10);
assert_eq!(a + b, Rational::from_frac(3, 10));
```

### Type-safe AST — invalid trees don't compile

```rust
use tpt_fl_proof_ast::{AstBuilder, Formula, Term};

let b = AstBuilder::new();
let x = b.var_term("x");
let stmt = b.forall("x", b.gt(x, b.int_term(0)));
// You cannot accidentally use a Term where a Formula is expected.
```

### Formal contracts — zero runtime cost in release

```rust
use tpt_fl_verify_macros::{requires, ensures};

#[requires(x > 0)]
#[ensures(result > 0)]
fn double(x: i32) -> i32 { x * 2 }
```

### Bitwise-deterministic simulation

```rust
use tpt_fl_deterministic_sim::{DeterministicSim, FixedPoint};

type Fp = FixedPoint<1_000_000>;
let mut sim = DeterministicSim::<Fp>::new();
// Identical on x86, ARM, RISC-V — guaranteed.
```

### SMT solver interface — pure Rust, no C

```rust
use tpt_fl_smt_bridge::{SmtSolver, Sort, Expr};

let mut solver = SmtSolver::new();
solver.declare_const("x", Sort::Int);
solver.assert(Expr::gt(Expr::var("x", Sort::Int), Expr::int(0)));
println!("{}", solver.emit_check()); // pipe to z3 -in
```

### Verified voting — tolerate faulty channels

```rust
use tpt_fl_redundancy::Replicated;

let channels = Replicated::new([10u32, 10u32, 12u32]);
assert_eq!(channels.majority_vote().value(), Some(&10u32));
```

### Verified ODE integration — enclosures, not floats

```rust
use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::OdeSolver;

// y' = y, y(0) = 1
let mut solver = OdeSolver::new(
    |_t: &Interval, y: &Interval| y.clone(),
    Rational::from(0),
    Rational::from(1),
);
let (lo, hi) = solver.step(&Rational::from_frac(1, 10)).unwrap(); // encloses e^0.1
assert!(lo <= hi);
```

### Requirement tracing — link code to specs

```rust
use tpt_fl_trace_macros::traces;

#[traces("REQ-BRAKE-001")]
pub fn braking_distance(speed: f64) -> f64 { 0.5 * speed * speed }
```

### Verified algorithms — checked postconditions

```rust
use tpt_fl_verified_algorithms::verified_sort;

let mut data = [3, 1, 2];
verified_sort(&mut data);
assert_eq!(data, [1, 2, 3]);
```

### Deterministic property testing — reproducible by seed

```rust
use tpt_fl_det_proptest::{check, IntRange, Seed};

let r = check(100, Seed(1), IntRange::<i32>::new(0, 100), |x| *x < 100);
assert!(r.is_ok());
```

---

## Getting started

Add the crates you need to your `Cargo.toml`. The five crates on crates.io:

```toml
[dependencies]
tpt-fl-exact-math        = "0.1"
tpt-fl-proof-ast         = "0.1"
tpt-fl-verify-macros     = "0.1"
tpt-fl-deterministic-sim = "0.1"
tpt-fl-smt-bridge        = "0.1"
```

The other five (`tpt-fl-redundancy`, `tpt-fl-verified-ode`, `tpt-fl-trace-macros`,
`tpt-fl-verified-algorithms`, `tpt-fl-det-proptest`) are not yet published. Depend on
them by path or git until they are:

```toml
[dependencies]
tpt-fl-redundancy = { git = "https://github.com/tpt-solutions/tpt-formal-lab" }
```

## MSRV

Rust **1.75** or later.

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
