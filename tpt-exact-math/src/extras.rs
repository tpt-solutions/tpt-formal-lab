//! Parsing, exact `f64` conversion, powers, rounding, reference operators,
//! extra `From` impls, and the set and power operations on [`Interval`].

use core::fmt;
use core::ops::{Add, Div, Mul, Sub};
use core::str::FromStr;

use num_bigint::BigInt;
use num_traits::{One, Zero};

use crate::{Interval, Rational};

// ── Parsing ───────────────────────────────────────────────────────────────────

/// An error from parsing a [`Rational`] with [`FromStr`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseRationalError {
    reason: &'static str,
}

impl ParseRationalError {
    fn new(reason: &'static str) -> Self {
        Self { reason }
    }
}

impl fmt::Display for ParseRationalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid rational: {}", self.reason)
    }
}

/// Parses `[+-]digits`, `[+-]digits/digits`, or a decimal such as `-1.25` or `.5`.
///
/// Decimals are read exactly, so `"0.1"` is `1/10`. Whitespace and exponents are
/// not accepted.
///
/// # Example
///
/// ```rust
/// use tpt_exact_math::Rational;
///
/// assert_eq!("-3/4".parse::<Rational>().unwrap(), Rational::from_frac(-3, 4));
/// assert_eq!("0.1".parse::<Rational>().unwrap(), Rational::from_frac(1, 10));
/// assert!("1/0".parse::<Rational>().is_err());
/// ```
impl FromStr for Rational {
    type Err = ParseRationalError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((numer, denom)) = s.split_once('/') {
            let numer = parse_integer(numer)?;
            let denom = parse_integer(denom)?;
            if denom.is_zero() {
                return Err(ParseRationalError::new("zero denominator"));
            }
            return Ok(Rational::new(numer, denom));
        }

        if let Some((int_part, frac_part)) = s.split_once('.') {
            let negative = int_part.starts_with('-');
            let unsigned_int = int_part.trim_start_matches(['+', '-']);
            if !frac_part.bytes().all(|b| b.is_ascii_digit())
                || frac_part.is_empty()
                || (unsigned_int.is_empty() && frac_part.is_empty())
            {
                return Err(ParseRationalError::new("malformed decimal"));
            }
            let int_digits = if unsigned_int.is_empty() {
                "0"
            } else {
                unsigned_int
            };
            if !int_digits.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ParseRationalError::new("malformed decimal"));
            }
            let digits = alloc::format!("{int_digits}{frac_part}");
            let mut numer = digits
                .parse::<BigInt>()
                .map_err(|_| ParseRationalError::new("malformed decimal"))?;
            if negative {
                numer = -numer;
            }
            let denom = BigInt::from(10).pow(frac_part.len() as u32);
            return Ok(Rational::new(numer, denom));
        }

        parse_integer(s).map(Rational::new_integer)
    }
}

/// Parses an optionally signed run of ASCII digits.
fn parse_integer(s: &str) -> Result<BigInt, ParseRationalError> {
    let digits = s.trim_start_matches(['+', '-']);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ParseRationalError::new("expected an integer"));
    }
    s.parse::<BigInt>()
        .map_err(|_| ParseRationalError::new("expected an integer"))
}

impl Rational {
    /// Wraps an integer as a rational. Helper for [`FromStr`].
    fn new_integer(n: BigInt) -> Self {
        Rational::from(n)
    }

    /// Converts an `f64` to the exact rational it represents.
    ///
    /// Every finite `f64` is a dyadic rational, so the conversion is exact.
    /// Returns `None` for NaN and the infinities.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert_eq!(Rational::from_f64(0.75), Some(Rational::from_frac(3, 4)));
    /// assert_eq!(Rational::from_f64(f64::NAN), None);
    /// ```
    pub fn from_f64(x: f64) -> Option<Self> {
        if !x.is_finite() {
            return None;
        }
        if x == 0.0 {
            return Some(Rational::zero());
        }
        let bits = x.to_bits();
        let negative = bits >> 63 == 1;
        let biased_exp = ((bits >> 52) & 0x7ff) as i64;
        let fraction = bits & ((1u64 << 52) - 1);
        // Normal numbers have an implicit leading 1 bit. Subnormals do not.
        let (mantissa, exponent) = if biased_exp == 0 {
            (fraction, -1074)
        } else {
            (fraction | (1u64 << 52), biased_exp - 1075)
        };
        let magnitude = BigInt::from(mantissa);
        let (numer, denom) = if exponent >= 0 {
            (magnitude << exponent as usize, BigInt::one())
        } else {
            (magnitude, BigInt::one() << (-exponent) as usize)
        };
        Some(Rational::new(if negative { -numer } else { numer }, denom))
    }

