//! `FixedPoint<DENOM>` — cross-platform bitwise-deterministic fixed-point numbers.

use core::fmt;
use core::ops::{Add, Div, Mul, Neg, Sub};

/// A fixed-point rational number `raw / DENOM` where `DENOM` is a compile-time
/// constant denominator.
///
/// All arithmetic uses integer operations only — no floating-point at any step —
/// guaranteeing that results are bitwise identical across all CPU architectures.
///
/// The type parameter `DENOM` sets the precision. Common choices:
/// - `1_000` — 3 decimal places
/// - `1_000_000` — 6 decimal places (microsecond-level timesteps)
/// - `1_000_000_000` — 9 decimal places
///
/// # Example
///
/// ```rust
/// use tpt_deterministic_sim::FixedPoint;
///
/// type Fp = FixedPoint<1_000>; // 3 decimal places
///
/// let a = Fp::from_int(1);          //  1.000
/// let b = Fp::from_raw(500);        //  0.500
/// let c = a + b;
/// assert_eq!(c.raw(), 1_500);       //  1.500
/// assert_eq!(c.to_string(), "1.500");
/// ```
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FixedPoint<const DENOM: i64> {
    raw: i64,
}

impl<const DENOM: i64> FixedPoint<DENOM> {
    /// Creates a `FixedPoint` from a raw integer numerator.
    ///
    /// The value represented is `raw / DENOM`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// let half = Fp::from_raw(500); // 0.500
    /// assert_eq!(half.raw(), 500);
    /// ```
    pub const fn from_raw(raw: i64) -> Self {
        Self { raw }
    }

    /// Creates a `FixedPoint` from an integer value (multiplied by `DENOM`).
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// let three = Fp::from_int(3);
    /// assert_eq!(three.raw(), 3_000);
    /// ```
    pub const fn from_int(n: i64) -> Self {
        Self { raw: n * DENOM }
    }

    /// Returns zero.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::FixedPoint;
    /// assert_eq!(FixedPoint::<1_000>::zero().raw(), 0);
    /// ```
    pub const fn zero() -> Self {
        Self { raw: 0 }
    }

    /// Returns one (= `DENOM / DENOM`).
    pub const fn one() -> Self {
        Self { raw: DENOM }
    }

    /// Returns the raw integer numerator.
    pub const fn raw(self) -> i64 {
        self.raw
    }

    /// Returns `true` if this value is zero.
    pub const fn is_zero(self) -> bool {
        self.raw == 0
    }

    /// Returns `true` if this value is strictly negative.
    pub const fn is_negative(self) -> bool {
        self.raw < 0
    }

    /// Returns `true` if this value is strictly positive.
    pub const fn is_positive(self) -> bool {
        self.raw > 0
    }

    /// Returns the absolute value.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tpt_deterministic_sim::FixedPoint;
    /// type Fp = FixedPoint<1_000>;
    /// assert_eq!(Fp::from_raw(-500).abs(), Fp::from_raw(500));
    /// ```
    pub const fn abs(self) -> Self {
        Self {
            raw: self.raw.abs(),
        }
    }

    /// Converts to `f64` for display or interoperability only.
    ///
    /// Do not use the result in further simulation computations.
    pub fn to_f64(self) -> f64 {
        self.raw as f64 / DENOM as f64
    }

    /// Converts from an `f64` by rounding toward zero.
    ///
    /// Lossy — prefer [`from_raw`](Self::from_raw) or [`from_int`](Self::from_int)
    /// for deterministic construction.
    pub fn from_f64_truncating(v: f64) -> Self {
        Self {
            raw: (v * DENOM as f64) as i64,
        }
    }
}

// ── Arithmetic ────────────────────────────────────────────────────────────────

impl<const DENOM: i64> Add for FixedPoint<DENOM> {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            raw: self
                .raw
                .checked_add(rhs.raw)
                .expect("FixedPoint addition overflow"),
        }
    }
}

impl<const DENOM: i64> Sub for FixedPoint<DENOM> {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            raw: self
                .raw
                .checked_sub(rhs.raw)
                .expect("FixedPoint subtraction overflow"),
        }
    }
}

impl<const DENOM: i64> Neg for FixedPoint<DENOM> {
    type Output = Self;
    fn neg(self) -> Self {
        Self { raw: -self.raw }
    }
}

