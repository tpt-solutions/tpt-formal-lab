//! Interval arithmetic — track worst-case error bounds through calculations.

use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

use crate::Rational;

/// A closed interval `[lo, hi]` over `Rational` values.
///
/// Every arithmetic operation on `Interval` produces the tightest possible
/// bounding interval for the result, given that the operands lie somewhere
/// within their respective intervals. This lets you propagate uncertainty or
/// worst-case error bounds through an entire computation.
///
/// # Example
///
/// ```rust
/// use tpt_exact_math::{Interval, Rational};
///
/// // x is somewhere between 1/10 and 3/10
/// let x = Interval::new(
///     Rational::from_frac(1, 10),
///     Rational::from_frac(3, 10),
/// );
///
/// // y is exactly 1/2
/// let y = Interval::point(Rational::from_frac(1, 2));
///
/// // z = x + y is between 6/10 and 8/10
/// let z = x + y;
/// assert!(z.contains(&Rational::from_frac(7, 10)));
/// assert!(!z.contains(&Rational::from_frac(9, 10)));
/// ```
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Interval {
    lo: Rational,
    hi: Rational,
}

impl Interval {
    /// Creates an interval `[lo, hi]`.
    ///
    /// # Panics
    ///
    /// Panics if `lo > hi`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let i = Interval::new(Rational::from_frac(0, 1), Rational::from_frac(1, 1));
    /// assert_eq!(i.width(), Rational::from_frac(1, 1));
    /// ```
    pub fn new(lo: Rational, hi: Rational) -> Self {
        assert!(lo <= hi, "Interval lower bound must be <= upper bound");
        Self { lo, hi }
    }

    /// Creates a degenerate interval `[x, x]` representing a single exact value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let p = Interval::point(Rational::from_frac(1, 2));
    /// assert_eq!(p.width(), Rational::zero());
    /// ```
    pub fn point(x: Rational) -> Self {
        Self {
            lo: x.clone(),
            hi: x,
        }
    }

    /// Returns the lower bound of the interval.
    pub fn lo(&self) -> &Rational {
        &self.lo
    }

    /// Returns the upper bound of the interval.
    pub fn hi(&self) -> &Rational {
        &self.hi
    }

    /// Returns the width `hi - lo` of the interval.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let i = Interval::new(Rational::from_frac(1, 4), Rational::from_frac(3, 4));
    /// assert_eq!(i.width(), Rational::from_frac(1, 2));
    /// ```
    pub fn width(&self) -> Rational {
        self.hi.clone() - self.lo.clone()
    }

    /// Returns the midpoint `(lo + hi) / 2`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let i = Interval::new(Rational::from_frac(0, 1), Rational::from_frac(1, 1));
    /// assert_eq!(i.midpoint(), Rational::from_frac(1, 2));
    /// ```
    pub fn midpoint(&self) -> Rational {
        (self.lo.clone() + self.hi.clone()) / Rational::from_frac(2, 1)
    }

    /// Returns `true` if `x` lies within `[lo, hi]`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let i = Interval::new(Rational::from_frac(0, 1), Rational::from_frac(1, 1));
    /// assert!(i.contains(&Rational::from_frac(1, 2)));
    /// assert!(!i.contains(&Rational::from_frac(3, 2)));
    /// ```
    pub fn contains(&self, x: &Rational) -> bool {
        &self.lo <= x && x <= &self.hi
    }

    /// Returns `true` if this interval completely contains `other`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let outer = Interval::new(Rational::from_frac(0, 1), Rational::from_frac(1, 1));
    /// let inner = Interval::new(Rational::from_frac(1, 4), Rational::from_frac(3, 4));
    /// assert!(outer.contains_interval(&inner));
    /// assert!(!inner.contains_interval(&outer));
    /// ```
    pub fn contains_interval(&self, other: &Interval) -> bool {
        self.lo <= other.lo && other.hi <= self.hi
    }

    /// Returns the smallest interval that contains both `self` and `other` (hull).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let a = Interval::new(Rational::from_frac(0, 1), Rational::from_frac(1, 2));
    /// let b = Interval::new(Rational::from_frac(1, 4), Rational::from_frac(3, 4));
    /// let h = a.hull(&b);
    /// assert_eq!(h.lo(), &Rational::from_frac(0, 1));
    /// assert_eq!(h.hi(), &Rational::from_frac(3, 4));
    /// ```
    pub fn hull(&self, other: &Interval) -> Interval {
        let lo = if self.lo <= other.lo {
            self.lo.clone()
        } else {
            other.lo.clone()
        };
        let hi = if self.hi >= other.hi {
            self.hi.clone()
        } else {
            other.hi.clone()
        };
        Interval { lo, hi }
    }
}

// ── Arithmetic — monotone interval propagation ────────────────────────────────

impl Add for Interval {
    type Output = Self;
    /// `[a,b] + [c,d] = [a+c, b+d]`
    fn add(self, rhs: Self) -> Self {
        Self {
            lo: self.lo + rhs.lo,
            hi: self.hi + rhs.hi,
        }
    }
}

