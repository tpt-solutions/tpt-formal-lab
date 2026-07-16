//! Arbitrary-precision rational numbers.

use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};
use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, Zero};

/// An exact rational number `numer / denom` in lowest terms.
///
/// - The denominator is always positive and non-zero.
/// - The fraction is always fully reduced (GCD of numerator and denominator is 1).
/// - Backed by arbitrary-precision [`BigInt`] integers — no overflow, no rounding.
///
/// # Example
///
/// ```rust
/// use tpt_exact_math::Rational;
///
/// let a = Rational::from_frac(1, 3);
/// let b = Rational::from_frac(1, 6);
/// assert_eq!(a + b, Rational::from_frac(1, 2));
/// ```
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Rational {
    numer: BigInt,
    denom: BigInt,
}

impl Rational {
    /// Creates a new `Rational` equal to `numer / denom`, reduced to lowest terms.
    ///
    /// # Panics
    ///
    /// Panics if `denom` is zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    /// use num_bigint::BigInt;
    ///
    /// let r = Rational::new(BigInt::from(4), BigInt::from(6));
    /// assert_eq!(r, Rational::from_frac(2, 3));
    /// ```
    pub fn new(numer: BigInt, denom: BigInt) -> Self {
        assert!(!denom.is_zero(), "Rational denominator must not be zero");
        Self::reduce(numer, denom)
    }

    /// Creates a `Rational` from `i64` numerator and denominator.
    ///
    /// # Panics
    ///
    /// Panics if `denom` is zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// let r = Rational::from_frac(3, 4);
    /// assert_eq!(r.to_string(), "3/4");
    /// ```
    pub fn from_frac(numer: i64, denom: i64) -> Self {
        Self::new(BigInt::from(numer), BigInt::from(denom))
    }

    /// Returns the rational number zero (0/1).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert!(Rational::zero().is_zero());
    /// ```
    pub fn zero() -> Self {
        Self {
            numer: BigInt::zero(),
            denom: BigInt::one(),
        }
    }

    /// Returns the rational number one (1/1).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert_eq!(Rational::one() + Rational::one(), Rational::from_frac(2, 1));
    /// ```
    pub fn one() -> Self {
        Self {
            numer: BigInt::one(),
            denom: BigInt::one(),
        }
    }

    /// Returns `true` if this rational is zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert!(Rational::zero().is_zero());
    /// assert!(!Rational::one().is_zero());
    /// ```
    pub fn is_zero(&self) -> bool {
        self.numer.is_zero()
    }

    /// Returns `true` if this rational is strictly negative.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert!(Rational::from_frac(-1, 2).is_negative());
    /// assert!(!Rational::from_frac(1, 2).is_negative());
    /// ```
    pub fn is_negative(&self) -> bool {
        self.numer.is_negative()
    }

    /// Returns `true` if this rational is strictly positive.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert!(Rational::from_frac(1, 2).is_positive());
    /// assert!(!Rational::from_frac(-1, 2).is_positive());
    /// ```
    pub fn is_positive(&self) -> bool {
        self.numer.is_positive()
    }

    /// Returns the absolute value of this rational.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert_eq!(Rational::from_frac(-3, 4).abs(), Rational::from_frac(3, 4));
    /// ```
    pub fn abs(&self) -> Self {
        Self {
            numer: self.numer.abs(),
            denom: self.denom.clone(),
        }
    }

    /// Returns the multiplicative reciprocal (1/self).
    ///
    /// # Panics
    ///
    /// Panics if `self` is zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert_eq!(Rational::from_frac(3, 4).recip(), Rational::from_frac(4, 3));
    /// ```
    pub fn recip(&self) -> Self {
        assert!(!self.is_zero(), "Cannot take reciprocal of zero");
        Self::new(self.denom.clone(), self.numer.clone())
    }

    /// Returns a reference to the numerator.
    pub fn numer(&self) -> &BigInt {
        &self.numer
    }

    /// Returns a reference to the denominator (always positive).
    pub fn denom(&self) -> &BigInt {
        &self.denom
    }

    /// Converts to an `f64`, potentially losing precision.
    ///
    /// Use only for display or interoperability — not for computation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// let r = Rational::from_frac(1, 4);
    /// assert!((r.to_f64() - 0.25).abs() < 1e-15);
    /// ```
    pub fn to_f64(&self) -> f64 {
        use num_traits::ToPrimitive;
        let n = self.numer.to_f64().unwrap_or(f64::NAN);
        let d = self.denom.to_f64().unwrap_or(f64::NAN);
        n / d
    }

    fn reduce(numer: BigInt, denom: BigInt) -> Self {
        if numer.is_zero() {
            return Self { numer: BigInt::zero(), denom: BigInt::one() };
        }
        // Ensure denom is positive
        let (numer, denom) = if denom < BigInt::zero() {
            (-numer, -denom)
        } else {
            (numer, denom)
        };
        let g = numer.gcd(&denom);
        Self {
            numer: numer / &g,
            denom: denom / &g,
        }
    }
}

// ── From conversions ──────────────────────────────────────────────────────────

impl From<i64> for Rational {
    fn from(n: i64) -> Self {
        Self { numer: BigInt::from(n), denom: BigInt::one() }
    }
}