    /// Raises `self` to a non-negative integer power.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert_eq!(Rational::from_frac(-2, 3).pow(3), Rational::from_frac(-8, 27));
    /// assert_eq!(Rational::from_frac(5, 7).pow(0), Rational::one());
    /// ```
    pub fn pow(&self, exp: u32) -> Self {
        Rational::new(self.numer().pow(exp), self.denom().pow(exp))
    }

    /// Rounds to the nearest integer, with halves rounded away from zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::Rational;
    ///
    /// assert_eq!(Rational::from_frac(5, 2).round(), Rational::from(3));
    /// assert_eq!(Rational::from_frac(-5, 2).round(), Rational::from(-3));
    /// assert_eq!(Rational::from_frac(7, 3).round(), Rational::from(2));
    /// ```
    pub fn round(&self) -> Self {
        let half = Rational::from_frac(1, 2);
        if self.is_negative() {
            -((-self.clone()) + half).floor()
        } else {
            (self.clone() + half).floor()
        }
    }
}

// ── Reference operators ───────────────────────────────────────────────────────

macro_rules! impl_ref_binop {
    ($trait:ident, $method:ident) => {
        impl $trait<&Rational> for &Rational {
            type Output = Rational;
            fn $method(self, rhs: &Rational) -> Rational {
                $trait::$method(self.clone(), rhs.clone())
            }
        }

        impl $trait<&Rational> for Rational {
            type Output = Rational;
            fn $method(self, rhs: &Rational) -> Rational {
                $trait::$method(self, rhs.clone())
            }
        }

        impl $trait<Rational> for &Rational {
            type Output = Rational;
            fn $method(self, rhs: Rational) -> Rational {
                $trait::$method(self.clone(), rhs)
            }
        }
    };
}

impl_ref_binop!(Add, add);
impl_ref_binop!(Sub, sub);
impl_ref_binop!(Mul, mul);
impl_ref_binop!(Div, div);

// ── Further From impls ────────────────────────────────────────────────────────

macro_rules! impl_from_small_int {
    ($($t:ty),*) => {
        $(
            impl From<$t> for Rational {
                fn from(n: $t) -> Self {
                    Rational::from(i128::from(n))
                }
            }
        )*
    };
}

impl_from_small_int!(i8, i16, u8, u16);

impl From<u32> for Rational {
    fn from(n: u32) -> Self {
        Rational::from(u64::from(n))
    }
}

impl From<i128> for Rational {
    fn from(n: i128) -> Self {
        Rational::from(BigInt::from(n))
    }
}

impl From<u128> for Rational {
    fn from(n: u128) -> Self {
        Rational::from(BigInt::from(n))
    }
}

// ── Interval operations ───────────────────────────────────────────────────────

impl Interval {
    /// Returns the overlap of two intervals, or `None` if they are disjoint.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let a = Interval::new(Rational::from(0), Rational::from(2));
    /// let b = Interval::new(Rational::from(1), Rational::from(5));
    /// let c = a.intersect(&b).unwrap();
    /// assert_eq!((c.lo(), c.hi()), (&Rational::from(1), &Rational::from(2)));
    /// ```
    pub fn intersect(&self, other: &Interval) -> Option<Interval> {
        let lo = core::cmp::max(self.lo(), other.lo()).clone();
        let hi = core::cmp::min(self.hi(), other.hi()).clone();
        (lo <= hi).then(|| Interval::new(lo, hi))
    }

