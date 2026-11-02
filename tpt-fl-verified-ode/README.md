# tpt-fl-verified-ode

[![crates.io](https://img.shields.io/crates/v/tpt-fl-verified-ode.svg)](https://crates.io/crates/tpt-fl-verified-ode)
[![docs.rs](https://docs.rs/tpt-fl-verified-ode/badge.svg)](https://docs.rs/tpt-fl-verified-ode)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

Rigorously verified ODE integration via interval arithmetic and Picard iteration.

**Every step returns an enclosure that is guaranteed to contain the true solution.
No floating-point error and no truncation error escape into the result unnoticed.**

## Contents

- [The problem](#the-problem)
- [How a step is verified](#how-a-step-is-verified)
- [Solvers](#solvers)
- [Quick start](#quick-start)
- [Writing the right-hand side](#writing-the-right-hand-side)
- [Integrating over time](#integrating-over-time)
- [Vector systems](#vector-systems)
- [Taylor-model solver](#taylor-model-solver)
- [Errors](#errors)
- [Guarantees](#guarantees)
- [Limitations](#limitations)
- [Crate features and MSRV](#crate-features-and-msrv)
- [Testing](#testing)
- [Changelog](CHANGELOG.md)
- [License](#license)

## The problem

A numerical ODE solver returns a sequence of approximate states. Nothing in that
output says how far each state is from the true solution. Floating-point rounding,
truncation error and step-size choices are all hidden.

`tpt-fl-verified-ode` returns an **enclosure** instead: a closed interval
`[y_lo, y_hi]` that contains the exact solution at the time of the step. The width of
the interval is the honest uncertainty of the answer.

## How a step is verified

For `y' = f(t, y)` with `y(t0) = y0`, the solver uses the Picard–Lindelöf iteration

```text
y_{k+1}(t) = y0 + ∫_{t0}^{t} f(s, y_k(s)) ds
```

on interval boxes. For a step of size `h`:

1. **A priori box.** The solver finds a box `B` that is invariant under the Picard
   operator, so that `y0 + h · f(t, B)` lies inside `B`. If it cannot find one, the
   step is too large.
2. **Tightening.** It applies the Picard operator again, starting from `B`, to get an
   end-point enclosure at `t + h`.
3. **Verification.** A step is accepted only once its enclosure is verified to
   contain the solution. If the whole step cannot be verified, it is halved and each
   half is attempted in turn.

No unverified box is ever returned. The solver is left unchanged if verification
fails at the smallest step.

The argument is only as good as the right-hand side. `f` must be supplied as an
**interval extension**: it must return an enclosure of its range over the boxes it
is given. See [Writing the right-hand side](#writing-the-right-hand-side).

## Solvers

| Type | State | Use it when |
|------|-------|-------------|
| `OdeSolver` | One scalar | The system is a single equation |
| `VectorOdeSolver` | `Vec` of intervals | The system has several components and the right-hand side is an interval extension |
| `TaylorOdeSolver` | `Vec` of intervals | The system is smooth and you want tighter enclosures per step, at more cost |

All three share one core. The vector solver with one component behaves like the
scalar solver.

## Quick start

```rust
use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::OdeSolver;

// y' = y, y(0) = 1, so y(t) = e^t
let mut solver = OdeSolver::new(
    |_t: &Interval, y: &Interval| y.clone(),
    Rational::from(0),
    Rational::from(1),
);

// One step of size 1/10 encloses e^0.1
let (lo, hi) = solver.step(&Rational::from_frac(1, 10)).unwrap();
assert!(lo <= hi);
```

Add the crate to your project:

```toml
[dependencies]
tpt-fl-verified-ode = "0.1"
```

## Writing the right-hand side

`f` implements `IntervalFn`, which takes the time box and the state box and returns
an enclosure of `f` over them. Any `Fn(&Interval, &Interval) -> Interval` closure
also implements it.

```rust
use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::IntervalFn;

// Logistic growth: y' = y (1 - y)
struct Logistic;

impl IntervalFn for Logistic {
    fn eval(&self, _t: &Interval, y: &Interval) -> Interval {
        let one = Interval::point(Rational::from(1));
        y.clone() * (one - y.clone())
    }
}
```

Build the enclosure from `Interval`'s operators and `Interval::point` for constants.
Do **not** evaluate `f` only at the corners of the box. That is unsound for functions
that are not monotone in their arguments. The operator-based form is the natural
interval extension, and it handles that case.

## Integrating over time

`step` advances one step. `solve` takes a fixed number of steps of the same size.
`solve_until` reaches an exact end time, using steps no larger than `h_max`:

```rust
use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::OdeSolver;

let mut solver = OdeSolver::new(
    |_t: &Interval, y: &Interval| y.clone(),
    Rational::from(0),
    Rational::from(1),
);

// Ten steps of size 1/10: enclosures of e^(k/10) for k = 1..=10
let trajectory = solver.solve(10, &Rational::from_frac(1, 10)).unwrap();
assert_eq!(trajectory.len(), 10);

// Integrate exactly to t = 1/3, with steps no larger than 1/4
let mut solver2 = OdeSolver::new(
    |_t: &Interval, y: &Interval| y.clone(),
    Rational::from(0),
    Rational::from(1),
);
let steps = solver2
    .solve_until(&Rational::from_frac(1, 3), &Rational::from_frac(1, 4))
    .unwrap();
assert_eq!(steps.len(), 2);
```

Each element of a trajectory is `(t_i, lo_i, hi_i)`. Methods that return a trajectory
keep the state reached by the last successful step if a later step fails.

Two accessors report the current state:

- `t()` is the current time.
- `y_box()` is the current enclosure. `y()` is its midpoint, a point estimate that is
  not itself verified.

`a_priori(h)` verifies a single step without advancing the solver. It is useful for
choosing `h` before committing to it.

## Vector systems

`VectorIntervalFn` takes the state as a slice of intervals, one per component, and
returns one interval per component:

```rust
use tpt_fl_exact_math::{Interval, Rational};
use tpt_fl_verified_ode::{VectorIntervalFn, VectorOdeSolver};

// Harmonic oscillator: y0' = y1, y1' = -y0
struct Oscillator;

impl VectorIntervalFn for Oscillator {
    fn eval(&self, _t: &Interval, y: &[Interval]) -> Vec<Interval> {
        vec![y[1].clone(), -y[0].clone()]
    }
}

let mut solver = VectorOdeSolver::new(
    Oscillator,
    Rational::from(0),
    vec![Rational::from(1), Rational::from(0)], // y0(0) = 1, y1(0) = 0
);
let enclosure = solver.step(&Rational::from_frac(1, 10)).unwrap();
assert_eq!(enclosure.len(), 2);
```

Each component has its own enclosure, and a step is verified for all components at
once. `solve`, `solve_until` and `a_priori` have the same shape as the scalar solver,
with a vector in place of each interval.

## Taylor-model solver

`TaylorOdeSolver` writes the right-hand side once and evaluates it on two algebras:

- `Interval`, for the a priori box, and
- `TaylorModel`, a polynomial in the step variable with an interval remainder, for the
  enclosure.

The right-hand side implements `TaylorFn`, and it is generic over `TaylorArith`:

```rust
use tpt_fl_exact_math::Rational;
use tpt_fl_verified_ode::{TaylorArith, TaylorFn, TaylorOdeSolver};

// Logistic growth: y' = y - y²
struct Logistic;

impl TaylorFn for Logistic {
    fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
        vec![y[0].clone() - y[0].clone() * y[0].clone()]
    }
}

// Taylor models of order 4
let mut solver = TaylorOdeSolver::new(Logistic, 4, Rational::from(0), vec![Rational::from_frac(1, 2)]);
let enclosure = solver.step(&Rational::from_frac(1, 10)).unwrap();
assert_eq!(enclosure.len(), 1);
```

For smooth problems the Taylor enclosure is usually much tighter than the Picard
end point, at more cost per step. Use `order` to trade cost against tightness. The
order must be at least `1`, or `new` panics.

## Errors

`OdeError` has two variants:

| Variant | Meaning |
|---------|---------|
| `NonPositiveStep` | `h` was zero or negative |
| `NoEnclosure` | No verified enclosure was found, even after halving the step as far as allowed. The solver is unchanged |

Handle both. An error means the solver could not give a guarantee, and returning an
unverified value would defeat the purpose of the crate.

## Guarantees

- **Enclosure.** When `f` is a correct interval extension, every accepted step contains
  the true solution at the end of the step.
- **No silent failure.** A step that cannot be verified returns `NoEnclosure`. An
  unverified box is never returned.
- **Exact arithmetic.** Bounds are `Rational` values from
  [`tpt-fl-exact-math`](https://github.com/tpt-solutions/tpt-formal-lab/tree/master/tpt-fl-exact-math), with endpoints rounded outward to
  multiples of 2⁻⁶⁴. This keeps denominators bounded over long integrations.
- **No `unsafe` code.**

## Limitations

- **Soundness depends on `f`.** The crate cannot check that `IntervalFn::eval`
  encloses `f`. A wrong interval extension gives a wrong enclosure without any
  error.
- **Step size.** Verification may need a smaller `h` than the solution's dynamics
  suggest. Use `a_priori` to check, and let the halving search find a step that
  works.
- **Blow-up.** Near a finite-time blow-up the solver may be unable to verify any
  step. It then returns `NoEnclosure` and does not return a box.
- **Cost.** Rational arithmetic is slower than floating point. Exact enclosures are
  for verification and reference results, not for real-time loops.
- **Taylor right-hand sides** may use only the operations of `TaylorArith`: `+`, `-`,
  `*`, unary `-` and constants. There is no division and no transcendental function,
  so a right-hand side that needs them cannot use `TaylorOdeSolver`.

## Crate features and MSRV

- No optional features.
- `#![no_std]`, using `alloc`.
- Minimum supported Rust version: **1.75**.

## Testing

```sh
cargo test -p tpt-fl-verified-ode
```

The tests in `tests/solver.rs` cover the scalar solver: non-monotone and stiff
right-hand sides, non-autonomous systems, failing fixpoints and width bounds.
`tests/vector.rs` covers the vector solver, and `tests/taylor.rs` covers the Taylor
solver.

## Changelog

See [CHANGELOG.md](CHANGELOG.md).

## License

Licensed under either of [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.