impl Sub for Interval {
    type Output = Self;
    /// `[a,b] - [c,d] = [a-d, b-c]`
    fn sub(self, rhs: Self) -> Self {
        Self {
            lo: self.lo - rhs.hi,
            hi: self.hi - rhs.lo,
        }
    }
}

impl Neg for Interval {
    type Output = Self;
    /// `-[a,b] = [-b,-a]`
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}

impl Mul for Interval {
    type Output = Self;
    /// `[a,b] * [c,d]` — takes min/max over all four corner products.
    fn mul(self, rhs: Self) -> Self {
        let ac = self.lo.clone() * rhs.lo.clone();
        let ad = self.lo.clone() * rhs.hi.clone();
        let bc = self.hi.clone() * rhs.lo.clone();
        let bd = self.hi.clone() * rhs.hi.clone();
        let lo = ac.clone().min(ad.clone()).min(bc.clone()).min(bd.clone());
        let hi = ac.max(ad).max(bc).max(bd);
        Self { lo, hi }
    }
}

impl Div for Interval {
    type Output = Self;
    /// `[a,b] / [c,d]` — multiplies by the reciprocal interval.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` contains zero (division by an interval spanning zero is
    /// undefined in standard interval arithmetic).
    fn div(self, rhs: Self) -> Self {
        assert!(
            !rhs.contains(&Rational::zero()),
            "Interval division by an interval containing zero"
        );
        let recip = Interval::new(rhs.lo.recip(), rhs.hi.recip());
        self * recip
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl fmt::Display for Interval {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}, {}]", self.lo, self.hi)
    }
}

// ── Helpers for Rational ──────────────────────────────────────────────────────

impl Rational {
    fn min(self, other: Self) -> Self {
        if self <= other {
            self
        } else {
            other
        }
    }
    fn max(self, other: Self) -> Self {
        if self >= other {
            self
        } else {
            other
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn r(n: i64, d: i64) -> Rational {
        Rational::from_frac(n, d)
    }

    #[test]
    fn addition_is_correct() {
        let a = Interval::new(r(1, 10), r(3, 10));
        let b = Interval::new(r(2, 10), r(4, 10));
        let c = a + b;
        assert_eq!(c.lo(), &r(3, 10));
        assert_eq!(c.hi(), &r(7, 10));
    }

    #[test]
    fn subtraction_swaps_rhs_bounds() {
        // [1,3] - [1,2] = [1-2, 3-1] = [-1, 2]
        let a = Interval::new(r(1, 1), r(3, 1));
        let b = Interval::new(r(1, 1), r(2, 1));
        let c = a - b;
        assert_eq!(c.lo(), &r(-1, 1));
        assert_eq!(c.hi(), &r(2, 1));
    }

    #[test]
    fn negation() {
        let a = Interval::new(r(1, 4), r(3, 4));
        let b = -a;
        assert_eq!(b.lo(), &r(-3, 4));
        assert_eq!(b.hi(), &r(-1, 4));
    }

    #[test]
    fn multiplication_all_positive() {
        // [1,2] * [3,4] = [3, 8]
        let a = Interval::new(r(1, 1), r(2, 1));
        let b = Interval::new(r(3, 1), r(4, 1));
        let c = a * b;
        assert_eq!(c.lo(), &r(3, 1));
        assert_eq!(c.hi(), &r(8, 1));
    }

    #[test]
    fn multiplication_mixed_signs() {
        // [-1,2] * [-1,3]  corners: 1, -3, -2, 6 → [min=-3, max=6]
        let a = Interval::new(r(-1, 1), r(2, 1));
        let b = Interval::new(r(-1, 1), r(3, 1));
        let c = a * b;
        assert_eq!(c.lo(), &r(-3, 1));
        assert_eq!(c.hi(), &r(6, 1));
    }

    #[test]
    fn point_interval_width_zero() {
        let p = Interval::point(r(1, 2));
        assert_eq!(p.width(), Rational::zero());
    }

    #[test]
    fn contains() {
        let i = Interval::new(r(0, 1), r(1, 1));
        assert!(i.contains(&r(1, 2)));
        assert!(i.contains(&r(0, 1)));
        assert!(i.contains(&r(1, 1)));
        assert!(!i.contains(&r(2, 1)));
        assert!(!i.contains(&r(-1, 1)));
    }

    #[test]
    fn midpoint() {
        let i = Interval::new(r(0, 1), r(1, 1));
        assert_eq!(i.midpoint(), r(1, 2));
    }

    #[test]
    fn hull() {
        let a = Interval::new(r(0, 1), r(1, 2));
        let b = Interval::new(r(1, 4), r(3, 4));
        let h = a.hull(&b);
        assert_eq!(h.lo(), &r(0, 1));
        assert_eq!(h.hi(), &r(3, 4));
    }

    #[test]
    fn display() {
        let i = Interval::new(r(1, 4), r(3, 4));
        assert_eq!(i.to_string(), "[1/4, 3/4]");
    }
}
