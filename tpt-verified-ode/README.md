# tpt-verified-ode

[![crates.io](https://img.shields.io/crates/v/tpt-verified-ode.svg)](https://crates.io/crates/tpt-verified-ode)
[![docs.rs](https://docs.rs/tpt-verified-ode/badge.svg)](https://docs.rs/tpt-verified-ode)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](../LICENSE-MIT)

Rigorously verified ODE integration via interval arithmetic and Picard iteration.

**Every step returns an enclosure guaranteed to contain the true solution — no
floating-point error, no truncation error leaking through.**

## What this crate does

Given an initial-value problem `y' = f(t, y)`, `y(t0) = y0`, the solver computes
a sequence of intervals `[y_lo, y_hi]` such that the exact solution `y(t)` is
contained in the enclosure at every step. This is achieved with the
[Picard–Lindelöf iteration](https://en.wikipedia.org/wiki/Picard%E2%80%93Lindel%C3%B6f_theorem):

```text
y_{k+1}(t) = y0 + ∫_{t0}^{t} f(s, y_k(s)) ds
```

We run the iteration with the exact `Rational` interval arithmetic of
[`tpt-exact-math`], starting from the a priori enclosure and tightening until a
fixpoint is reached.

## ⚠ Rigor caveat

The Picard iteration provably converges to the unique solution **only when the
step size `h` is small enough that the iteration is a contraction** on the
enclosure. With a too-large `h`, the a priori box may fail to contract and the
enclosure can become wide or fail to tighten.

This crate does **not** automatically verify the contraction condition. It is the
caller's responsibility to choose a step size `h` that is sufficiently small for
the problem at hand. As a rule of thumb, `h * L < 1` where `L` is a Lipschitz
constant of `f` over the enclosure. When in doubt, use a smaller `h` and confirm
the enclosure widths shrink monotonically (see [`OdeSolver::step`]).

## Example

```rust
use tpt_exact_math::Rational;
use tpt_verified_ode::{IntervalFn, OdeSolver};

// y' = y,  y(0) = 1  →  exact solution y(t) = e^t
struct Exp;
impl IntervalFn for Exp {
    fn eval(&self, _t: &Rational, y: &Rational) -> Rational {
        y.clone()
    }
}

let mut solver = OdeSolver::new(Exp, Rational::from(0), Rational::from(1));
let h = Rational::from_frac(1, 4); // 0.25 — comfortably small for the contraction
let (lo, hi) = solver.step(&h);
// The enclosure must contain e^0.25 ≈ 1.2840...
let e_h = Rational::from_frac(1284, 1000);
assert!(lo <= e_h && e_h <= hi);
```

## Features

- `#![no_std]` + `alloc`.
- No floating point anywhere — `Rational` exact arithmetic throughout.

## Usage

```toml
[dependencies]
tpt-verified-ode = "0.1"
```

## License

MIT OR Apache-2.0