impl<const DENOM: i64> Mul for FixedPoint<DENOM> {
    type Output = Self;
    /// Multiplies two fixed-point values: `(a/D) * (b/D) = (a*b) / D²`,
    /// then rescales back to `/ D` by dividing the raw product by `DENOM`.
    #[allow(clippy::suspicious_arithmetic_impl)] // rescaling division is correct fixed-point math, not a bug
    fn mul(self, rhs: Self) -> Self {
        let prod = self
            .raw
            .checked_mul(rhs.raw)
            .expect("FixedPoint multiplication overflow");
        Self { raw: prod / DENOM }
    }
}

impl<const DENOM: i64> Div for FixedPoint<DENOM> {
    type Output = Self;
    /// Divides: `(a/D) / (b/D) = (a * D) / b`.
    ///
    /// # Panics
    ///
    /// Panics if `rhs` is zero.
    fn div(self, rhs: Self) -> Self {
        assert!(!rhs.is_zero(), "FixedPoint division by zero");
        let scaled = self
            .raw
            .checked_mul(DENOM)
            .expect("FixedPoint division overflow");
        Self {
            raw: scaled / rhs.raw,
        }
    }
}

// ── Display / Debug ───────────────────────────────────────────────────────────

impl<const DENOM: i64> fmt::Display for FixedPoint<DENOM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Count decimal digits in DENOM (e.g. 1_000 → 3 digits)
        let digits = (DENOM as f64).log10().round() as usize;
        let abs_raw = self.raw.unsigned_abs();
        let sign = if self.raw < 0 { "-" } else { "" };
        let denom_u = DENOM as u64;
        write!(
            f,
            "{}{}.{:0>width$}",
            sign,
            abs_raw / denom_u,
            abs_raw % denom_u,
            width = digits
        )
    }
}

impl<const DENOM: i64> fmt::Debug for FixedPoint<DENOM> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "FixedPoint<{DENOM}>({self})")
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    type Fp = FixedPoint<1_000>;

    #[test]
    fn addition() {
        let a = Fp::from_raw(750);
        let b = Fp::from_raw(250);
        assert_eq!((a + b).raw(), 1_000);
    }

    #[test]
    fn subtraction() {
        let a = Fp::from_int(2);
        let b = Fp::from_raw(500);
        assert_eq!((a - b).raw(), 1_500);
    }

    #[test]
    fn multiplication() {
        // 2.0 * 3.0 = 6.0
        let a = Fp::from_int(2);
        let b = Fp::from_int(3);
        assert_eq!((a * b).raw(), 6_000);
    }

    #[test]
    fn multiplication_fractional() {
        // 0.5 * 0.5 = 0.25
        let a = Fp::from_raw(500);
        let b = Fp::from_raw(500);
        assert_eq!((a * b).raw(), 250);
    }

    #[test]
    fn division() {
        // 1.0 / 4.0 = 0.25
        let a = Fp::from_int(1);
        let b = Fp::from_int(4);
        assert_eq!((a / b).raw(), 250);
    }

    #[test]
    fn negation() {
        let a = Fp::from_raw(500);
        assert_eq!((-a).raw(), -500);
    }

    #[test]
    fn ordering() {
        assert!(Fp::from_raw(500) < Fp::from_raw(750));
        assert!(Fp::from_int(1) > Fp::from_raw(999));
        assert_eq!(Fp::from_raw(500), Fp::from_raw(500));
    }

    #[test]
    fn display_integer() {
        assert_eq!(Fp::from_int(3).to_string(), "3.000");
    }

    #[test]
    fn display_fractional() {
        assert_eq!(Fp::from_raw(1_500).to_string(), "1.500");
    }

    #[test]
    fn display_negative() {
        assert_eq!(Fp::from_raw(-750).to_string(), "-0.750");
    }

    #[test]
    fn deterministic_across_operations() {
        // Prove that the same sequence of operations always gives the same raw bits
        let a = Fp::from_raw(314_159);
        let b = Fp::from_raw(271_828);
        let result = (a + b) * Fp::from_int(2) - Fp::from_raw(100_000);
        assert_eq!(result.raw(), 1_071_974);
    }
}