impl From<i32> for Rational {
    fn from(n: i32) -> Self {
        Self::from(n as i64)
    }
}

impl From<u64> for Rational {
    fn from(n: u64) -> Self {
        Self { numer: BigInt::from(n), denom: BigInt::one() }
    }
}

impl From<BigInt> for Rational {
    fn from(n: BigInt) -> Self {
        Self { numer: n, denom: BigInt::one() }
    }
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

impl Add for Rational {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        // a/b + c/d = (a*d + c*b) / (b*d)
        let numer = &self.numer * &rhs.denom + &rhs.numer * &self.denom;
        let denom = &self.denom * &rhs.denom;
        Self::reduce(numer, denom)
    }
}

impl Sub for Rational {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        let numer = &self.numer * &rhs.denom - &rhs.numer * &self.denom;
        let denom = &self.denom * &rhs.denom;
        Self::reduce(numer, denom)
    }
}

impl Mul for Rational {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self {
        Self::reduce(&self.numer * &rhs.numer, &self.denom * &rhs.denom)
    }
}

impl Div for Rational {
    type Output = Self;
    fn div(self, rhs: Self) -> Self {
        assert!(!rhs.is_zero(), "Division by zero");
        Self::reduce(&self.numer * &rhs.denom, &self.denom * &rhs.numer)
    }
}

impl Neg for Rational {
    type Output = Self;
    fn neg(self) -> Self {
        Self { numer: -self.numer, denom: self.denom }
    }
}

// ── Ordering ──────────────────────────────────────────────────────────────────

impl PartialOrd for Rational {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Rational {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        // a/b <=> c/d  iff  a*d <=> c*b  (both denoms positive)
        let lhs = &self.numer * &other.denom;
        let rhs = &other.numer * &self.denom;
        lhs.cmp(&rhs)
    }
}

// ── Display / Debug ───────────────────────────────────────────────────────────

impl fmt::Display for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.denom == BigInt::one() {
            write!(f, "{}", self.numer)
        } else {
            write!(f, "{}/{}", self.numer, self.denom)
        }
    }
}

impl fmt::Debug for Rational {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Rational({}/{})", self.numer, self.denom)
    }
}

// ── num-traits compatibility ──────────────────────────────────────────────────

impl Zero for Rational {
    fn zero() -> Self { Rational::zero() }
    fn is_zero(&self) -> bool { Rational::is_zero(self) }
}

impl One for Rational {
    fn one() -> Self { Rational::one() }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;
    use num_bigint::Sign;

    #[test]
    fn addition_reduces() {
        let a = Rational::from_frac(1, 3);
        let b = Rational::from_frac(1, 6);
        assert_eq!(a + b, Rational::from_frac(1, 2));
    }

    #[test]
    fn subtraction() {
        let a = Rational::from_frac(3, 4);
        let b = Rational::from_frac(1, 4);
        assert_eq!(a - b, Rational::from_frac(1, 2));
    }

    #[test]
    fn multiplication() {
        let a = Rational::from_frac(2, 3);
        let b = Rational::from_frac(3, 4);
        assert_eq!(a * b, Rational::from_frac(1, 2));
    }

    #[test]
    fn division() {
        let a = Rational::from_frac(1, 2);
        let b = Rational::from_frac(3, 4);
        assert_eq!(a / b, Rational::from_frac(2, 3));
    }

    #[test]
    fn negation() {
        let a = Rational::from_frac(3, 4);
        assert_eq!(-a, Rational::from_frac(-3, 4));
    }

    #[test]
    fn negative_denom_normalised() {
        // -1/-2 should become 1/2
        let r = Rational::new(BigInt::from(-1), BigInt::from(-2));
        assert_eq!(r, Rational::from_frac(1, 2));
        assert!(r.denom().sign() == Sign::Plus);
    }

    #[test]
    fn ordering() {
        let a = Rational::from_frac(1, 3);
        let b = Rational::from_frac(1, 2);
        assert!(a < b);
        assert!(b > a);
        assert_eq!(Rational::from_frac(2, 4), Rational::from_frac(1, 2));
    }

    #[test]
    fn zero_stays_canonical() {
        // 0/anything should reduce to 0/1
        let z = Rational::new(BigInt::zero(), BigInt::from(42));
        assert_eq!(z.denom(), &BigInt::one());
    }

    #[test]
    fn point_one_plus_point_two_is_exact() {
        // The famous floating-point pitfall: 0.1 + 0.2 != 0.3 in f64
        // In exact arithmetic it is perfectly 3/10
        let a = Rational::from_frac(1, 10);
        let b = Rational::from_frac(2, 10);
        let c = Rational::from_frac(3, 10);
        assert_eq!(a + b, c);
    }

    #[test]
    fn recip_roundtrip() {
        let a = Rational::from_frac(3, 7);
        let b = a.clone().recip();
        assert_eq!(a * b, Rational::one());
    }

    #[test]
    fn display() {
        assert_eq!(Rational::from_frac(3, 4).to_string(), "3/4");
        assert_eq!(Rational::from_frac(6, 3).to_string(), "2");
        assert_eq!(Rational::from_frac(-1, 2).to_string(), "-1/2");
    }
}
