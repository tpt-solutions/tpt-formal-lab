//! Taylor-model arithmetic for verified ODE enclosures.
//!
//! A `TaylorModel` stands for a function of `s = t - t0` on a domain `dom`
//! (for a step, `dom = [0, h]`). It is a polynomial in `s` with interval
//! coefficients, plus an interval remainder that absorbs everything else.
//! Every operation keeps this invariant: for every `s ∈ dom`, the true value
//! lies in `Σ c_k s^k + remainder`. Terms of degree at or above the order are
//! folded into the remainder by evaluating them over `dom`.

use alloc::vec::Vec;
use core::ops::{Add, Mul, Neg, Sub};

use tpt_fl_exact_math::{Interval, Rational};

use crate::solver::DYADIC_BITS;

/// The values a [`TaylorFn`] computes with: [`Interval`] or `TaylorModel`.
///
/// Every operation must be a sound enclosure of the real operation. The four
/// operators are enough for polynomial and rational right-hand sides.
pub trait TaylorArith:
    Clone + Add<Output = Self> + Sub<Output = Self> + Mul<Output = Self> + Neg<Output = Self>
{
    /// Returns the constant `c` in the same algebra as `self`.
    ///
    /// `self` is only a template. A `TaylorModel` takes its domain and order
    /// from it, and an [`Interval`] ignores it.
    fn constant(&self, c: Rational) -> Self;
}

impl TaylorArith for Interval {
    fn constant(&self, c: Rational) -> Self {
        Interval::point(c)
    }
}

impl TaylorArith for TaylorModel {
    fn constant(&self, c: Rational) -> Self {
        self.constant_like(Interval::point(c))
    }
}

/// A polynomial in `s` with interval coefficients and an interval remainder.
///
/// The number of coefficients is the order of the model. A model is valid for
/// `s` in its domain `dom`.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TaylorModel {
    coeffs: Vec<Interval>,
    rem: Interval,
    dom: Interval,
}

impl TaylorModel {
    /// Builds a model of the given order. Terms of degree `>= order` are moved
    /// into the remainder, and missing coefficients are zero.
    pub(crate) fn new(
        mut coeffs: Vec<Interval>,
        rem: Interval,
        dom: Interval,
        order: usize,
    ) -> Self {
        let excess = poly_sum(&coeffs, &dom, order);
        coeffs.truncate(order);
        coeffs.resize(order, zero());
        let coeffs = coeffs
            .into_iter()
            .map(|c| c.round_outward(DYADIC_BITS))
            .collect();
        Self {
            coeffs,
            rem: (rem + excess).round_outward(DYADIC_BITS),
            dom,
        }
    }

    /// The order of the model, that is, its number of coefficients.
    fn order(&self) -> usize {
        self.coeffs.len()
    }

    /// The constant model `c` with zero remainder, on the same domain and order.
    pub(crate) fn constant_like(&self, c: Interval) -> Self {
        Self::new(alloc::vec![c], zero(), self.dom.clone(), self.order())
    }

    /// Returns `∫_0^s`, the antiderivative that is zero at `s = 0`, with the
    /// same domain and order.
    pub(crate) fn integrate(self) -> Self {
        let order = self.order();
        let mut coeffs = Vec::with_capacity(order + 1);
        coeffs.push(zero());
        for (k, c) in self.coeffs.iter().enumerate() {
            let inv = Rational::from_frac(1, (k + 1) as i64);
            coeffs.push(c.clone() * Interval::point(inv));
        }
        // ∫_0^s r dτ lies in [0, s]·r, and [0, s] ⊆ dom.
        let rem = self.dom.clone() * self.rem;
        Self::new(coeffs, rem, self.dom, order)
    }

    /// Returns an enclosure of the value at the end of the domain, `s = dom.hi()`.
    pub(crate) fn at_end(&self) -> Interval {
        let end = Interval::point(self.dom.hi().clone());
        (poly_sum(&self.coeffs, &end, 0) + self.rem.clone()).round_outward(DYADIC_BITS)
    }
}

impl Add for TaylorModel {
    type Output = Self;

    fn add(self, rhs: Self) -> Self {
        let order = self.order();
        let coeffs = self
            .coeffs
            .iter()
            .zip(&rhs.coeffs)
            .map(|(a, b)| a.clone() + b.clone())
            .collect();
        Self::new(coeffs, self.rem + rhs.rem, self.dom, order)
    }
}

impl Neg for TaylorModel {
    type Output = Self;

    fn neg(self) -> Self {
        let order = self.order();
        let coeffs = self.coeffs.into_iter().map(|c| -c).collect();
        Self::new(coeffs, -self.rem, self.dom, order)
    }
}

impl Sub for TaylorModel {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self {
        self + (-rhs)
    }
}

impl Mul for TaylorModel {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self {
        let order = self.order();
        // Full convolution, with degree up to 2·order − 2. `new` folds the
        // part above the order into the remainder.
        let mut conv = alloc::vec![zero(); 2 * order - 1];
        for (i, a) in self.coeffs.iter().enumerate() {
            for (j, b) in rhs.coeffs.iter().enumerate() {
                conv[i + j] = conv[i + j].clone() + a.clone() * b.clone();
            }
        }
        // (p1 + r1)(p2 + r2) = p1 p2 + p1 r2 + p2 r1 + r1 r2. The range of each
        // polynomial over the domain bounds the cross terms.
        let range_a = poly_sum(&self.coeffs, &self.dom, 0);
        let range_b = poly_sum(&rhs.coeffs, &self.dom, 0);
        let rem = range_a * rhs.rem.clone() + range_b * self.rem.clone() + self.rem * rhs.rem;
        Self::new(conv, rem, self.dom, order)
    }
}

/// Returns `Σ_{k ≥ from} c_k x^k` as an enclosure, with each power rounded outward.
fn poly_sum(coeffs: &[Interval], x: &Interval, from: usize) -> Interval {
    let mut sum = zero();
    let mut pow = one();
    for (k, c) in coeffs.iter().enumerate() {
        if k >= from {
            sum = sum + c.clone() * pow.clone();
        }
        pow = (pow * x.clone()).round_outward(DYADIC_BITS);
    }
    sum.round_outward(DYADIC_BITS)
}

fn zero() -> Interval {
    Interval::point(Rational::zero())
}

fn one() -> Interval {
    Interval::point(Rational::one())
}

/// A right-hand side `f(t, y)` written once and evaluated on any [`TaylorArith`].
///
/// Use the operators and [`TaylorArith::constant`] for constants. Every
/// operation is evaluated in the chosen algebra, so the same code gives
/// interval enclosures for the a-priori box and Taylor models for the
/// polynomial part.
///
/// # Example
///
/// ```rust
/// use tpt_fl_verified_ode::{TaylorArith, TaylorFn};
///
/// // Logistic growth: y' = y - y²
/// struct Logistic;
/// impl TaylorFn for Logistic {
///     fn eval<A: TaylorArith>(&self, _t: &A, y: &[A]) -> Vec<A> {
///         vec![y[0].clone() - y[0].clone() * y[0].clone()]
///     }
/// }
/// ```
pub trait TaylorFn {
    /// Evaluates `f(t, y)` in the algebra `A`.
    fn eval<A: TaylorArith>(&self, t: &A, y: &[A]) -> Vec<A>;
}