    /// Returns `[|x|]` over the interval: the image of `|·|`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let a = Interval::new(Rational::from(-3), Rational::from(2));
    /// assert_eq!(a.abs().hi(), &Rational::from(3));
    /// assert_eq!(a.abs().lo(), &Rational::from(0));
    /// ```
    pub fn abs(&self) -> Interval {
        let zero = Rational::zero();
        if self.lo() >= &zero {
            self.clone()
        } else if self.hi() <= &zero {
            -self.clone()
        } else {
            let hi = core::cmp::max(-self.lo().clone(), self.hi().clone());
            Interval::new(zero, hi)
        }
    }

    /// Returns the square of every value in the interval. Equal to `self.pow(2)`.
    pub fn sqr(&self) -> Interval {
        self.pow(2)
    }

    /// Returns `{x^n : x ∈ self}` as an enclosure, for a non-negative integer `n`.
    ///
    /// Odd powers are increasing, so the bounds are the powers of the bounds.
    /// Even powers depend only on `|x|`, so an interval that straddles zero has
    /// a lower bound of zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let a = Interval::new(Rational::from(-2), Rational::from(3));
    /// let p = a.pow(2);
    /// assert_eq!(p.lo(), &Rational::from(0));
    /// assert_eq!(p.hi(), &Rational::from(9));
    /// assert_eq!(a.pow(3).lo(), &Rational::from(-8));
    /// ```
    pub fn pow(&self, n: u32) -> Interval {
        if n == 0 {
            return Interval::point(Rational::one());
        }
        if n % 2 == 1 {
            return Interval::new(self.lo().pow(n), self.hi().pow(n));
        }
        let zero = Rational::zero();
        let magnitude = self.abs();
        let lo = if self.lo() <= &zero && self.hi() >= &zero {
            Rational::zero()
        } else {
            magnitude.lo().pow(n)
        };
        Interval::new(lo, magnitude.hi().pow(n))
    }

    /// Splits the interval at its midpoint into two halves that share the midpoint.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_exact_math::{Interval, Rational};
    ///
    /// let a = Interval::new(Rational::from(0), Rational::from(1));
    /// let (left, right) = a.bisect();
    /// assert_eq!(left.hi(), &Rational::from_frac(1, 2));
    /// assert_eq!(right.lo(), &Rational::from_frac(1, 2));
    /// ```
    pub fn bisect(&self) -> (Interval, Interval) {
        let mid = self.midpoint();
        (
            Interval::new(self.lo().clone(), mid.clone()),
            Interval::new(mid, self.hi().clone()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::string::ToString;

    fn r(n: i64, d: i64) -> Rational {
        Rational::from_frac(n, d)
    }

    #[test]
    fn parses_integers_fractions_and_decimals() {
        assert_eq!("42".parse::<Rational>().unwrap(), Rational::from(42));
        assert_eq!("+7".parse::<Rational>().unwrap(), Rational::from(7));
        assert_eq!("-3/4".parse::<Rational>().unwrap(), r(-3, 4));
        assert_eq!("6/4".parse::<Rational>().unwrap(), r(3, 2));
        assert_eq!("1.25".parse::<Rational>().unwrap(), r(5, 4));
        assert_eq!("-0.5".parse::<Rational>().unwrap(), r(-1, 2));
        assert_eq!(".5".parse::<Rational>().unwrap(), r(1, 2));
        assert_eq!(
            "0.1".parse::<Rational>().unwrap() + "0.2".parse::<Rational>().unwrap(),
            r(3, 10)
        );
    }

    #[test]
    fn rejects_malformed_input() {
        for bad in [
            "", "-", "1/", "/2", "1/0", "1.", "1.2.3", "abc", "1e3", " 1", "--1",
        ] {
            assert!(bad.parse::<Rational>().is_err(), "accepted {bad:?}");
        }
        assert!("1/0"
            .parse::<Rational>()
            .unwrap_err()
            .to_string()
            .contains("zero denominator"));
    }

    #[test]
    fn parse_display_round_trip() {
        for v in [r(1, 3), r(-22, 7), Rational::from(0), Rational::from(-9)] {
            assert_eq!(v.to_string().parse::<Rational>().unwrap(), v);
        }
    }

    #[test]
    fn from_f64_is_exact() {
        assert_eq!(Rational::from_f64(0.75), Some(r(3, 4)));
        assert_eq!(Rational::from_f64(-2.0), Some(Rational::from(-2)));
        assert_eq!(Rational::from_f64(0.0), Some(Rational::zero()));
        // 0.1 is not 1/10 in binary: the exact value is a dyadic with denominator 2^55.
        let tenth = Rational::from_f64(0.1).unwrap();
        assert_ne!(tenth, r(1, 10));
        assert!((tenth.to_f64() - 0.1).abs() < 1e-17);
        assert_eq!(Rational::from_f64(f64::INFINITY), None);
        assert_eq!(Rational::from_f64(f64::NAN), None);
    }

    #[test]
    fn from_f64_handles_subnormals() {
        let tiny = f64::from_bits(1); // smallest positive subnormal, 2^-1074
        let v = Rational::from_f64(tiny).unwrap();
        assert_eq!(v, Rational::new(BigInt::one(), BigInt::one() << 1074usize));
    }

    #[test]
    fn pow_and_round() {
        assert_eq!(r(2, 3).pow(0), Rational::one());
        assert_eq!(r(-2, 3).pow(2), r(4, 9));
        assert_eq!(r(-2, 3).pow(3), r(-8, 27));
        assert_eq!(r(5, 2).round(), Rational::from(3));
        assert_eq!(r(-5, 2).round(), Rational::from(-3));
        assert_eq!(r(1, 3).round(), Rational::zero());
        assert_eq!(r(-1, 3).round(), Rational::zero());
    }

    #[test]
    fn reference_operators_match_value_operators() {
        let a = r(1, 3);
        let b = r(1, 6);
        assert_eq!(&a + &b, r(1, 2));
        assert_eq!(&a - &b, r(1, 6));
        assert_eq!(&a * &b, r(1, 18));
        assert_eq!(&a / &b, Rational::from(2));
        assert_eq!(a.clone() + &b, r(1, 2));
        assert_eq!(&a + b, r(1, 2));
    }

    #[test]
    fn from_impls_for_all_integer_widths() {
        assert_eq!(Rational::from(-8i8), Rational::from(-8));
        assert_eq!(Rational::from(300u16), Rational::from(300));
        assert_eq!(
            Rational::from(u32::MAX),
            Rational::from(i64::from(u32::MAX))
        );
        assert_eq!(
            Rational::from(i128::MIN),
            Rational::from(BigInt::from(i128::MIN))
        );
        assert_eq!(
            Rational::from(u128::MAX),
            Rational::from(BigInt::from(u128::MAX))
        );
    }

    #[test]
    fn interval_intersection() {
        let a = Interval::new(r(0, 1), r(2, 1));
        let b = Interval::new(r(1, 1), r(5, 1));
        let c = Interval::new(r(3, 1), r(4, 1));
        assert_eq!(a.intersect(&b), Some(Interval::new(r(1, 1), r(2, 1))));
        assert_eq!(a.intersect(&c), None);
        assert_eq!(a.intersect(&a), Some(a.clone()));
    }

    #[test]
    fn interval_abs_for_each_sign_pattern() {
        let pos = Interval::new(r(1, 1), r(2, 1));
        let neg = Interval::new(r(-2, 1), r(-1, 1));
        let mixed = Interval::new(r(-3, 1), r(2, 1));
        assert_eq!(pos.abs(), pos);
        assert_eq!(neg.abs(), Interval::new(r(1, 1), r(2, 1)));
        assert_eq!(mixed.abs(), Interval::new(r(0, 1), r(3, 1)));
    }

    #[test]
    fn interval_powers_are_enclosures() {
        // Sample points of x in [-2, 3] must lie inside x^n's enclosure.
        let a = Interval::new(r(-2, 1), r(3, 1));
        for n in 0..6u32 {
            let enc = a.pow(n);
            for k in -20..=30 {
                let x = r(k, 10);
                assert!(enc.contains(&x.pow(n)) || !a.contains(&x), "n={n} x={x}");
            }
        }
        assert_eq!(a.sqr(), a.pow(2));
    }

    #[test]
    fn interval_bisect_covers_the_original() {
        let a = Interval::new(r(-1, 1), r(3, 1));
        let (left, right) = a.bisect();
        assert_eq!(left.lo(), a.lo());
        assert_eq!(right.hi(), a.hi());
        assert_eq!(left.hi(), right.lo());
        assert!(a.contains_interval(&left) && a.contains_interval(&right));
    }
}
